<script setup>
defineProps({
	pairings: { type: Array, required: true },
	oauthStatus: {
		type: Object,
		default: () => ({
			client_configured: false,
			picker_configured: false,
			connected: false,
		}),
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

const WARN_THRESHOLD = 10 * 1024 * 1024
const CRITICAL_THRESHOLD = 256 * 1024

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

function percent(used, total) {
	if (!total) return 0
	return Math.max(0, Math.min(100, (used / total) * 100))
}
</script>

<template>
	<div>
		<div class="head">
			<div>
				<div class="eyebrow">Your pairings</div>
				<h1 class="h1">
					Encrypted 1-to-1 over <br />one-time pads.
				</h1>
				<p class="lead">
					Each pairing is a per-direction stream of pad material shared once
					in person and consumed message by message. No reuse, ever.
				</p>
			</div>

			<div class="head-actions">
				<button class="btn btn-primary" type="button" @click="emit('create')">
					New pairing
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<path d="M5 12h14" /><path d="M13 6l6 6-6 6" />
					</svg>
				</button>
				<button class="btn btn-ghost" type="button" @click="emit('import')">
					Import pairing
				</button>
				<button
					class="btn btn-icon"
					type="button"
					@click="emit('settings')"
					title="Settings"
					aria-label="Settings"
				>
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<circle cx="12" cy="12" r="3" />
						<path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" />
					</svg>
				</button>
				<button
					class="btn btn-icon"
					type="button"
					@click="emit('refresh')"
					title="Refresh"
					aria-label="Refresh"
				>
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<path d="M3 12a9 9 0 0 1 15-6.7L21 8" />
						<path d="M21 3v5h-5" />
						<path d="M21 12a9 9 0 0 1-15 6.7L3 16" />
						<path d="M3 21v-5h5" />
					</svg>
				</button>
			</div>
		</div>

		<div v-if="loadError" class="banner banner-error">{{ loadError }}</div>

		<div v-if="pairings.length === 0 && !loadError" class="card empty">
			<div class="eyebrow">No pairings yet</div>
			<p>
				Create one to generate pad material for a peer (you'll write it to
				USB and hand it over in person), or import a pairing a peer just
				handed you.
			</p>
		</div>

		<ul v-else class="pairings">
			<li
				v-for="p in pairings"
				:key="p.id"
				:class="['pairing-card', pairingLevel(p)]"
				@click="emit('open', p)"
			>
				<div class="pairing-head">
					<span class="name">{{ p.name }}</span>
					<div class="badges">
						<span v-if="p.drive_folder_id" class="pill pill-accent">
							<span class="dot"></span> Drive bound
						</span>
						<span v-if="pairingLevel(p) === 'critical'" class="pill pill-danger">
							<span class="dot"></span> Pad critical
						</span>
						<span
							v-else-if="pairingLevel(p) === 'warn'"
							class="pill pill-warn"
						>
							<span class="dot"></span> Pad low
						</span>
					</div>
				</div>

				<div class="pairing-meters">
					<div class="meter">
						<div class="meter-label">
							<span>Outbound</span>
							<span :class="['stat', padLevel(p.out_remaining)]">
								{{ formatBytes(p.out_remaining) }} of {{ formatBytes(p.out_total) }}
							</span>
						</div>
						<div class="meter-bar">
							<div
								:class="['meter-fill', padLevel(p.out_remaining)]"
								:style="{ width: percent(p.out_total - p.out_remaining, p.out_total) + '%' }"
							></div>
						</div>
					</div>
					<div class="meter">
						<div class="meter-label">
							<span>Inbound</span>
							<span :class="['stat', padLevel(p.in_remaining)]">
								{{ formatBytes(p.in_remaining) }} of {{ formatBytes(p.in_total) }}
							</span>
						</div>
						<div class="meter-bar">
							<div
								:class="['meter-fill', padLevel(p.in_remaining)]"
								:style="{ width: percent(p.in_total - p.in_remaining, p.in_total) + '%' }"
							></div>
						</div>
					</div>
				</div>

				<div class="pairing-foot">
					<span class="mono">seq · sent {{ p.seq_out }} · received {{ p.last_seq_in }}</span>
					<span class="open-hint">
						Open
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
							<path d="M5 12h14" /><path d="M13 6l6 6-6 6" />
						</svg>
					</span>
				</div>
			</li>
		</ul>
	</div>
</template>

<style scoped>
	.head {
		display: grid;
		grid-template-columns: 1fr;
		gap: 20px;
		margin-bottom: 32px;
	}

	@media (min-width: 720px) {
		.head {
			grid-template-columns: 1fr auto;
			align-items: end;
		}
	}

	.head-actions {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
		align-items: center;
	}

	.empty {
		text-align: left;
	}

	.empty p {
		margin: 0;
	}

	.pairings {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.pairing-card {
		background: var(--panel);
		border: 1px solid var(--line);
		border-radius: var(--r-card);
		padding: 18px 22px;
		box-shadow: var(--shadow-card);
		cursor: pointer;
		transition: transform var(--dur-hover) var(--ease-out),
			border-color var(--dur-hover), box-shadow var(--dur-hover);
	}

	.pairing-card:hover {
		transform: translateY(-1px);
		border-color: color-mix(in oklab, var(--accent) 30%, var(--line));
		box-shadow: var(--shadow-elev);
	}

	.pairing-card.warn {
		border-left: 3px solid var(--warn);
	}

	.pairing-card.critical {
		border-left: 3px solid var(--danger);
	}

	.pairing-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		margin-bottom: 14px;
	}

	.name {
		font-family: var(--sans);
		font-weight: 700;
		font-size: 17px;
		letter-spacing: -0.01em;
		color: var(--fg);
	}

	.badges {
		display: flex;
		gap: 6px;
		flex-wrap: wrap;
	}

	.pairing-meters {
		display: grid;
		grid-template-columns: 1fr;
		gap: 12px;
	}

	@media (min-width: 520px) {
		.pairing-meters {
			grid-template-columns: 1fr 1fr;
		}
	}

	.meter-label {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		font-size: 12px;
		color: var(--fg-3);
		margin-bottom: 5px;
	}

	.meter-label > span:first-child {
		font-weight: 600;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		font-size: 10.5px;
	}

	.stat.warn {
		color: var(--warn-fg);
	}
	.stat.critical {
		color: var(--danger-fg);
	}

	.meter-bar {
		height: 4px;
		background: var(--panel-2);
		border-radius: 999px;
		overflow: hidden;
	}

	.meter-fill {
		height: 100%;
		background: var(--accent);
		border-radius: 999px;
		transition: width 0.4s var(--ease-out);
	}

	.meter-fill.warn {
		background: var(--warn);
	}
	.meter-fill.critical {
		background: var(--danger);
	}

	.pairing-foot {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 14px;
		padding-top: 12px;
		border-top: 1px dashed var(--line);
		font-size: 11px;
		color: var(--fg-3);
	}

	.pairing-foot .mono {
		font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
		background: transparent;
		padding: 0;
		font-size: 11px;
		letter-spacing: 0.02em;
	}

	.open-hint {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-weight: 600;
		color: var(--accent);
	}

	.open-hint svg {
		width: 12px;
		height: 12px;
	}
</style>
