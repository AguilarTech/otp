<script setup>
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

const emit = defineEmits(['done', 'cancel'])

// Always keep a buffer for the sidecar + other files on the stick.
const BUFFER_MB = 100

// Manual-fallback slider range (used when free-space detection fails).
// Log-scaled so small values are tunable and the high end still reaches 16 GB.
const MANUAL_MAX_MB = 16384
const MANUAL_LOG_MAX = Math.log10(MANUAL_MAX_MB)

const name = ref('')
const hint = ref('')
const usbDir = ref('')
const freeBytes = ref(null)
const usableMb = ref(0)
const percent = ref(95)
const manualPercent = ref(71) // ≈ 1 GB by default
const busy = ref(false)
const error = ref('')
const detectError = ref('')

const knowsFreeSpace = computed(
	() => freeBytes.value != null && usableMb.value > 0,
)

const freeMb = computed(() =>
	freeBytes.value != null ? Math.floor(freeBytes.value / (1024 * 1024)) : null,
)

const manualMb = computed(() => {
	const p = Math.max(0, Math.min(100, manualPercent.value))
	const raw = Math.pow(10, (p / 100) * MANUAL_LOG_MAX)
	if (raw < 10) return Math.max(1, Math.round(raw))
	return Math.max(1, Math.round(raw / 10) * 10)
})

const padSizeMb = computed(() => {
	if (knowsFreeSpace.value) {
		return Math.max(1, Math.floor((usableMb.value * percent.value) / 100))
	}
	return manualMb.value
})

const padSizeLabel = computed(() => {
	const mb = padSizeMb.value
	if (mb >= 1024) return `${(mb / 1024).toFixed(2)} GB`
	return `${mb.toLocaleString()} MB`
})

const usableLabel = computed(() => {
	const mb = usableMb.value
	if (mb >= 1024) return `${(mb / 1024).toFixed(2)} GB`
	return `${mb.toLocaleString()} MB`
})

async function pickUsb() {
	error.value = ''
	detectError.value = ''
	try {
		const dir = await open({
			directory: true,
			multiple: false,
			title: 'Choose a folder for the key file',
		})
		if (!dir) return
		usbDir.value = dir
		try {
			const free = await invoke('usb_free_space', { path: dir })
			const bytes = Number(free)
			if (!Number.isFinite(bytes) || bytes <= BUFFER_MB * 1024 * 1024) {
				freeBytes.value = null
				usableMb.value = 0
				detectError.value =
					'This folder doesn\'t have enough free space to leave a safety buffer. Enter a size manually below.'
			} else {
				freeBytes.value = bytes
				const totalMb = Math.floor(bytes / (1024 * 1024))
				usableMb.value = Math.max(1, totalMb - BUFFER_MB)
				percent.value = 95
			}
		} catch (e) {
			freeBytes.value = null
			usableMb.value = 0
			detectError.value = `Couldn't read free space (${e}). Enter a size manually below.`
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
		error.value = 'Choose a folder to save the key file into first.'
		return
	}
	if (padSizeMb.value < 1) {
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
		<div class="back-row">
			<button
				class="btn btn-ghost"
				type="button"
				@click="emit('cancel')"
				:disabled="busy"
			>
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
					<path d="M19 12H5" /><path d="M11 18l-6-6 6-6" />
				</svg>
				Back
			</button>
		</div>

		<div class="eyebrow">Set up a new friend</div>
		<h1 class="h1">Make a secret to share in person.</h1>
		<p class="lead">
			We'll generate a big random file and write it to a folder of your
			choice — ideally a USB stick you can hand to your friend. After
			they import it, every message you send will use a fresh slice of
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
				<label>Destination folder</label>
				<div class="combo">
					<input
						:value="usbDir"
						readonly
						placeholder="Pick a folder (USB stick recommended)"
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
				<div v-if="freeMb !== null" class="field-hint">
					{{ freeMb.toLocaleString() }} MB free on this drive · we'll
					leave {{ BUFFER_MB }} MB for safety.
				</div>
				<div v-else-if="detectError" class="field-hint">
					{{ detectError }}
				</div>
			</div>

			<!-- Slider mode (free space known) -->
			<div class="field" v-if="knowsFreeSpace">
				<label class="slider-label">
					<span>Key file size</span>
					<span class="slider-value">{{ padSizeLabel }} ({{ percent }}%)</span>
				</label>
				<input
					type="range"
					class="slider"
					min="1"
					max="100"
					v-model.number="percent"
					:style="{ '--fill': percent + '%' }"
					:disabled="busy"
				/>
				<div class="slider-scale">
					<span>1%</span>
					<span>100% · {{ usableLabel }}</span>
				</div>
				<div class="field-hint">
					We always reserve {{ BUFFER_MB }} MB for the sidecar file
					and other things on the stick. Bigger key file = more
					messages before you need a fresh swap.
				</div>
			</div>

			<!-- Manual mode (free space not detectable): same drag-style slider
				 on a log scale so tiny and large values are both reachable. -->
			<div class="field" v-else-if="usbDir">
				<label class="slider-label">
					<span>Key file size</span>
					<span class="slider-value">{{ padSizeLabel }}</span>
				</label>
				<input
					type="range"
					class="slider"
					min="0"
					max="100"
					v-model.number="manualPercent"
					:style="{ '--fill': manualPercent + '%' }"
					:disabled="busy"
				/>
				<div class="slider-scale">
					<span>1 MB</span>
					<span>16 GB</span>
				</div>
				<div class="field-hint">
					Bigger key file = more messages before you need a fresh
					swap. Drag right for more.
				</div>
			</div>

			<div v-else class="field-hint" style="margin-bottom: 16px">
				Pick a destination folder first.
			</div>

			<div v-if="error" class="banner banner-error">{{ error }}</div>

			<div class="row-end">
				<button
					class="btn btn-primary"
					type="button"
					@click="generate"
					:disabled="busy || !usbDir || padSizeMb < 1"
				>
					{{ busy ? 'Generating…' : 'Generate &amp; save' }}
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
				random data to your disk and the chosen folder at the same
				time. Don't disconnect the drive.
			</p>
		</div>
	</div>
</template>

<style scoped>
	.combo {
		display: flex;
		gap: 8px;
	}

	.combo input {
		flex: 1;
	}

	.slider-label {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 12px;
	}

	.slider-value {
		font-weight: 700;
		color: var(--accent);
		font-size: 14px;
		letter-spacing: -0.01em;
		text-transform: none;
	}

	.slider-scale {
		display: flex;
		justify-content: space-between;
		font-size: 10.5px;
		color: var(--fg-3);
		margin-top: -2px;
		margin-bottom: 6px;
	}
</style>
