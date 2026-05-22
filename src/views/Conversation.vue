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
			info.value = `Uploaded to Drive — file id ${result.uploaded_file_id.slice(0, 8)}…`
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
		info.value = 'Frame copied to clipboard.'
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
		error.value = 'Paste a frame to decode.'
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
			? `Folder created and shared with ${peerEmail.value.trim()} (Drive sent an invite).`
			: 'Folder created. Bind it on the peer side manually.'
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
		error.value = 'Paste a Drive folder ID or URL.'
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
			'Folder bound. Background polling will pick up new messages every ~30s.'
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
		'A browser tab opened with Google Picker. Pick the folder a peer shared with you and return here.'
	driveBusy.value = true
	try {
		await invoke('drive_pick_folder', { pairingId: local.value.id })
		info.value = 'Folder picked and bound. Background polling is active.'
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
		info.value = 'Folder unbound from this pairing.'
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

		<div class="eyebrow">Pairing</div>
		<h1 class="h1">{{ local.name }}</h1>

		<div class="card">
			<div class="card-head">
				<div>
					<div class="h2" style="margin-bottom: 4px">Drive mailbox</div>
					<p class="small" style="margin: 0">
						Each pairing gets its own Drive folder. Both peers upload
						ciphertext blobs; the poller verifies and pulls them down.
					</p>
				</div>
				<span
					class="pill"
					:class="autoUploadActive ? 'pill-success' : driveBound ? 'pill-neutral' : 'pill-neutral'"
				>
					<span class="dot" :class="{ pulse: autoUploadActive }"></span>
					{{
						autoUploadActive
							? 'Auto-upload + poll'
							: driveBound
								? 'Bound, drive offline'
								: 'Not bound'
					}}
				</span>
			</div>

			<div v-if="!oauthStatus.client_configured" class="banner banner-warn">
				OAuth client not configured. Settings → see
				<code>CLOUD_SETUP.md</code>.
			</div>
			<div
				v-else-if="!oauthStatus.connected"
				class="banner banner-info"
			>
				Connect Google Drive in Settings to enable auto-upload and
				polling.
			</div>

			<div v-if="driveBound" class="bound-row">
				<div>
					<div class="small" style="margin-bottom: 4px">Folder</div>
					<code class="mono">{{ local.drive_folder_id }}</code>
				</div>
				<button
					class="btn btn-danger"
					type="button"
					@click="unbindDriveFolder"
					:disabled="driveBusy"
				>
					Unbind
				</button>
			</div>

			<div v-else-if="oauthStatus.connected" class="drive-options">
				<div class="drive-sub">
					<label>Create a new mailbox folder in your Drive</label>
					<div class="combo">
						<input
							v-model="peerEmail"
							placeholder="Peer's Google email (optional — sends Drive invite)"
							:disabled="driveBusy"
						/>
						<button
							class="btn btn-primary"
							type="button"
							@click="createDriveFolder"
							:disabled="driveBusy"
						>
							Create
						</button>
					</div>
				</div>

				<div class="drive-sub">
					<label>Claim a folder a peer shared with you (cross-account)</label>
					<div class="row">
						<button
							class="btn btn-primary"
							type="button"
							@click="pickDriveFolder"
							:disabled="driveBusy || !oauthStatus.picker_configured"
						>
							{{ driveBusy ? 'Waiting for picker…' : 'Pick from Google Drive' }}
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
						Picker API key not configured. Set
						<code>OTP_GOOGLE_API_KEY</code> and rebuild — see
						<code>CLOUD_SETUP.md</code>.
					</div>
					<div v-else class="field-hint">
						Opens Google Picker in your browser. Select the shared folder
						under "Shared with me". This is the only way to bind a folder
						owned by another account under
						<code>drive.file</code> scope.
					</div>
				</div>

				<div class="drive-sub">
					<label>Or paste a folder ID directly (same account only)</label>
					<div class="combo">
						<input
							v-model="folderToBind"
							placeholder="Drive folder URL or ID"
							:disabled="driveBusy"
						/>
						<button
							class="btn btn-ghost"
							type="button"
							@click="bindDriveFolder"
							:disabled="driveBusy"
						>
							Bind
						</button>
					</div>
				</div>
			</div>
		</div>

		<div class="card">
			<div class="eyebrow">Compose</div>
			<textarea
				v-model="draft"
				placeholder="Type a message…"
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
					{{ autoUploadActive ? 'Encrypt &amp; upload' : 'Encrypt' }}
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<path d="M5 12h14" /><path d="M13 6l6 6-6 6" />
					</svg>
				</button>
			</div>

			<div v-if="outFrame" class="frame-out">
				<div class="eyebrow" style="margin-top: 18px">
					{{ outUploaded ? 'Manual fallback' : 'Frame to send' }}
				</div>
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
			<div class="eyebrow">Receive (manual)</div>
			<textarea
				v-model="inFrame"
				placeholder="Paste a frame to decode…"
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
			<div class="eyebrow">Inbox · session only</div>
			<div v-for="m in inbox" :key="m.id" class="msg">
				<div class="msg-head">
					<span class="msg-seq">#{{ m.seq }}</span>
					<span class="msg-ts">{{ formatTs(m.ts) }}</span>
					<span class="pill" :class="m.source === 'drive' ? 'pill-accent' : 'pill-neutral'">
						{{ m.source }}
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
		gap: 16px;
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

	.msg-seq {
		font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
		font-size: 11px;
		color: var(--fg-3);
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
