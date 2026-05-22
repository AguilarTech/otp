<script setup>
	import { ref } from 'vue'
	import { open } from '@tauri-apps/api/dialog'
	// import { invoke } from '@tauri-apps/api/tauri'

	const filePath = ref('')
	const emit = defineEmits(['file-selected'])

	async function selectFile() {
		try {
			const selectedPath = await open({})
			if (selectedPath) {
				filePath.value = selectedPath
				console.log('Received file path:', filePath.value)
				emit('file-selected', selectedPath)
			}
		} catch (error) {
			console.error('Failed to select file:', error)
		}
	}
</script>

<template>
	<div class="file-loader">
		<div v-if="filePath">{{ filePath }}</div>
		<button type="button" @click="selectFile">Select File</button>
	</div>
</template>

<style scoped>
	.file-loader {
		margin: 20px 0px;
	}

	div {
		margin-bottom: 20px;
	}
</style>
