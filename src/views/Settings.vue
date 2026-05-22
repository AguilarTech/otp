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
		'Your browser will open with the Google sign-in page. Approve access to "Files created by this app" and come back here.'
	busy.value = true
	try {
		await invoke('oauth_connect')
		info.value = 'Signed in to Google Drive.'
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
		info.value = 'Signed out. Your sign-in is removed from this device.'
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
		<div class="back-row">
			<button
				class="btn btn-ghost"
				type="button"
				@click="emit('back')"
				:disabled="busy"
			>
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
					<path d="M19 12H5" /><path d="M11 18l-6-6 6-6" />
				</svg>
				Back
			</button>
		</div>

		<div class="eyebrow">Settings</div>
		<h1 class="h1">Internet delivery.</h1>
		<p class="lead">
			Drive delivery is <strong>optional</strong>. The app works fine
			without it — every message can also be saved as a small file or
			copy-pasted into any channel you already use (email, Signal, even
			paper).
		</p>

		<div class="card">
			<div class="card-head">
				<div>
					<div class="h2" style="margin-bottom: 4px">Google Drive</div>
					<p class="small" style="margin: 0">
						When signed in, the app can auto-send and auto-receive
						messages through private folders in your Drive. Google only
						sees the encrypted text — never your messages.
					</p>
				</div>
				<span
					class="pill"
					:class="status.connected ? 'pill-success' : 'pill-neutral'"
				>
					<span class="dot" :class="{ pulse: status.connected }"></span>
					{{ status.connected ? 'Signed in' : 'Not signed in' }}
				</span>
			</div>

			<div v-if="!status.client_configured" class="banner banner-warn">
				<div>
					<strong>Drive isn't configured in this build.</strong>
					Whoever compiled the app needs to follow
					<code>CLOUD_SETUP.md</code> and rebuild it with the right
					Google credentials. The app still works without Drive — you
					can save or copy-paste encrypted messages by hand.
				</div>
			</div>

			<div
				v-else-if="!status.picker_configured"
				class="banner banner-warn"
			>
				<div>
					Sign-in works, but choosing folders from other Google
					accounts won't. You can still create new folders in your own
					Drive. Set <code>OTP_GOOGLE_API_KEY</code> and rebuild — see
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
					{{ busy ? 'Waiting for browser…' : 'Sign in to Google Drive' }}
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
					Sign out
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
			<div class="h2">Using OTP Messenger without Google</div>
			<p>
				You don't need a Google account to use the app. After you set
				up a friend on USB, every message can be saved as a small
				<code>.otp</code> file (or copied as text) and shared through
				whatever you prefer — email, Signal, Telegram, AirDrop, a USB
				stick, or even printed paper. Your friend pastes or opens it,
				their app decodes it. Same security guarantees either way.
			</p>
			<p style="margin-bottom: 0">
				Google Drive is just there to make it automatic for the people
				who want it. If you don't sign in, just leave it switched off.
			</p>
		</div>
	</div>
</template>

<style scoped>
	.card-head {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 16px;
		margin-bottom: 16px;
	}
</style>
