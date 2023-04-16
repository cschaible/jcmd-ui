<script>
    import JvmProcessList from "$lib/jvm-process-list.svelte";
    import JvmProcess from "$lib/jvm-process.svelte";
    import {Styles} from "sveltestrap";

    let processId;

</script>
<Styles/>

<div class="main-page">
    <JvmProcessList bind:processId/>
    <JvmProcess bind:processId/>
</div>

<style>
    .main-page :global(.jvm-process-list) {
        margin-left: 10px;
        margin-top: 10px;
        margin-right: 10px;
    }
</style>