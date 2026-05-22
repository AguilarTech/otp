<script setup>
	import { defineProps } from 'vue'
	import { ref } from 'vue'
	import { invoke } from '@tauri-apps/api/tauri'
	import ErrorModal from './ErrorModal.vue'

	const props = defineProps({
		keyPath: String // Define the prop and its type
	})

	const showErrorModal = ref(false)
	const errorMessage = ref('')
	const decryptMessage = ref('')

	async function decrypt() {
		console.log(props.keyPath)
		try {
			// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
			const decryptedMsg = await invoke('decrypt', {
				encryptedMsg: decryptMessage.value,
				filePath: props.keyPath
			})
			decryptMessage.value = decryptedMsg
		} catch (error) {
			console.error('Decryption failed:', error)
			errorMessage.value = `Select a file to use as the decryption key.`
			showErrorModal.value = true
		}
	}
</script>

<template>
	<div class="decode-message">
		<form @submit.prevent="decrypt">
			<textarea
				id="coded-message-input"
				v-model="decryptMessage"
				placeholder="Enter coded message..."
				rows="5"
				class="message-input"
			></textarea>
			<button type="submit" class="decode-button">Decode</button>
			<ErrorModal
				:showError="showErrorModal"
				:errorMessage="errorMessage"
				@close="showErrorModal = false"
			/>
		</form>
	</div>
</template>

<style scoped>
	.decode-message {
		margin: 20px 0px;
	}

	.message-input {
		width: 95%;
		margin-bottom: 20px;
	}
</style>
