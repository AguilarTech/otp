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
			info.value = `Uploaded to Drive (file id ${result.uploaded_file_id.slice(0, 8)}…).`
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
		const folderId = await invoke('drive_create_folder', {
			pairingId: local.value.id,
			peerEmail: peerEmail.value.trim(),
		})
		info.value = peerEmail.value.trim()
			? `Folder created and shared with ${peerEmail.value.trim()} (Drive sent them an invite).`
			: 'Folder created (not shared — bind it on the peer side manually).'
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
		info.value = 'Folder bound. Background polling will pick up new messages every ~30s.'
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
	info.value = 'A browser tab opened with Google Picker. Pick the folder a peer shared with you, then return here.'
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
	// Accept either a raw id or a Drive URL like
	// https://drive.google.com/drive/folders/<id>?usp=sharing
	const m = input.match(/folders\/([a-zA-Z0-9_-]+)/)
	return m ? m[1] : input
}
</script>

<template>
	<div>
		<header>
			<button @click="emit('back')" class="ghost">← Back</button>
			<h1>{{ local.name }}</h1>
		</header>

		<section class="drive">
			<h2>Drive mailbox</h2>
			<div v-if="!oauthStatus.client_configured" class="hint">
				OAuth not configured — Settings → see CLOUD_SETUP.md.
			</div>
			<div v-else-if="!oauthStatus.connected" class="hint">
				Connect Google Drive in Settings to enable auto-upload and polling.
			</div>
			<div v-else-if="driveBound">
				<div class="bound">
					<span :class="['dot', autoUploadActive ? 'on' : 'off']"></span>
					Bound to folder
					<code>{{ local.drive_folder_id.slice(0, 16) }}…</code>
					— auto-upload + poll active.
				</div>
				<div class="row">
					<button class="ghost" @click="unbindDriveFolder" :disabled="driveBusy">
						Unbind
					</button>
				</div>
			</div>
			<div v-else>
				<div class="sub">
					<label>Create a new mailbox folder in your Drive</label>
					<div class="row">
						<input
							v-model="peerEmail"
							placeholder="Peer's Google email (optional — sends Drive invite)"
							:disabled="driveBusy"
						/>
						<button @click="createDriveFolder" :disabled="driveBusy">
							Create
						</button>
					</div>
				</div>
				<div class="sub">
					<label>Claim a folder a peer shared with you (cross-account)</label>
					<div class="row">
						<button
							@click="pickDriveFolder"
							:disabled="driveBusy || !oauthStatus.picker_configured"
						>
							{{ driveBusy ? 'Waiting for picker…' : 'Pick from Google Drive' }}
						</button>
					</div>
					<div v-if="!oauthStatus.picker_configured" class="hint warn">
						Picker API key not configured at build time. See
						<code>CLOUD_SETUP.md</code> and rebuild with
						<code>OTP_GOOGLE_API_KEY</code> set.
					</div>
					<div v-else class="hint">
						Opens Google Picker in your browser. Select the shared folder
						under "Shared with me". This is the only way to bind a folder
						owned by a different account under <code>drive.file</code> scope.
					</div>
				</div>
				<div class="sub">
					<label>Or paste a folder ID directly (same account only)</label>
					<div class="row">
						<input
							v-model="folderToBind"
							placeholder="Drive folder URL or ID"
							:disabled="driveBusy"
						/>
						<button @click="bindDriveFolder" :disabled="driveBusy">
							Bind
						</button>
					</div>
					<div class="hint">
						Verifies the app can already see the folder under
						<code>drive.file</code> (it can if your app created it). If
						the folder is owned by another account use Pick above.
					</div>
				</div>
			</div>
		</section>

		<section>
			<h2>Compose</h2>
			<textarea
				v-model="draft"
				placeholder="Type a message…"
				rows="3"
				:disabled="busy"
			/>
			<div class="row">
				<button @click="send" :disabled="busy">
					{{ autoUploadActive ? 'Encrypt &amp; upload' : 'Encrypt' }}
				</button>
			</div>
			<div v-if="outFrame" class="frame">
				<label v-if="outUploaded">
					Uploaded to Drive. Manual fallback frame:
				</label>
				<label v-else>Frame to send (paste into your transport):</label>
				<textarea :value="outFrame" readonly rows="3" />
				<div class="row">
					<button @click="copyOut">Copy</button>
					<button @click="clearOut" class="ghost">Clear</button>
				</div>
			</div>
		</section>

		<section>
			<h2>Receive (manual)</h2>
			<textarea
				v-model="inFrame"
				placeholder="Paste a frame to decode…"
				rows="3"
				:disabled="busy"
			/>
			<div class="row">
				<button @click="receive" :disabled="busy">Decode</button>
			</div>
		</section>

		<section v-if="inbox.length">
			<h2>Inbox (session only)</h2>
			<div v-for="m in inbox" :key="m.id" class="msg">
				<div class="msg-head">
					<span class="seq">#{{ m.seq }}</span>
					<span class="ts">{{ formatTs(m.ts) }}</span>
					<span class="src">{{ m.source }}</span>
					<button class="ghost" @click="dismiss(m.id)">Dismiss</button>
				</div>
				<pre>{{ m.text }}</pre>
			</div>
		</section>

		<div v-if="info" class="info">{{ info }}</div>
		<div v-if="error" class="error">{{ error }}</div>
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
	textarea {
		width: 100%;
		box-sizing: border-box;
		font-family: inherit;
		resize: vertical;
	}
	.row {
		display: flex;
		gap: 8px;
		margin-top: 8px;
	}
	.row input {
		flex: 1;
	}
	.frame {
		margin-top: 12px;
	}
	.frame label {
		font-size: 0.8em;
		color: #888;
	}
	.drive .sub {
		margin-bottom: 12px;
	}
	.drive .sub label {
		font-size: 0.8em;
		color: #888;
		display: block;
		margin-bottom: 4px;
	}
	.bound {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 0.9em;
		color: #ccc;
	}
	.bound code {
		background: #1f1f1f;
		padding: 1px 5px;
		border-radius: 3px;
		font-size: 0.85em;
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
	.hint {
		font-size: 0.85em;
		color: #888;
		margin-top: 4px;
	}
	.hint.warn {
		background: #2a2410;
		color: #d0c070;
		padding: 8px;
		border-radius: 4px;
		margin-top: 8px;
		line-height: 1.5;
	}
	.msg {
		background: #2a2a2a;
		padding: 12px;
		border-radius: 6px;
		margin-bottom: 8px;
	}
	.msg-head {
		display: flex;
		gap: 12px;
		align-items: center;
		font-size: 0.8em;
		color: #999;
		margin-bottom: 4px;
	}
	.src {
		text-transform: uppercase;
		font-size: 0.7em;
		color: #888;
		background: #1f1f1f;
		padding: 1px 6px;
		border-radius: 3px;
		letter-spacing: 0.05em;
	}
	.msg-head button {
		margin-left: auto;
		font-size: 0.85em;
		padding: 2px 8px;
	}
	.msg pre {
		margin: 0;
		white-space: pre-wrap;
		word-break: break-word;
		font-family: inherit;
	}
	.info {
		color: #8acc8a;
		padding: 8px;
		background: #1a2a1a;
		border-radius: 6px;
		margin-top: 12px;
	}
	.error {
		color: #ff7070;
		padding: 8px;
		background: #2a1a1a;
		border-radius: 6px;
		margin-top: 12px;
	}
	.seq {
		font-family: monospace;
	}
</style>
