<script setup>
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const emit = defineEmits(['back'])

const status = ref({
	client_configured: false,
	picker_configured: false,
	connected: false,
})
const busy = ref(false)
const error = ref('')
const info = ref('')

async function refresh() {
	try {
		status.value = await invoke('oauth_status')
	} catch (e) {
		error.value = String(e)
	}
}

onMounted(refresh)

async function connect() {
	error.value = ''
	info.value = 'A browser tab opened to Google. Approve drive.file access and return here.'
	busy.value = true
	try {
		await invoke('oauth_connect')
		info.value = 'Connected to Google Drive.'
	} catch (e) {
		error.value = String(e)
		info.value = ''
	} finally {
		busy.value = false
		await refresh()
	}
}

async function disconnect() {
	error.value = ''
	busy.value = true
	try {
		await invoke('oauth_disconnect')
		info.value = 'Disconnected. Refresh token cleared from the OS keychain.'
	} catch (e) {
		error.value = String(e)
	} finally {
		busy.value = false
		await refresh()
	}
}
</script>

<template>
	<div>
		<header>
			<button @click="emit('back')" class="ghost" :disabled="busy">← Back</button>
			<h1>Settings</h1>
		</header>

		<section>
			<h2>Google Drive</h2>

			<div v-if="!status.client_configured" class="warn">
				OAuth client credentials were not embedded at build time. See
				<code>CLOUD_SETUP.md</code> for the Google Cloud console steps,
				then rebuild with <code>OTP_GOOGLE_CLIENT_ID</code> and
				<code>OTP_GOOGLE_CLIENT_SECRET</code> set in the environment.
			</div>

			<div v-else-if="!status.picker_configured" class="warn">
				OAuth is configured but the Google Picker API key isn't.
				Cross-account folder claiming will be disabled — both peers must
				use the same Google identity, or use the manual paste flow.
				Set <code>OTP_GOOGLE_API_KEY</code> and rebuild (see
				<code>CLOUD_SETUP.md</code>).
			</div>

			<div v-if="status.client_configured">
				<div class="status">
					<span :class="['dot', status.connected ? 'on' : 'off']"></span>
					{{ status.connected ? 'Connected' : 'Not connected' }}
				</div>
				<div class="actions">
					<button
						v-if="!status.connected"
						@click="connect"
						:disabled="busy"
					>
						{{ busy ? 'Waiting for browser…' : 'Connect Google Drive' }}
					</button>
					<button
						v-else
						@click="disconnect"
						class="ghost"
						:disabled="busy"
					>
						Disconnect
					</button>
				</div>
			</div>
		</section>

		<div v-if="info" class="info">{{ info }}</div>
		<div v-if="error" class="error">{{ error }}</div>

		<section>
			<h2>About</h2>
			<p class="help">
				Pad material lives in this device's app data dir. Plaintext is
				dropped from memory when you dismiss messages — no persistent
				history is kept. Drive sees ciphertext blob sizes and timing;
				it never sees plaintext or pad bytes.
			</p>
		</section>
	</div>
</template>

<style scoped>
	header {
		display: flex;
		align-items: center;
		gap: 16px;
		margin-bottom: 16px;
	}
	header h1 {
		margin: 0;
	}
	.ghost {
		background: transparent;
	}
	section {
		margin-bottom: 24px;
		text-align: left;
	}
	h2 {
		font-size: 1em;
		color: #aaa;
		margin: 0 0 8px 0;
	}
	.status {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 0.9em;
		color: #ccc;
		margin-bottom: 8px;
	}
	.dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		display: inline-block;
	}
	.dot.on {
		background: #4caf50;
	}
	.dot.off {
		background: #666;
	}
	.actions {
		display: flex;
		gap: 8px;
	}
	.warn {
		background: #2a2410;
		color: #d0c070;
		padding: 12px;
		border-radius: 6px;
		font-size: 0.9em;
		line-height: 1.5;
	}
	.warn code {
		background: #1f1f1f;
		padding: 1px 5px;
		border-radius: 3px;
	}
	.info {
		color: #8acc8a;
		padding: 8px;
		background: #1a2a1a;
		border-radius: 6px;
		margin-bottom: 12px;
	}
	.error {
		color: #ff7070;
		padding: 8px;
		background: #2a1a1a;
		border-radius: 6px;
		margin-bottom: 12px;
	}
	.help {
		color: #888;
		font-size: 0.85em;
		line-height: 1.5;
	}
</style>
