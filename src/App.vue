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
const oauthStatus = ref({
	client_configured: false,
	picker_configured: false,
	connected: false,
})
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
	<div class="shell">
		<header class="topbar">
			<div class="topbar-inner">
				<button
					class="wordmark wordmark-btn"
					type="button"
					@click="view = { name: 'list' }"
				>
					AguilarTech<span class="sub">OTP Messenger</span>
				</button>

				<div class="topbar-actions">
					<span
						v-if="view.name === 'list'"
						class="pill"
						:class="oauthStatus.connected ? 'pill-success' : 'pill-neutral'"
					>
						<span class="dot" :class="{ pulse: oauthStatus.connected }"></span>
						{{ oauthStatus.connected ? 'Drive connected' : 'Drive offline' }}
					</span>
				</div>
			</div>
		</header>

		<main class="content reveal" :key="view.name">
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
		</main>
	</div>
</template>

<style scoped>
	.shell {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
	}

	.topbar {
		position: sticky;
		top: 0;
		z-index: 30;
		background: color-mix(in oklab, var(--bg) 82%, transparent);
		backdrop-filter: blur(14px);
		-webkit-backdrop-filter: blur(14px);
		border-bottom: 1px solid var(--line-2);
	}

	.topbar-inner {
		max-width: 760px;
		margin: 0 auto;
		padding: 14px var(--pad-x);
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
	}

	.wordmark-btn {
		background: transparent;
		border: 0;
		padding: 0;
		cursor: pointer;
		font: inherit;
	}

	.wordmark-btn:hover .sub {
		color: var(--fg-2);
	}

	.topbar-actions {
		display: flex;
		gap: 10px;
		align-items: center;
	}

	.content {
		flex: 1;
		max-width: 760px;
		width: 100%;
		margin: 0 auto;
		padding: 32px var(--pad-x) 64px;
	}
</style>
