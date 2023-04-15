<script>
    import JvmProcessList from "$lib/jvm-process-list.svelte";

    let ms = 1000
    let counter = 0
    const incr = () => (counter += 1)
    const reset = () => (counter = 0)

    let clear
    $: {
        clearInterval(clear)
        clear = setInterval(incr, ms)
    }
</script>

<h1>Welcome to SvelteKit</h1>
<p>Visit <a href="https://kit.svelte.dev">kit.svelte.dev</a> to read the documentation</p>

<p>{counter}</p>
<input type="number" bind:value={ms} step="500" min="0" max="5000"/>
<button on:click="{reset}">Reset</button>
<JvmProcessList/>