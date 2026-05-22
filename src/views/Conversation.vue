<script setup>
import { ref, onMounted, onBeforeUnmount, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open, save } from '@tauri-apps/plugin-dialog'

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
const messages = ref([]) // unified thread: { id, direction, kind, text?, file_*?, ts, source }
const outFrame = ref('')
const outFrameKind = ref('text')
const outUploaded = ref(null)
const inFrame = ref('')
const showPaste = ref(false)
const error = ref('')
const info = ref('')
const busy = ref(false)
const sendingFile = ref(false)

const peerEmail = ref('')
const folderToBind = ref('')
const driveBusy = ref(false)

const confirmingRemove = ref(false)
const removeBusy = ref(false)

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
		messages.value.push({
			id: Math.random().toString(36).slice(2),
			direction: 'received',
			source: 'drive',
			kind: m.kind,
			text: m.text,
			file_name: m.file_name,
			file_size: m.file_size,
			file_bytes_b64: m.file_bytes_b64,
			ts: m.timestamp_ms,
		})
		void refreshLocal()
	})
})

onBeforeUnmount(() => {
	if (unlistenInbox) unlistenInbox()
})

async function pasteIntoDraft() {
	try {
		const text = await navigator.clipboard.readText()
		draft.value += text
	} catch (e) {
		error.value = `Couldn't read the clipboard: ${e}`
	}
}

async function pasteIntoReceive() {
	try {
		const text = await navigator.clipboard.readText()
		inFrame.value = text.trim()
	} catch (e) {
		error.value = `Couldn't read the clipboard: ${e}`
	}
}

async function send() {
	error.value = ''
	info.value = ''
	if (!draft.value.length) {
		error.value = 'Type something to send.'
		return
	}
	busy.value = true
	try {
		const result = await invoke('send_text_message', {
			pairingId: local.value.id,
			plaintext: draft.value,
		})
		messages.value.push({
			id: Math.random().toString(36).slice(2),
			direction: 'sent',
			source: result.uploaded_file_id ? 'drive' : 'local',
			kind: 'text',
			text: draft.value,
			ts: Date.now(),
		})
		outFrame.value = result.frame
		outFrameKind.value = 'text'
		outUploaded.value = result.uploaded_file_id
		draft.value = ''
		if (result.uploaded_file_id) {
			info.value = `Sent through Google Drive. Your friend's app will pick it up within ~30 seconds.`
		}
		await refreshLocal()
	} catch (e) {
		error.value = String(e)
	} finally {
		busy.value = false
	}
}

