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
		error.value = "Give your friend a name so you can find them in the list."
		return
	}
	if (!usbDir.value) {
		error.value = 'Choose a USB stick to save the key file onto.'
		return
	}
	if (!padSizeMb.value || padSizeMb.value < 1) {
		error.value = 'Key file size must be at least 1 MB.'
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

		<div class="eyebrow">Set up a new friend</div>
		<h1 class="h1">Make a secret to share in person.</h1>
		<p class="lead">
			We'll generate a big random file and write it to a USB stick. Hand
			the USB to your friend so they can import it on their machine. After
			that, you're set — every message you send will use a fresh slice of
			this file as a one-use key.
		</p>

		<div class="card">
			<div class="field">
				<label>What you'll call your friend</label>
				<input
					v-model="name"
					placeholder="e.g. Bob"
					:disabled="busy"
				/>
				<div class="field-hint">
					This is your local label — your friend won't see it.
				</div>
			</div>

			<div class="field">
				<label>Your name (shown to your friend)</label>
				<input
					v-model="hint"
					placeholder="e.g. Alice — work laptop"
					:disabled="busy"
				/>
				<div class="field-hint">
					When your friend imports the USB, they'll see this label so
					they know it's from you.
				</div>
			</div>

			<div class="field">
				<label>USB stick</label>
				<div class="combo">
					<input
						:value="usbDir"
						readonly
						placeholder="Pick a folder on a USB stick"
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
					{{ freeSpaceMb.toLocaleString() }} MB free on this stick.
				</div>
			</div>

			<div class="field">
				<label>Key file size (MB)</label>
				<input
					v-model.number="padSizeMb"
					type="number"
					min="1"
					:disabled="busy"
				/>
				<div class="field-hint">
					Two files of this size are generated — one for messages you
					send, one for messages you receive. Bigger files mean more
					messages before you need to swap a new USB.
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
					{{ busy ? 'Generating…' : 'Generate &amp; save to USB' }}
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
				This can take a while for big files — we're writing secure
				random data to your disk and the USB at the same time. Don't
				unplug the stick.
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
