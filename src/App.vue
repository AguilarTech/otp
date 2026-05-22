<script setup>
import { ref } from 'vue'

const phase = ref('Phase 0 + 1 complete')
</script>

<template>
	<div class="container">
		<a href="https://AguilarTech.com.au" target="_blank">
			<img src="/logo.png" class="logo" alt="AguilarTech" />
		</a>
		<h1>OTP Messenger</h1>
		<p class="status">{{ phase }} &mdash; demo cleared, Tauri 2 scaffold in place.</p>
		<p class="next">Pairings, vault, and Drive transport land in subsequent commits.</p>
	</div>
</template>

<style scoped>
	.container {
		text-align: center;
		padding: 40px 20px;
	}

	.logo {
		max-height: 70px;
		padding: 1em;
	}

	.status,
	.next {
		color: #aaa;
		font-size: 0.9em;
	}
</style>
