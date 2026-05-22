<script setup>
import AboutContent from '../components/AboutContent.vue'

const emit = defineEmits(['back', 'create', 'import'])
</script>

<template>
	<div>
		<div class="back-row">
			<button class="btn btn-ghost" type="button" @click="emit('back')">
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
					<path d="M19 12H5" /><path d="M11 18l-6-6 6-6" />
				</svg>
				Back
			</button>
		</div>

		<AboutContent
			:show-ctas="true"
			@create="emit('create')"
			@import="emit('import')"
		/>
	</div>
</template>
