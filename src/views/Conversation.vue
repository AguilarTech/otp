<script setup>
import { ref, onMounted, onBeforeUnmount, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

const props = defineProps({ pairing: { type: Object, required: true } })
const emit = defineEmits(['back', 'pairing-changed'])

const local = ref({ ...props.pairing })
const oauthStatus = ref({
	client_configured: false,
	picker_configured: false,
	connected: false,
})

const driveBound = computed(() => !!local.value.drive_folder_id)
const autoUploadActive = computed(
	() => driveBound.value && oauthStatus.value.connected,
)

const draft = ref('')
const outFrame = ref('')
const outUploaded = ref(null)
const inFrame = ref('')
const inbox = ref([])
const error = ref('')
const info = ref('')
const busy = ref(false)

const peerEmail = ref('')
const folderToBind = ref('')
const driveBusy = ref(false)

let unlistenInbox = null

async function refreshLocal() {
	try {
		const list = await invoke('list_pairings')
		const fresh = list.find((p) => p.id === local.value.id)
		if (fresh) local.value = fresh
		oauthStatus.value = await invoke('oauth_status')
	} catch (e) {
		error.value = String(e)
	}
}

onMounted(async () => {
	await refreshLocal()
	unlistenInbox = await listen('inbox-message', (event) => {
		const m = event.payload
		if (m.pairing_id !== local.value.id) return
		inbox.value.push({
			id: Math.random().toString(36).slice(2),
			seq: m.seq,
			ts: m.timestamp_ms,
			text: m.plaintext,
			source: 'drive',
		})
		void refreshLocal()
	})
})

onBeforeUnmount(() => {
	if (unlistenInbox) unlistenInbox()
})

async function send() {
	error.value = ''
	info.value = ''
	if (!draft.value.length) {
		error.value = 'Type something to send.'
		return
	}
	busy.value = true
	try {
		const result = await invoke('send_message', {
			pairingId: local.value.id,
			plaintext: draft.value,
		})
		outFrame.value = result.frame
		outUploaded.value = result.uploaded_file_id
		draft.value = ''
		if (result.uploaded_file_id) {
			info.value = `Sent through Google Drive (id ${result.uploaded_file_id.slice(0, 8)}…). Your friend's app will pick it up within ~30 seconds.`
		}
		await refreshLocal()
	} catch (e) {
		error.value = String(e)
	} finally {
		busy.value = false
	}
}

async function copyOut() {
	try {
		await navigator.clipboard.writeText(outFrame.value)
		info.value = 'Copied. Paste it into email, Signal, or anywhere else.'
	} catch (e) {
		error.value = `Copy failed: ${e}`
	}
}

function clearOut() {
	outFrame.value = ''
	outUploaded.value = null
}

async function receive() {
	error.value = ''
	if (!inFrame.value.trim()) {
		error.value = 'Paste an encrypted message to decode.'
		return
	}
	busy.value = true
	try {
		const msg = await invoke('decrypt_message', {
			frameB64: inFrame.value.trim(),
		})
		inbox.value.push({
			id: Math.random().toString(36).slice(2),
			seq: msg.seq,
			ts: msg.timestamp_ms,
			text: msg.plaintext,
			source: 'manual',
		})
		inFrame.value = ''
		await refreshLocal()
	} catch (e) {
		error.value = String(e)
	} finally {
		busy.value = false
	}
}

function dismiss(id) {
	const idx = inbox.value.findIndex((m) => m.id === id)
	if (idx >= 0) inbox.value.splice(idx, 1)
}

function formatTs(ms) {
	return new Date(Number(ms)).toLocaleString()
}

async function createDriveFolder() {
	error.value = ''
	info.value = ''
	driveBusy.value = true
	try {
		await invoke('drive_create_folder', {
			pairingId: local.value.id,
			peerEmail: peerEmail.value.trim(),
		})
		info.value = peerEmail.value.trim()
			? `Drive folder ready and shared with ${peerEmail.value.trim()}. Google has sent them an email invite.`
			: 'Drive folder ready. Your friend will need to link it on their side too.'
		peerEmail.value = ''
		await refreshLocal()
		emit('pairing-changed')
	} catch (e) {
		error.value = String(e)
	} finally {
		driveBusy.value = false
	}
}

async function bindDriveFolder() {
	error.value = ''
	info.value = ''
	if (!folderToBind.value.trim()) {
		error.value = 'Paste the Drive folder link or ID first.'
		return
	}
	const folderId = extractFolderId(folderToBind.value.trim())
	driveBusy.value = true
	try {
		await invoke('drive_bind_folder', {
			pairingId: local.value.id,
			folderId,
		})
		info.value =
			'Linked. New messages will arrive automatically every ~30 seconds.'
		folderToBind.value = ''
		await refreshLocal()
		emit('pairing-changed')
	} catch (e) {
		error.value = String(e)
	} finally {
		driveBusy.value = false
	}
}

async function pickDriveFolder() {
	error.value = ''
	info.value =
		'Your browser will open with the Google folder picker. Choose the folder your friend shared with you and come back here.'
	driveBusy.value = true
	try {
		await invoke('drive_pick_folder', { pairingId: local.value.id })
		info.value = 'Linked. New messages will arrive automatically every ~30 seconds.'
		await refreshLocal()
		emit('pairing-changed')
	} catch (e) {
		error.value = String(e)
		info.value = ''
	} finally {
		driveBusy.value = false
	}
}

async function unbindDriveFolder() {
	error.value = ''
	info.value = ''
	driveBusy.value = true
	try {
		await invoke('drive_unbind_folder', { pairingId: local.value.id })
		info.value = 'Unlinked. The app will stop checking Drive for this friend.'
		await refreshLocal()
		emit('pairing-changed')
	} catch (e) {
		error.value = String(e)
	} finally {
		driveBusy.value = false
	}
}

function extractFolderId(input) {
	const m = input.match(/folders\/([a-zA-Z0-9_-]+)/)
	return m ? m[1] : input
}
</script>

<template>
	<div>
		<button class="btn btn-ghost back" type="button" @click="emit('back')">
			<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
				<path d="M19 12H5" /><path d="M11 18l-6-6 6-6" />
			</svg>
			Back
		</button>

		<div class="eyebrow">Friend</div>
		<h1 class="h1">{{ local.name }}</h1>

		<div class="card">
			<div class="card-head">
				<div>
					<div class="h2" style="margin-bottom: 4px">Internet delivery</div>
					<p class="small" style="margin: 0">
						Send and receive automatically through a private Google
						Drive folder you and {{ local.name }} both have access to.
					</p>
				</div>
				<span
					class="pill"
					:class="autoUploadActive ? 'pill-success' : 'pill-neutral'"
				>
					<span class="dot" :class="{ pulse: autoUploadActive }"></span>
					{{
						autoUploadActive
							? 'Live'
							: driveBound
								? 'Drive offline'
								: 'Not set up'
					}}
				</span>
			</div>

			<div v-if="!oauthStatus.client_configured" class="banner banner-warn">
				Google Drive isn't set up in this build. Open
				<strong>Settings</strong> for details.
			</div>
			<div
				v-else-if="!oauthStatus.connected"
				class="banner banner-info"
			>
				Connect Google Drive in <strong>Settings</strong> to turn on
				auto-delivery for this friend.
			</div>

			<div v-if="driveBound" class="bound-row">
				<div>
					<div class="small" style="margin-bottom: 4px">Drive folder</div>
					<code class="mono">{{ local.drive_folder_id }}</code>
				</div>
				<button
					class="btn btn-danger"
					type="button"
					@click="unbindDriveFolder"
					:disabled="driveBusy"
				>
					Unlink
				</button>
			</div>

			<div v-else-if="oauthStatus.connected" class="drive-options">
				<div class="drive-sub">
					<label>Create a fresh folder in your Drive and invite them</label>
					<div class="combo">
						<input
							v-model="peerEmail"
							:placeholder="`${local.name}'s Google email (optional)`"
							:disabled="driveBusy"
						/>
						<button
							class="btn btn-primary"
							type="button"
							@click="createDriveFolder"
							:disabled="driveBusy"
						>
							Create folder
						</button>
					</div>
					<div class="field-hint">
						If you fill in their email, Google sends them an invite.
						Otherwise you'll need to share the folder yourself.
					</div>
				</div>

				<div class="drive-sub">
					<label>Or pick a folder they shared with you</label>
					<div class="row">
						<button
							class="btn btn-primary"
							type="button"
							@click="pickDriveFolder"
							:disabled="driveBusy || !oauthStatus.picker_configured"
						>
							{{ driveBusy ? 'Waiting for browser…' : 'Choose folder from Drive' }}
							<svg
								v-if="!driveBusy"
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
					</div>
					<div v-if="!oauthStatus.picker_configured" class="field-hint">
						The Drive picker isn't set up in this build. See
						<code>CLOUD_SETUP.md</code>.
					</div>
					<div v-else class="field-hint">
						Opens Google Drive in your browser. Find the folder
						{{ local.name }} shared with you (under
						<em>Shared with me</em>) and select it.
					</div>
				</div>

				<div class="drive-sub">
					<label>Or paste a folder link directly</label>
					<div class="combo">
						<input
							v-model="folderToBind"
							placeholder="https://drive.google.com/drive/folders/…"
							:disabled="driveBusy"
						/>
						<button
							class="btn btn-ghost"
							type="button"
							@click="bindDriveFolder"
							:disabled="driveBusy"
						>
							Link
						</button>
					</div>
					<div class="field-hint">
						Only works if you and your friend share a Google account.
						Otherwise use <em>Choose folder from Drive</em> above.
					</div>
				</div>
			</div>
		</div>

		<div class="card">
			<div class="eyebrow">Send a message</div>
			<textarea
				v-model="draft"
				:placeholder="`Type a message to ${local.name}…`"
				rows="3"
				:disabled="busy"
			/>
			<div class="row" style="margin-top: 12px">
				<button
					class="btn btn-primary"
					type="button"
					@click="send"
					:disabled="busy"
				>
					{{ autoUploadActive ? 'Encrypt &amp; send' : 'Encrypt' }}
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<path d="M5 12h14" /><path d="M13 6l6 6-6 6" />
					</svg>
				</button>
			</div>

			<div v-if="outFrame" class="frame-out">
				<div class="eyebrow" style="margin-top: 18px">
					{{ outUploaded ? 'Backup copy' : 'Encrypted message' }}
				</div>
				<p class="small" v-if="!outUploaded" style="margin-bottom: 8px">
					Copy this text and paste it into email, Signal, or anywhere
					else. Your friend's app will decode it.
				</p>
				<p class="small" v-else style="margin-bottom: 8px">
					Already sent over Drive. You can also copy this version if
					you want to send it through another channel as a backup.
				</p>
				<textarea :value="outFrame" readonly rows="3" />
				<div class="row" style="margin-top: 8px">
					<button class="btn btn-ghost" type="button" @click="copyOut">
						Copy
					</button>
					<button class="btn btn-ghost" type="button" @click="clearOut">
						Clear
					</button>
				</div>
			</div>
		</div>

		<div class="card">
			<div class="eyebrow">Receive a message by hand</div>
			<p class="small" style="margin-bottom: 10px">
				If your friend sent you an encrypted message outside Google
				Drive (email, Signal, paper, etc.), paste the text here to
				decode it.
			</p>
			<textarea
				v-model="inFrame"
				placeholder="Paste an encrypted message…"
				rows="3"
				:disabled="busy"
			/>
			<div class="row" style="margin-top: 12px">
				<button
					class="btn btn-primary"
					type="button"
					@click="receive"
					:disabled="busy"
				>
					Decode
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<path d="M5 12h14" /><path d="M13 6l6 6-6 6" />
					</svg>
				</button>
			</div>
		</div>

		<div v-if="inbox.length" class="card">
			<div class="eyebrow">Inbox · gone when you close the app</div>
			<div v-for="m in inbox" :key="m.id" class="msg">
				<div class="msg-head">
					<span class="msg-ts">{{ formatTs(m.ts) }}</span>
					<span class="pill" :class="m.source === 'drive' ? 'pill-accent' : 'pill-neutral'">
						{{ m.source === 'drive' ? 'via Drive' : 'pasted in' }}
					</span>
					<button
						class="btn btn-ghost btn-dismiss"
						type="button"
						@click="dismiss(m.id)"
					>
						Dismiss
					</button>
				</div>
				<pre>{{ m.text }}</pre>
			</div>
		</div>

		<div v-if="info" class="banner banner-success">{{ info }}</div>
		<div v-if="error" class="banner banner-error">{{ error }}</div>
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

	.bound-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		background: var(--panel-2);
		border: 1px solid var(--line);
		border-radius: var(--r-panel);
		padding: 12px 14px;
	}

	.bound-row code {
		word-break: break-all;
	}

	.drive-options {
		display: flex;
		flex-direction: column;
		gap: 18px;
	}

	.drive-sub {
		display: flex;
		flex-direction: column;
	}

	.combo {
		display: flex;
		gap: 8px;
	}

	.combo input {
		flex: 1;
	}

	.frame-out {
		margin-top: 4px;
	}

	.msg {
		background: var(--panel-2);
		border: 1px solid var(--line);
		border-radius: var(--r-panel);
		padding: 14px 16px;
		margin-bottom: 10px;
	}

	.msg:last-child {
		margin-bottom: 0;
	}

	.msg-head {
		display: flex;
		gap: 10px;
		align-items: center;
		margin-bottom: 8px;
	}

	.msg-ts {
		font-size: 12px;
		color: var(--fg-3);
	}

	.btn-dismiss {
		margin-left: auto;
		padding: 6px 12px;
		font-size: 12px;
	}

	.msg pre {
		margin: 0;
		font-family: var(--sans);
		font-size: 14px;
		color: var(--fg);
		white-space: pre-wrap;
		word-break: break-word;
		line-height: 1.55;
	}
</style>
