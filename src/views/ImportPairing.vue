<script setup>
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

const emit = defineEmits(['done', 'cancel'])

const usbPairingDir = ref('')
const name = ref('')
const busy = ref(false)
const error = ref('')

async function pickDir() {
	try {
		const dir = await open({
			directory: true,
			multiple: false,
			title: 'Choose the pairing folder on USB',
		})
		if (!dir) return
		usbPairingDir.value = dir
	} catch (e) {
		error.value = String(e)
	}
}

async function importPairing() {
	error.value = ''
	if (!usbPairingDir.value) {
		error.value = 'Pick the pairing folder on USB.'
		return
	}
	if (!name.value.trim()) {
		error.value = 'Choose a local label for this pairing.'
		return
	}
	busy.value = true
	try {
		await invoke('import_pairing', {
			name: name.value.trim(),
			usbPairingDir: usbPairingDir.value,
		})
		emit('done')
	} catch (e) {
		error.value = String(e)
	} finally {
		busy.value = false
	}
}
</script>

<template>
	<div>
		<header>
			<button @click="emit('cancel')" class="ghost" :disabled="busy">← Back</button>
			<h1>Import pairing</h1>
		</header>

		<p class="help">
			Pick the folder on the USB drive that contains
			<code>pairing.toml</code>, <code>A.pad</code>, and
			<code>B.pad</code>. The pads are copied into local storage; you can
			wipe the USB afterwards.
		</p>

		<div class="field">
			<label>Pairing folder on USB</label>
			<div class="row">
				<input :value="usbPairingDir" readonly placeholder="No folder selected" />
				<button @click="pickDir" :disabled="busy">Choose…</button>
			</div>
		</div>

		<div class="field">
			<label>Your label for this pairing</label>
			<input v-model="name" placeholder="e.g. Alice" :disabled="busy" />
		</div>

		<div v-if="error" class="error">{{ error }}</div>

		<div class="actions">
			<button @click="importPairing" :disabled="busy">
				{{ busy ? 'Importing…' : 'Import' }}
			</button>
		</div>
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
	.help {
		color: #aaa;
		font-size: 0.9em;
		text-align: left;
	}
	.help code {
		background: #1f1f1f;
		padding: 2px 6px;
		border-radius: 4px;
	}
	.field {
		margin-bottom: 16px;
		text-align: left;
	}
	.field label {
		display: block;
		font-size: 0.85em;
		color: #aaa;
		margin-bottom: 4px;
	}
	.field input {
		width: 100%;
		box-sizing: border-box;
	}
	.row {
		display: flex;
		gap: 8px;
	}
	.row input {
		flex: 1;
	}
	.error {
		color: #ff7070;
		padding: 8px;
		background: #2a1a1a;
		border-radius: 6px;
		margin-bottom: 12px;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
	}
</style>
