<template>
	<div v-if="props.showError" class="error-modal" @click="closeModal">
		<div class="modal-content">
			<span class="close" @click.stop="closeModal">&times;</span>
			<p>{{ errorMessage }}</p>
		</div>
	</div>
</template>

<script setup>
	const props = defineProps({
		showError: Boolean,
		errorMessage: String
	})

	const emit = defineEmits(['close'])

	const closeModal = () => {
		emit('close')
	}
</script>

<style>
	/* Add your modal styling here */
	.error-modal {
		position: fixed;
		top: 0;
		left: 0;
		width: 100vw; /* Cover the full viewport width */
		height: 100vh; /* Cover the full viewport height */
		display: flex;
		justify-content: center;
		align-items: center;
		background-color: rgba(0, 0, 0, 0.5); /* Semi-transparent background */
		box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1); /* Soft shadow for depth */
	}

	.modal-content {
		width: 80%; /* Adjust based on your preference */
		max-width: 500px; /* Maximum width */
		margin: auto; /* Center horizontally */
		padding: 20px;
		background-color: #333; /* Slightly different color to highlight the modal */
		color: #f6f6f6;
		border-radius: 10px; /* Rounded corners */
		position: relative; /* Needed for positioning the close button */
	}

	.close {
		position: absolute;
		top: 10px; /* Adjust as needed */
		right: 10px; /* Adjust as needed */
		cursor: pointer; /* Change cursor to pointer on hover */
		background-color: #ff5f5f; /* Different color for the close button */
		color: #fff;
		border: none; /* Remove border */
		border-radius: 50%; /* Circular button */
		width: 30px; /* Adjust size as needed */
		height: 30px; /* Adjust size as needed */
		display: flex;
		justify-content: center;
		align-items: center;
		font-size: 20px; /* Adjust font size as needed */
	}
</style>
