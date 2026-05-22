<script setup>
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

const emit = defineEmits(['done', 'cancel'])

const name = ref('')
const hint = ref('')
const usbDir = ref('')
const freeSpace = ref(null)
const padSizeMb = ref(1024)
const busy = ref(false)
const error = ref('')

const freeSpaceMb = computed(() =>
	freeSpace.value != null ? Math.floor(freeSpace.value / (1024 * 1024)) : null,
)

async function pickUsb() {
	try {
		const dir = await open({
			directory: true,
			multiple: false,
			title: 'Choose USB destination',
		})
		if (!dir) return
		usbDir.value = dir
		try {
			const free = await invoke('usb_free_space', { path: dir })
			freeSpace.value = free
			const headroom = 50 * 1024 * 1024
			const suggestedMb = Math.max(
				1,
				Math.floor((Number(free) - headroom) / (1024 * 1024)),
			)
			padSizeMb.value = suggestedMb
		} catch {
			freeSpace.value = null
		}
	} catch (e) {
		error.value = String(e)
	}
}

async function generate() {
	error.value = ''
	if (!name.value.trim()) {
		error.value = 'Pairing name required.'
		return
	}
	if (!usbDir.value) {
		error.value = 'Pick a USB destination first.'
		return
	}
	if (!padSizeMb.value || padSizeMb.value < 1) {
		error.value = 'Pad size must be at least 1 MB.'
		return
	}
	busy.value = true
	try {
		await invoke('create_pairing', {
			name: name.value.trim(),
			originatorHint: hint.value.trim(),
			padSizeBytes: padSizeMb.value * 1024 * 1024,
			usbDir: usbDir.value,
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
			<h1>New pairing</h1>
		</header>

		<div class="field">
			<label>Pairing name (your local label for the peer)</label>
			<input v-model="name" placeholder="e.g. Bob" :disabled="busy" />
		</div>

		<div class="field">
			<label>Hint shown to the peer on import</label>
			<input
				v-model="hint"
				placeholder="e.g. Alice — work laptop"
				:disabled="busy"
			/>
		</div>

		<div class="field">
			<label>USB destination</label>
			<div class="row">
				<input :value="usbDir" readonly placeholder="No folder selected" />
				<button @click="pickUsb" :disabled="busy">Choose…</button>
			</div>
			<div v-if="freeSpaceMb !== null" class="hint">{{ freeSpaceMb }} MB free on this volume</div>
		</div>

		<div class="field">
			<label>Pad size per direction (MB)</label>
			<input v-model.number="padSizeMb" type="number" min="1" :disabled="busy" />
			<div class="hint">
				Two independent pads of this size are generated, one per direction.
				Pad material is finite — bigger pad means more messages before a re-key.
			</div>
		</div>

		<div v-if="error" class="error">{{ error }}</div>

		<div class="actions">
			<button @click="generate" :disabled="busy">
				{{ busy ? 'Generating…' : 'Generate pads' }}
			</button>
		</div>

		<p v-if="busy" class="hint">
			Large pads take a while — OsRng output is being written to local
			storage and the USB simultaneously. Do not unplug the USB.
		</p>
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
	.hint {
		font-size: 0.8em;
		color: #888;
		margin-top: 4px;
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
