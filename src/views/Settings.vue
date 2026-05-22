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
	info.value =
		'A browser tab opened to Google. Approve drive.file access and return here.'
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
		<button
			class="btn btn-ghost back"
			type="button"
			@click="emit('back')"
			:disabled="busy"
		>
			<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
				<path d="M19 12H5" /><path d="M11 18l-6-6 6-6" />
			</svg>
			Back
		</button>

		<div class="eyebrow">Settings</div>
		<h1 class="h1">Transport &amp; account.</h1>

		<div class="card">
			<div class="card-head">
				<div>
					<div class="h2" style="margin-bottom: 4px">Google Drive</div>
					<p class="small" style="margin: 0">
						OAuth2 with the narrow <code>drive.file</code> scope. Refresh
						token lives in the OS keychain.
					</p>
				</div>
				<span
					class="pill"
					:class="status.connected ? 'pill-success' : 'pill-neutral'"
				>
					<span class="dot" :class="{ pulse: status.connected }"></span>
					{{ status.connected ? 'Connected' : 'Not connected' }}
				</span>
			</div>

			<div v-if="!status.client_configured" class="banner banner-warn">
				<div>
					<strong>OAuth client credentials not embedded.</strong>
					Rebuild with <code>OTP_GOOGLE_CLIENT_ID</code> and
					<code>OTP_GOOGLE_CLIENT_SECRET</code> set in the environment.
					See <code>CLOUD_SETUP.md</code>.
				</div>
			</div>

			<div
				v-else-if="!status.picker_configured"
				class="banner banner-warn"
			>
				<div>
					OAuth is configured but the Google Picker API key isn't.
					Cross-account folder claiming will stay disabled. Set
					<code>OTP_GOOGLE_API_KEY</code> and rebuild — see
					<code>CLOUD_SETUP.md</code>.
				</div>
			</div>

			<div v-if="status.client_configured" class="row" style="margin-top: 16px">
				<button
					v-if="!status.connected"
					class="btn btn-primary"
					type="button"
					@click="connect"
					:disabled="busy"
				>
					{{ busy ? 'Waiting for browser…' : 'Connect Google Drive' }}
					<svg
						v-if="!busy"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="1.8"
						stroke-linecap="round"
						stroke-linejoin="round"
						aria-hidden="true"
					>
						<path d="M5 12h14" /><path d="M13 6l6 6-6 6" />
					</svg>
				</button>
				<button
					v-else
					class="btn btn-ghost"
					type="button"
					@click="disconnect"
					:disabled="busy"
				>
					Disconnect
				</button>
			</div>

			<div v-if="info" class="banner banner-success" style="margin-top: 16px">
				{{ info }}
			</div>
			<div v-if="error" class="banner banner-error" style="margin-top: 16px">
				{{ error }}
			</div>
		</div>

		<div class="card">
			<div class="h2">About</div>
			<p>
				Pad material lives in this device's app data directory. Plaintext
				is dropped from memory when you dismiss messages — no persistent
				history is kept. Google Drive sees ciphertext blob sizes and
				timing; it never sees plaintext or pad bytes.
			</p>
		</div>
	</div>
</template>

<style scoped>
	.back {
		margin-bottom: 18px;
	}

	.card-head {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 16px;
		margin-bottom: 16px;
	}
</style>
