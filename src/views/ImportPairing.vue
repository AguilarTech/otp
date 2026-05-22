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

		<div class="eyebrow">Import pairing</div>
		<h1 class="h1">Bring in pad material a peer handed you.</h1>
		<p class="lead">
			Pick the folder on the USB drive that contains
			<code>pairing.toml</code>, <code>A.pad</code>, and
			<code>B.pad</code>. The pads are copied into local app data with the
			send / receive roles swapped — you can wipe the USB once import
			finishes.
		</p>

		<div class="card">
			<div class="field">
				<label>Pairing folder on USB</label>
				<div class="combo">
					<input
						:value="usbPairingDir"
						readonly
						placeholder="No folder selected"
					/>
					<button
						class="btn btn-ghost"
						type="button"
						@click="pickDir"
						:disabled="busy"
					>
						Choose…
					</button>
				</div>
			</div>

			<div class="field">
				<label>Your label for this pairing</label>
				<input
					v-model="name"
					placeholder="e.g. Alice"
					:disabled="busy"
				/>
			</div>

			<div v-if="error" class="banner banner-error">{{ error }}</div>

			<div class="row-end">
				<button
					class="btn btn-primary"
					type="button"
					@click="importPairing"
					:disabled="busy"
				>
					{{ busy ? 'Importing…' : 'Import' }}
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
