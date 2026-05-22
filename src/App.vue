<script setup>
	// This starter template is using Vue 3 <script setup> SFCs
	// Check out https://vuejs.org/api/sfc-script-setup.html#script-setup
	import { ref } from 'vue'

	import FileLoader from './components/FileLoader.vue'
	import DecodeMessage from './components/DecodeMessage.vue'
	import EncryptMessage from './components/EncryptMessage.vue'
	import GenerateKey from './components/GenerateKey.vue'

	const keyPath = ref(null)

	const onFileSelected = (filePath) => {
		// Handle the loaded file here
		console.log('File Path in App.vue:', filePath)
		keyPath.value = filePath
	}
</script>

<template>
	<div class="container">
		<div class="row">
			<a href="https://AguilarTech.com.au" target="_blank">
				<img
					src="/logo.png"
					class="logo AguilarTech"
					alt="AguilarTech"
				/>
			</a>
		</div>

		<h1>One-time Pad Encryption Messanger</h1>

		<p>
			In cryptography, the
			<a href="https://en.wikipedia.org/wiki/One-time_pad"
				>one-time pad (OTP) </a
			>is an encryption technique that cannot be cracked, but requires the
			use of a single-use pre-shared key that is larger than or equal to
			the size of the message being sent. In this technique, a plaintext
			is paired with a random secret key (also referred to as a one-time
			pad). Then, each bit or character of the plaintext is encrypted by
			combining it with the corresponding bit or character from the pad
			using modular addition.
		</p>

		<div class="line-separator"></div>

		<h2>Load One-time pad key</h2>

		<FileLoader @file-selected="onFileSelected" />

		<h2>Decode Message</h2>

		<DecodeMessage :keyPath="keyPath" />

		<h2>Encode Message</h2>

		<EncryptMessage :keyPath="keyPath" />

		<div class="line-separator"></div>

		<h1>Generate new key</h1>

		<h2>Create new One-time pad key</h2>

		<GenerateKey />
	</div>
</template>

<style scoped>
	.logo.AguilarTech:hover {
		filter: drop-shadow(0 0 2em #747bff);
	}
	.logo.AguilarTech {
		max-height: 70px;
	}

	.logo.vue:hover {
		filter: drop-shadow(0 0 2em #249b73);
	}

	.line-separator {
		border-bottom: 1px solid #ccc; /* Adjust color and thickness as needed */
		margin: 20px 0; /* Adjust spacing as needed */
	}
</style>
