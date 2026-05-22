<script setup>
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

import PairingList from './views/PairingList.vue'
import CreatePairing from './views/CreatePairing.vue'
import ImportPairing from './views/ImportPairing.vue'
import Conversation from './views/Conversation.vue'

const view = ref({ name: 'list' })
const pairings = ref([])
const loadError = ref('')

async function refresh() {
	try {
		pairings.value = await invoke('list_pairings')
		loadError.value = ''
	} catch (e) {
		loadError.value = String(e)
	}
}

onMounted(refresh)

function open(name, ctx = {}) {
	view.value = { name, ...ctx }
}

async function backToList() {
	await refresh()
	view.value = { name: 'list' }
}
</script>

<template>
	<div class="app">
		<PairingList
			v-if="view.name === 'list'"
			:pairings="pairings"
			:load-error="loadError"
			@create="open('create')"
			@import="open('import')"
			@open="(p) => open('convo', { pairing: p })"
			@refresh="refresh"
		/>
		<CreatePairing
			v-else-if="view.name === 'create'"
			@done="backToList"
			@cancel="backToList"
		/>
		<ImportPairing
			v-else-if="view.name === 'import'"
			@done="backToList"
			@cancel="backToList"
		/>
		<Conversation
			v-else-if="view.name === 'convo'"
			:pairing="view.pairing"
			@back="backToList"
		/>
	</div>
</template>

<style scoped>
	.app {
		max-width: 720px;
		margin: 0 auto;
		padding: 24px;
	}
</style>
