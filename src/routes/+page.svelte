<script>
	import JvmProcessList from '$lib/jvm-process-list.svelte';
	import JvmProcess from '$lib/jvm-process.svelte';
	import { Styles } from 'sveltestrap';

	let processId;
	let showProgressSpinner;
	let error;
</script>

<Styles />

<div class="main-page">
	<JvmProcessList bind:error bind:processId bind:showProgressSpinner />
	<JvmProcess bind:error bind:processId bind:showProgressSpinner />
</div>

<style>
	.main-page {
		display: flex;
		flex-direction: column;
		height: 100vh;
	}

	.main-page :global(.jvm-process-list) {
		margin-left: 10px;
		margin-top: 10px;
		margin-right: 10px;
	}
</style>
