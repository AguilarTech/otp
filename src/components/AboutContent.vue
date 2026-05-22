<script setup>
defineProps({
	showCtas: { type: Boolean, default: true },
})
const emit = defineEmits(['create', 'import'])
</script>

<template>
	<div>
		<section class="hero">
			<div class="eyebrow">Private messaging done right</div>
			<h1 class="h1">
				Send messages only your friend can read.
			</h1>
			<p class="lead">
				OTP Messenger uses the
				<strong>one-time pad</strong> — a classic encryption
				technique that is mathematically impossible to break, even by
				a quantum computer. The catch is you have to meet your friend
				in person <em>once</em>, to swap a USB stick. After that, you
				can message them from anywhere and your words stay completely
				private. Not even Google or your internet provider can read
				them.
			</p>
			<div v-if="showCtas" class="hero-cta">
				<button
					class="btn btn-primary"
					type="button"
					@click="emit('create')"
				>
					Set up a friend on USB
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<path d="M5 12h14" /><path d="M13 6l6 6-6 6" />
					</svg>
				</button>
				<button
					class="btn btn-ghost"
					type="button"
					@click="emit('import')"
				>
					I have a USB from a friend
				</button>
			</div>
		</section>

		<section class="steps">
			<div class="eyebrow">How it works</div>
			<ol class="step-grid">
				<li class="step-card">
					<div class="step-num">01</div>
					<div class="h3">Trade a secret in person</div>
					<p>
						Generate a big random file on your laptop. Copy it to a
						USB stick. Hand the USB to your friend so they can
						import it. Now you both have the same secret — and only
						you two.
					</p>
				</li>
				<li class="step-card">
					<div class="step-num">02</div>
					<div class="h3">Type, encrypt, send</div>
					<p>
						Every message borrows a fresh slice of the secret file
						as a one-use key. The slice is then erased from your
						disk. Even if someone steals your laptop tomorrow,
						yesterday's messages stay unreadable.
					</p>
				</li>
				<li class="step-card">
					<div class="step-num">03</div>
					<div class="h3">Deliver any way you like</div>
					<p>
						Sign in to Google Drive and we auto-deliver for you. Or
						save each message as a small <code>.otp</code> file and
						email/Signal/AirDrop it. Your friend's app opens it on
						the other end.
					</p>
				</li>
				<li class="step-card">
					<div class="step-num">04</div>
					<div class="h3">No history kept</div>
					<p>
						Messages live in memory until you dismiss them, then
						they're gone. Close the app and the inbox empties.
						Reopening won't bring anything back.
					</p>
				</li>
			</ol>
		</section>

		<section class="learn-more">
			<div class="eyebrow">Learn more</div>
			<div class="faq">
				<details class="faq-item">
					<summary>
						<span>What's a one-time pad?</span>
						<svg class="faq-chev" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
							<path d="M6 9l6 6 6-6" />
						</svg>
					</summary>
					<div class="faq-body">
						<p>
							A one-time pad is a big file of random data that you
							and a friend share. To encrypt a message, the app
							takes a fresh slice of that file and mixes it with
							your message — like overlaying a layer of perfect
							static. The slice is then erased and
							<strong>never reused</strong>.
						</p>
						<p>
							The technique is over 140 years old and was used to
							secure the Washington–Moscow hot line during the
							Cold War. It's still the only encryption that's
							mathematically proven unbreakable.
						</p>
					</div>
				</details>

				<details class="faq-item">
					<summary>
						<span>Why is it unbreakable?</span>
						<svg class="faq-chev" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
							<path d="M6 9l6 6 6-6" />
						</svg>
					</summary>
					<div class="faq-body">
						<p>
							Most encryption is <em>hard</em> to break — a fast
							enough computer, or a future quantum computer, could
							in theory grind through it. A one-time pad is
							<strong>impossible</strong> to break.
						</p>
						<p>
							Every message gets mixed with truly random data the
							attacker has no way to guess. To them, every
							possible plaintext is equally likely. There's
							literally no information to attack — nothing to
							brute-force, no clever shortcut.
						</p>
						<p>
							The trade-off: you have to meet your friend in
							person <em>once</em> to swap the key file. There's
							no way around that first handoff. After that, all
							your messaging is digital.
						</p>
					</div>
				</details>

				<details class="faq-item">
					<summary>
						<span>How do I send my first message?</span>
						<svg class="faq-chev" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
							<path d="M6 9l6 6 6-6" />
						</svg>
					</summary>
					<div class="faq-body">
						<p>
							<strong>1.</strong> Click
							<strong>Set up a friend on USB</strong> and meet
							your friend in person.
						</p>
						<p>
							<strong>2.</strong> Plug in a USB stick on your
							laptop, generate the key file, hand the USB over.
						</p>
						<p>
							<strong>3.</strong> They open the app on their
							laptop, click
							<strong>I have a USB from a friend</strong>, and
							import the file.
						</p>
						<p>
							<strong>4.</strong> You're done. Open their contact
							card, type a message, hit Encrypt. Send the
							encrypted text through Google Drive (if you sign in)
							or save it as a <code>.otp</code> file and email /
							Signal / AirDrop it.
						</p>
					</div>
				</details>

				<details class="faq-item">
					<summary>
						<span>Do I need Google Drive?</span>
						<svg class="faq-chev" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
							<path d="M6 9l6 6 6-6" />
						</svg>
					</summary>
					<div class="faq-body">
						<p>
							<strong>No.</strong> Google Drive is just an
							optional auto-delivery channel. Without it, every
							encrypted message can be saved as a small
							<code>.otp</code> file (or copied as text) and sent
							through whatever you already use — email, Signal,
							Telegram, AirDrop, a USB stick, even printed paper.
							Same security guarantees.
						</p>
						<p>
							If you do sign in, Google sees the size and timing
							of encrypted blobs and which Google accounts they
							pass between. <strong>Never the contents.</strong>
						</p>
					</div>
				</details>

				<details class="faq-item">
					<summary>
						<span>What does the app keep on disk?</span>
						<svg class="faq-chev" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
							<path d="M6 9l6 6 6-6" />
						</svg>
					</summary>
					<div class="faq-body">
						<p>
							<strong>Keeps:</strong> The key files for each of
							your friends, and how much of each one you've used.
							That's it.
						</p>
						<p>
							<strong>Doesn't keep:</strong> Any message history.
							Plaintext lives in memory while the app is open and
							disappears when you dismiss it or close the window.
							Reopening won't bring anything back.
						</p>
					</div>
				</details>
			</div>
		</section>
	</div>
