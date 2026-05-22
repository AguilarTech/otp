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
	const Message = ref('')

	async function encrypt() {
		console.log(props.keyPath)
		try {
			// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
			const encryptedMsg = await invoke('encrypt', {
				plaintextMsg: Message.value,
				filePath: props.keyPath
			})
			Message.value = encryptedMsg
		} catch (error) {
			console.error('Encryption failed:', error)
			errorMessage.value = `Select a file to use as the encryption key.`
			showErrorModal.value = true
		}
	}
</script>

<template>
	<div class="encrypt-message">
		<!-- File Path: {{ keyPath }} -->
		<form @submit.prevent="encrypt">
			<textarea
				id="message-input"
				v-model="Message"
				placeholder="Enter message..."
				rows="5"
				class="message-input"
			></textarea>
			<button type="submit" class="encode-button">Encrypt</button>
			<ErrorModal
				:showError="showErrorModal"
				:errorMessage="errorMessage"
				@close="showErrorModal = false"
			/>
		</form>
	</div>
</template>

<style scoped>
	.encrypt-message {
		margin: 20px 0px;
	}

	.message-input {
		width: 95%;
		margin-bottom: 20px;
	}
</style>
