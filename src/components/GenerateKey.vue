<script setup>
	import { ref } from 'vue'
	import { appWindow } from '@tauri-apps/api/window'
	import { invoke } from '@tauri-apps/api/tauri'
	import { save } from '@tauri-apps/api/dialog'
	import ErrorModal from './ErrorModal.vue'

	const fromName = ref('')
	const toName = ref('')

	const selectedSize = ref('1024')
	const progress = ref(0)
	const isGeneratingFile = ref(false)

	const showErrorModal = ref(false)
	const errorMessage = ref('')

	async function generateKey() {
		// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command

		// Check if fromName or toName are empty
		if (!fromName.value.trim() || !toName.value.trim()) {
			console.error('Error: fromName and toName must not be empty.')
			errorMessage.value =
				'Your Name and the Name of the recipient are required.'
			showErrorModal.value = true
			return // Stop the function execution
		} else {
			errorMessage.value = '' // Clear the error message if input is valid
		}

		// Set up a listener for progress updates

		appWindow.listen('otp-key-generation-progress', (event) => {
			progress.value = Math.round(event.payload)
			// console.log(`\rProgress: ${Math.round(event.payload)}%`)
		})

		appWindow.listen('otp-key-generation-complete', (event) => {
			progress.value = 100
			console.log(`\rComplete!`)
		})

		isGeneratingFile.value = true

		// Trigger the key generation process
		try {
			// Generate a timestamp in _YYYYMMDDHHmm format using toISOString
			const timestamp = new Date()
				.toISOString()
				.replace(/[-:T]/g, '')
				.slice(0, 12)

			// Sanitize fromName and toName by trimming whitespace
			const sanitizedFromName = fromName.value.trim()
			const sanitizedToName = toName.value.trim()

			const defaultName = `${
				sanitizedFromName.charAt(0).toUpperCase() +
				sanitizedFromName.slice(1).toLowerCase()
			}_${
				sanitizedToName.charAt(0).toUpperCase() +
				sanitizedToName.slice(1).toLowerCase()
			}_${timestamp}`

			// Ask the user to choose a location to save the file
			const selectedPath = await save({
				defaultPath: defaultName
			})

			if (!selectedPath) {
				// User canceled the save dialog
				console.log('File save was canceled by the user.')
				isGeneratingFile.value = false
				return
			}

			// Extract directory from selectedPath
			const directory = selectedPath.substring(
				0,
				selectedPath.lastIndexOf('\\') + 1 || selectedPath.length
			)

			// Concatenate directory with defaultName to form the new filePath
			const filePath = directory + defaultName

			console.log(`selectedPath: ${filePath}`)

			// Invoke the Rust command to generate and save the OTP key
			const response = await invoke('generate_otp_key', {
				filePath: filePath,
				fileSize: Number(selectedSize.value)
			})

			console.log(` ${response}`)
		} catch (error) {
			console.error(`Failed to save key: ${error}`)
		}
		// isGeneratingFile.value = false
	}
</script>

<template>
	<h2
		class="progress"
		:style="{ display: isGeneratingFile ? 'block' : 'none' }"
	>
		Generating Key File: {{ progress }}%
	</h2>

	<form class="row" @submit.prevent="generateKey">
		<input id="greet-input" v-model="fromName" placeholder="Your Name" />
		<input
			id="greet-input"
			v-model="toName"
			placeholder="Name of recipient"
		/>

		<select v-model="selectedSize" class="file-size-selector">
			<option value="512">512 MB</option>
			<option value="1024">1 GB</option>
			<option value="2048">2 GB</option>
			<option value="3072">3 GB</option>
			<option value="4096">4 GB</option>
			<option value="6144">6 GB</option>
			<option value="8192">8 GB</option>
			<option value="10240">10 GB</option>
			<option value="12288">12 GB</option>
			<option value="16384">16 GB</option>
		</select>

		<button type="submit">Generate Key</button>
	</form>
	<!-- <div v-if="errorMessage" class="error-message">{{ errorMessage }}</div> -->

	<ErrorModal
		:showError="showErrorModal"
		:errorMessage="errorMessage"
		@close="showErrorModal = false"
	/>
</template>

<style scoped>
	.progress {
		margin: 20px;
	}
	.error-message {
		color: #ff5555;
		margin: 20px;
	}

	.file-size-selector {
		border-radius: 8px;
		border: 1px solid transparent;
		padding: 0.5em 0.5em;
		margin: 0 5px;
		font-size: 0.9em;
		font-weight: 500;
		font-family: inherit;
		color: #ffffff;
		background-color: #0f0f0f98;
		transition: border-color 0.25s;
		box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
	}
	.file-size-selector:hover {
		border-color: #396cd8;
	}
</style>
