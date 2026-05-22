<script setup>
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

import PairingList from './views/PairingList.vue'
import CreatePairing from './views/CreatePairing.vue'
import ImportPairing from './views/ImportPairing.vue'
import Conversation from './views/Conversation.vue'
import Settings from './views/Settings.vue'

const view = ref({ name: 'list' })
const pairings = ref([])
const oauthStatus = ref({ configured: false, connected: false })
const loadError = ref('')

async function refresh() {
	try {
		pairings.value = await invoke('list_pairings')
		oauthStatus.value = await invoke('oauth_status')
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
			:oauth-status="oauthStatus"
			:load-error="loadError"
			@create="open('create')"
			@import="open('import')"
			@settings="open('settings')"
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
			@pairing-changed="refresh"
		/>
		<Settings v-else-if="view.name === 'settings'" @back="backToList" />
	</div>
</template>

<style scoped>
	.app {
		max-width: 720px;
		margin: 0 auto;
		padding: 24px;
	}
</style>
