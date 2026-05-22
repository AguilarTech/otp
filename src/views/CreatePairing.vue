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
		<button
			class="btn btn-ghost back"
			type="button"
			@click="emit('cancel')"
			:disabled="busy"
		>
			<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
				<path d="M19 12H5" /><path d="M11 18l-6-6 6-6" />
			</svg>
			Back
		</button>

		<div class="eyebrow">New pairing</div>
		<h1 class="h1">Generate fresh pad material.</h1>
		<p class="lead">
			Two independent random pads are streamed to a USB volume and your
			local app data dir in a single pass. Hand the USB to your peer in
			person — that's the only time the pad material leaves a trusted
			machine.
		</p>

		<div class="card">
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
				<div class="combo">
					<input
						:value="usbDir"
						readonly
						placeholder="No folder selected"
					/>
					<button
						class="btn btn-ghost"
						type="button"
						@click="pickUsb"
						:disabled="busy"
					>
						Choose…
					</button>
				</div>
				<div v-if="freeSpaceMb !== null" class="field-hint">
					{{ freeSpaceMb.toLocaleString() }} MB free on this volume
				</div>
			</div>

			<div class="field">
				<label>Pad size per direction (MB)</label>
				<input
					v-model.number="padSizeMb"
					type="number"
					min="1"
					:disabled="busy"
				/>
				<div class="field-hint">
					Two pads of this size are generated — one per direction. Pad
					material is finite; bigger pad means more messages before a
					re-key.
				</div>
			</div>

			<div v-if="error" class="banner banner-error">{{ error }}</div>

			<div class="row-end">
				<button
					class="btn btn-primary"
					type="button"
					@click="generate"
					:disabled="busy"
				>
					{{ busy ? 'Generating…' : 'Generate pads' }}
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
			</div>

			<p v-if="busy" class="field-hint" style="margin-top: 12px">
				Large pads take a while — OsRng output is being written to local
				storage and the USB simultaneously. Do not unplug the USB.
			</p>
		</div>
	</div>
</template>

<style scoped>
	.back {
		margin-bottom: 18px;
	}

	.combo {
		display: flex;
		gap: 8px;
	}

	.combo input {
		flex: 1;
	}
</style>
