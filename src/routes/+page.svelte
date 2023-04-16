<script>
    import JvmProcessList from "$lib/jvm-process-list.svelte";
    import JvmMetrics from "$lib/jvm-metrics.svelte";
    import {Styles} from "sveltestrap";

    let processId;

</script>
<Styles/>


<div class="cbody">
    <JvmProcessList bind:processId/>
    <JvmMetrics bind:processId/>
</div>

<style>
    .cbody {
        margin: 10px;
    }
</style>