</template>

<style scoped>
	.hero {
		margin-bottom: 40px;
	}

	.hero .h1 {
		font-size: clamp(28px, 3.4vw, 44px);
		margin-bottom: 22px;
	}

	.hero .lead {
		max-width: 60ch;
	}

	.hero .lead strong {
		font-weight: 700;
		color: var(--fg);
	}

	.hero .lead em {
		font-style: italic;
		color: var(--accent);
		font-weight: 600;
	}

	.hero-cta {
		display: flex;
		gap: 10px;
		flex-wrap: wrap;
		margin-top: 28px;
	}

	.steps {
		margin-bottom: 32px;
	}

	.step-grid {
		list-style: none;
		padding: 0;
		margin: 0;
		display: grid;
		grid-template-columns: 1fr;
		gap: 12px;
	}

	@media (min-width: 640px) {
		.step-grid {
			grid-template-columns: 1fr 1fr;
		}
	}

	.step-card {
		background: var(--panel);
		border: 1px solid var(--line);
		border-radius: var(--r-card);
		padding: 22px 24px;
		box-shadow: var(--shadow-card);
	}

	.step-num {
		font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
		font-size: 11px;
		font-weight: 600;
		color: var(--accent);
		letter-spacing: 0.12em;
		margin-bottom: 12px;
	}

	.step-card .h3 {
		margin-bottom: 8px;
		font-size: 16px;
	}

	.step-card p {
		margin: 0;
		font-size: 13.5px;
		line-height: 1.55;
	}

	.step-card p code {
		font-size: 12px;
	}

	.learn-more {
		margin-bottom: 32px;
	}
</style>
