<script setup>
defineProps({
	pairings: { type: Array, required: true },
	oauthStatus: {
		type: Object,
		default: () => ({ configured: false, connected: false }),
	},
	loadError: { type: String, default: '' },
})
const emit = defineEmits(['create', 'import', 'open', 'refresh', 'settings'])

function formatBytes(n) {
	if (n < 1024) return `${n} B`
	if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
	if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`
	return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`
}

const WARN_THRESHOLD = 10 * 1024 * 1024 // 10 MB
const CRITICAL_THRESHOLD = 256 * 1024 // 256 KB

function padLevel(remaining) {
	if (remaining < CRITICAL_THRESHOLD) return 'critical'
	if (remaining < WARN_THRESHOLD) return 'warn'
	return 'ok'
}

function pairingLevel(p) {
	const a = padLevel(p.out_remaining)
	const b = padLevel(p.in_remaining)
	if (a === 'critical' || b === 'critical') return 'critical'
	if (a === 'warn' || b === 'warn') return 'warn'
	return 'ok'
}
</script>

<template>
	<div>
		<header>
			<h1>OTP Messenger</h1>
			<div class="actions">
				<span class="oauth">
					<span :class="['dot', oauthStatus.connected ? 'on' : 'off']" />
					Drive
				</span>
				<button @click="emit('create')">New pairing</button>
				<button @click="emit('import')">Import pairing</button>
				<button @click="emit('settings')" class="ghost" title="Settings">⚙</button>
				<button @click="emit('refresh')" class="ghost" title="Refresh">↻</button>
			</div>
		</header>

		<div v-if="loadError" class="error">{{ loadError }}</div>

		<div v-if="pairings.length === 0 && !loadError" class="empty">
			No pairings yet. Create one to generate pad material to share via USB,
			or import one a peer handed you.
		</div>

		<ul v-else class="pairings">
			<li
				v-for="p in pairings"
				:key="p.id"
				:class="['pairing', pairingLevel(p)]"
				@click="emit('open', p)"
			>
				<div class="head">
					<span class="name">{{ p.name }}</span>
					<span v-if="p.drive_folder_id" class="badge drive">drive</span>
					<span v-if="pairingLevel(p) === 'critical'" class="badge crit">
						pad critical
					</span>
					<span v-else-if="pairingLevel(p) === 'warn'" class="badge warn">
						pad low
					</span>
				</div>
				<div class="meta">
					<span :class="['stat', padLevel(p.out_remaining)]">
						↑ {{ formatBytes(p.out_remaining) }} /
						{{ formatBytes(p.out_total) }}
					</span>
					<span :class="['stat', padLevel(p.in_remaining)]">
						↓ {{ formatBytes(p.in_remaining) }} /
						{{ formatBytes(p.in_total) }}
					</span>
					<span class="seq">seq {{ p.seq_out }} / {{ p.last_seq_in }}</span>
				</div>
			</li>
		</ul>
	</div>
</template>

<style scoped>
	header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 16px;
		gap: 12px;
	}
	header h1 {
		margin: 0;
	}
	.actions {
		display: flex;
		gap: 8px;
		align-items: center;
	}
	.oauth {
		display: flex;
		align-items: center;
		gap: 6px;
		color: #aaa;
		font-size: 0.85em;
		padding-right: 6px;
	}
	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		display: inline-block;
	}
	.dot.on {
		background: #4caf50;
	}
	.dot.off {
		background: #666;
	}
	.ghost {
		background: transparent;
		border-color: #444;
	}
	.empty {
		color: #888;
		padding: 24px;
		text-align: center;
	}
	.pairings {
		list-style: none;
		padding: 0;
	}
	.pairings li {
		padding: 12px 16px;
		background: #2a2a2a;
		margin-bottom: 8px;
		border-radius: 6px;
		cursor: pointer;
		text-align: left;
		border-left: 3px solid transparent;
	}
	.pairings li.warn {
		border-left-color: #d0c070;
	}
	.pairings li.critical {
		border-left-color: #ff7070;
	}
	.pairings li:hover {
		background: #333;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.name {
		font-weight: 500;
	}
	.badge {
		font-size: 0.7em;
		text-transform: uppercase;
		padding: 1px 6px;
		border-radius: 3px;
		letter-spacing: 0.05em;
	}
	.badge.drive {
		background: #1a3a1a;
		color: #8acc8a;
	}
	.badge.warn {
		background: #2a2410;
		color: #d0c070;
	}
	.badge.crit {
		background: #2a1a1a;
		color: #ff9090;
	}
	.meta {
		display: flex;
		gap: 16px;
		color: #aaa;
		font-size: 0.85em;
		margin-top: 4px;
	}
	.stat.warn {
		color: #d0c070;
	}
	.stat.critical {
		color: #ff9090;
	}
	.seq {
		font-family: monospace;
	}
	.error {
		color: #ff7070;
		padding: 8px;
		background: #2a1a1a;
		border-radius: 6px;
		margin-bottom: 12px;
	}
</style>
