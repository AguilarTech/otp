<script setup>
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps({ pairing: { type: Object, required: true } })
const emit = defineEmits(['back'])

const draft = ref('')
const outFrame = ref('')
const inFrame = ref('')
const inbox = ref([])
const error = ref('')
const busy = ref(false)

async function send() {
	error.value = ''
	if (!draft.value.length) {
		error.value = 'Type something to send.'
		return
	}
	busy.value = true
	try {
		const frame = await invoke('encrypt_message', {
			pairingId: props.pairing.id,
			plaintext: draft.value,
		})
		outFrame.value = frame
		draft.value = ''
	} catch (e) {
		error.value = String(e)
	} finally {
		busy.value = false
	}
}

async function copyOut() {
	try {
		await navigator.clipboard.writeText(outFrame.value)
	} catch (e) {
		error.value = `Copy failed: ${e}`
	}
}

function clearOut() {
	outFrame.value = ''
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
		})
		inFrame.value = ''
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
</script>

<template>
	<div>
		<header>
			<button @click="emit('back')" class="ghost">← Back</button>
			<h1>{{ pairing.name }}</h1>
		</header>

		<section>
			<h2>Compose</h2>
			<textarea
				v-model="draft"
				placeholder="Type a message…"
				rows="3"
				:disabled="busy"
			/>
			<div class="row">
				<button @click="send" :disabled="busy">Encrypt</button>
			</div>
			<div v-if="outFrame" class="frame">
				<label>Frame to send (paste into your transport):</label>
				<textarea :value="outFrame" readonly rows="3" />
				<div class="row">
					<button @click="copyOut">Copy</button>
					<button @click="clearOut" class="ghost">Clear</button>
				</div>
			</div>
		</section>

		<section>
			<h2>Receive</h2>
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
					<button class="ghost" @click="dismiss(m.id)">Dismiss</button>
				</div>
				<pre>{{ m.text }}</pre>
			</div>
		</section>

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
	.frame {
		margin-top: 12px;
	}
	.frame label {
		font-size: 0.8em;
		color: #888;
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