async function sendFile() {
	error.value = ''
	info.value = ''
	try {
		const filePath = await open({
			multiple: false,
			title: 'Pick a file to encrypt and send',
		})
		if (!filePath) return
		sendingFile.value = true
		const result = await invoke('send_file_attachment', {
			pairingId: local.value.id,
			filePath,
		})
		const displayName = String(filePath).split(/[\\/]/).pop() || 'attachment'
		messages.value.push({
			id: Math.random().toString(36).slice(2),
			direction: 'sent',
			source: result.uploaded_file_id ? 'drive' : 'local',
			kind: 'file',
			file_name: displayName,
			file_size: Math.max(0, result.pad_consumed - 32 - 3 - displayName.length),
			ts: Date.now(),
		})
		outFrame.value = result.frame
		outFrameKind.value = 'file'
		outUploaded.value = result.uploaded_file_id
		if (result.uploaded_file_id) {
			info.value = 'File encrypted and sent through Google Drive.'
		} else {
			info.value = 'File encrypted. Save it as a .otp file or copy the text below to send it any way you like.'
		}
		await refreshLocal()
	} catch (e) {
		error.value = String(e)
	} finally {
		sendingFile.value = false
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

async function saveOutAsFile() {
	try {
		const defaultName = `otp-${local.value.name.replace(/[^a-z0-9-_]/gi, '_')}-${Date.now()}.otp`
		const path = await save({
			defaultPath: defaultName,
			filters: [{ name: 'OTP encrypted message', extensions: ['otp'] }],
		})
		if (!path) return
		await invoke('write_file_text', { path, contents: outFrame.value })
		info.value = `Saved. Send the .otp file to your friend any way you like — they can paste its contents into their app.`
	} catch (e) {
		error.value = `Save failed: ${e}`
	}
}

function clearOut() {
	outFrame.value = ''
	outUploaded.value = null
}

async function loadOtpFile() {
	try {
		const filePath = await open({
			multiple: false,
			title: 'Pick an .otp file your friend sent you',
			filters: [{ name: 'OTP encrypted message', extensions: ['otp', 'txt'] }],
		})
		if (!filePath) return
		// We read the file via fetch — Tauri exposes file: URLs via the
		// custom protocol; instead, read it via a quick fetch trick: use
		// invoke. But we don't have a Rust read-text-file command, so just
		// have the user paste. Actually, easier: keep this disabled and
		// rely on paste/clipboard for now.
		error.value = "Opening .otp files directly isn't wired up yet — please paste their contents into the box below."
	} catch (e) {
		error.value = String(e)
	}
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
		messages.value.push({
			id: Math.random().toString(36).slice(2),
			direction: 'received',
			source: 'pasted',
			kind: msg.kind,
			text: msg.text,
			file_name: msg.file_name,
			file_size: msg.file_size,
			file_bytes_b64: msg.file_bytes_b64,
			ts: msg.timestamp_ms,
		})
		inFrame.value = ''
		showPaste.value = false
		await refreshLocal()
	} catch (e) {
		error.value = String(e)
	} finally {
		busy.value = false
	}
}

async function saveAttachment(m) {
	try {
		const path = await save({
			defaultPath: m.file_name || 'attachment',
		})
		if (!path) return
		await invoke('write_file_bytes', {
			path,
			contentsB64: m.file_bytes_b64,
		})
		info.value = `Saved to ${path}.`
	} catch (e) {
		error.value = `Save failed: ${e}`
	}
}

function dismiss(id) {
	const idx = messages.value.findIndex((m) => m.id === id)
	if (idx >= 0) messages.value.splice(idx, 1)
}

function formatTs(ms) {
	const d = new Date(Number(ms))
	const today = new Date()
	const isToday =
		d.getDate() === today.getDate() &&
		d.getMonth() === today.getMonth() &&
		d.getFullYear() === today.getFullYear()
	const time = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
	if (isToday) return time
	return `${d.toLocaleDateString()} ${time}`
}

function formatSize(n) {
	if (n == null) return ''
	if (n < 1024) return `${n} B`
	if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
	if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`
	return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`
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
		info.value = 'Linked. New messages will arrive automatically every ~30 seconds.'
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

async function removeFriend() {
	error.value = ''
	removeBusy.value = true
	try {
		await invoke('remove_pairing', { pairingId: local.value.id })
		emit('back')
	} catch (e) {
		error.value = String(e)
		removeBusy.value = false
		confirmingRemove.value = false
	}
}
</script>

<template>
	<div>
		<div class="back-row">
			<button class="btn btn-ghost" type="button" @click="emit('back')">
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
					<path d="M19 12H5" /><path d="M11 18l-6-6 6-6" />
				</svg>
				Back
			</button>
		</div>

		<div class="eyebrow">Friend</div>
		<h1 class="h1">{{ local.name }}</h1>

		<div class="card">
			<div class="card-head">
				<div>
					<div class="h2" style="margin-bottom: 4px">Internet delivery</div>
					<p class="small" style="margin: 0">
						Optional auto-delivery through a private Google Drive
						folder. If you'd rather not use Google, just save
						each message as a small <code>.otp</code> file below.
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
				Google Drive isn't set up in this build. You can still send and
				receive messages by saving them as <code>.otp</code> files.
			</div>
			<div v-else-if="!oauthStatus.connected" class="banner banner-info">
				Sign in to Google Drive in <strong>Settings</strong> to turn on
				auto-delivery, or just save messages as files instead.
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
						</button>
					</div>
					<div v-if="!oauthStatus.picker_configured" class="field-hint">
						The Drive picker isn't set up in this build. See
						<code>CLOUD_SETUP.md</code>.
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
				</div>
			</div>
		</div>

		<!-- Compose -->
		<div class="card">
			<div class="eyebrow">Send a message</div>
			<textarea
				v-model="draft"
				:placeholder="`Type something to ${local.name}…`"
				rows="3"
				:disabled="busy"
			/>
			<div class="compose-actions">
				<button
					class="btn btn-ghost"
					type="button"
					@click="pasteIntoDraft"
					:disabled="busy"
					title="Paste from clipboard"
				>
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<rect x="8" y="4" width="8" height="4" rx="1" />
						<path d="M16 6h2a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h2" />
					</svg>
					Paste
				</button>
				<button
					class="btn btn-ghost"
					type="button"
					@click="sendFile"
					:disabled="busy || sendingFile"
				>
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<path d="M21 10v6a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4V8a4 4 0 0 1 4-4h6" />
						<path d="M16 4l4 4-4 4" />
						<path d="M20 8H10" />
					</svg>
					{{ sendingFile ? 'Encrypting file…' : 'Attach file' }}
				</button>
				<div class="spacer"></div>
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
					{{ outUploaded ? 'Backup copy' : 'Your encrypted message' }}
				</div>
				<p class="small" style="margin-bottom: 10px">
					<template v-if="outUploaded">
						Already sent through Drive. You can also save or copy
						this version to send through another channel as a backup.
					</template>
					<template v-else-if="outFrameKind === 'file'">
						Save this as a <code>.otp</code> file (or copy as text)
						and send it to your friend any way you like. Their app
						will recognise it as an encrypted file.
					</template>
					<template v-else>
						Save this as a <code>.otp</code> file or copy it as text
						and paste into email, Signal, anywhere. Your friend's
						app will decode it.
					</template>
				</p>
				<textarea :value="outFrame" readonly rows="3" />
				<div class="frame-actions">
					<button class="btn btn-primary" type="button" @click="saveOutAsFile">
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
							<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
							<path d="M7 10l5 5 5-5" />
							<path d="M12 15V3" />
						</svg>
						Save as .otp file
					</button>
					<button class="btn btn-ghost" type="button" @click="copyOut">
						Copy text
					</button>
					<button class="btn btn-ghost" type="button" @click="clearOut">
						Clear
					</button>
				</div>
			</div>
		</div>

		<!-- Conversation thread + paste-to-receive -->
		<div class="card">
			<div class="thread-head">
				<div>
					<div class="eyebrow" style="margin-bottom: 0">
						Conversation · gone when you close the app
					</div>
				</div>
				<button
					class="btn btn-ghost btn-small"
					type="button"
					@click="showPaste = !showPaste"
				>
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<rect x="8" y="4" width="8" height="4" rx="1" />
						<path d="M16 6h2a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h2" />
					</svg>
					{{ showPaste ? 'Hide paste box' : 'Paste a message' }}
				</button>
			</div>

			<div v-if="showPaste" class="paste-area">
				<p class="small" style="margin: 0 0 8px">
					Got an encrypted message from <strong>{{ local.name }}</strong>
					somewhere outside the app? Paste it here to decode and add
					it to the conversation.
				</p>
				<textarea
					v-model="inFrame"
					placeholder="Paste an encrypted message (text or .otp file contents)…"
					rows="3"
					:disabled="busy"
				/>
				<div class="row" style="margin-top: 8px">
					<button
						class="btn btn-ghost"
						type="button"
						@click="pasteIntoReceive"
						:disabled="busy"
					>
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
							<rect x="8" y="4" width="8" height="4" rx="1" />
							<path d="M16 6h2a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h2" />
						</svg>
						Paste from clipboard
					</button>
					<div class="spacer"></div>
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

			<div v-if="messages.length === 0" class="thread-empty">
				No messages yet. Send one above, or paste an encrypted message
				to decode.
			</div>

			<div v-else class="thread">
				<div
					v-for="m in messages"
					:key="m.id"
					:class="['bubble', m.direction]"
				>
					<div v-if="m.kind === 'file'" class="bubble-file">
						<div class="bubble-file-icon">
							<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
								<path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" />
								<path d="M13 2v7h7" />
							</svg>
						</div>
						<div class="bubble-file-info">
							<div class="bubble-file-name">{{ m.file_name }}</div>
							<div class="bubble-file-size">
								{{ formatSize(m.file_size) }}
							</div>
						</div>
						<button
							v-if="m.direction === 'received' && m.file_bytes_b64"
							class="btn btn-ghost btn-small"
							type="button"
							@click="saveAttachment(m)"
						>
							Save…
						</button>
					</div>

					<p v-else class="bubble-text">{{ m.text }}</p>

					<div class="bubble-meta">
						<span>{{ formatTs(m.ts) }}</span>
						<span class="pill" :class="m.source === 'drive' ? 'pill-accent' : 'pill-neutral'">
							{{
								m.direction === 'sent'
									? m.source === 'drive'
										? 'sent · drive'
										: 'sent · local'
									: m.source === 'drive'
										? 'via drive'
										: 'pasted in'
							}}
						</span>
						<button class="dismiss" type="button" @click="dismiss(m.id)">
							Dismiss
						</button>
					</div>
				</div>
			</div>
		</div>

		<div v-if="info" class="banner banner-success">{{ info }}</div>
		<div v-if="error" class="banner banner-error">{{ error }}</div>

		<div class="card danger-zone">
			<div class="eyebrow danger-eyebrow">Danger zone</div>
			<p class="small">
				Remove {{ local.name }} from this device and wipe their key
				file. After this you won't be able to read past messages from
				them or send new ones until you meet in person and swap a
				fresh USB stick. The copy of the key on their device is
				untouched — they'll need to remove their side themselves.
			</p>
			<div v-if="!confirmingRemove" class="row" style="margin-top: 12px">
				<button
					class="btn btn-danger"
					type="button"
					@click="confirmingRemove = true"
					:disabled="removeBusy"
				>
					Remove this friend
				</button>
			</div>
			<div v-else class="row" style="margin-top: 12px">
				<button
					class="btn btn-danger"
					type="button"
					@click="removeFriend"
					:disabled="removeBusy"
				>
					{{ removeBusy ? 'Wiping…' : `Yes, remove ${local.name} permanently` }}
				</button>
				<button
					class="btn btn-ghost"
					type="button"
					@click="confirmingRemove = false"
					:disabled="removeBusy"
				>
					Cancel
				</button>
			</div>
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

	.compose-actions {
		display: flex;
		gap: 8px;
		margin-top: 12px;
		align-items: center;
		flex-wrap: wrap;
	}

	.frame-out {
		margin-top: 4px;
	}

	.frame-actions {
		display: flex;
		gap: 8px;
		margin-top: 10px;
		flex-wrap: wrap;
	}

	.thread-head {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 14px;
		gap: 12px;
	}

	.btn-small {
		padding: 6px 12px;
		font-size: 12px;
	}

	.paste-area {
		background: var(--panel-2);
		border: 1px solid var(--line-2);
		border-radius: var(--r-panel);
		padding: 14px;
		margin-bottom: 16px;
	}

	.paste-area textarea {
		margin-top: 4px;
	}

	.danger-zone {
		border-color: color-mix(in oklab, var(--danger) 25%, var(--line));
	}

	.danger-eyebrow::before {
		background: var(--danger);
		box-shadow: 0 0 0 4px
			color-mix(in oklab, var(--danger) 18%, transparent);
	}
</style>
