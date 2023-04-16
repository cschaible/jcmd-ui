<script>
    import {invoke} from "@tauri-apps/api/tauri";

    export let processId = undefined;

    let metrics

    const refreshMetrics = () => getJvmProcesses()
    let ms = 5000

    let clear
    $: {
        clearInterval(clear)
        clear = setInterval(refreshMetrics, ms)
    }

    async function getJvmProcesses() {
        let pid = await processId;
        if (pid !== undefined) {
            metrics = await invoke('get_jvm_metrics', {pid});
        } else {
            metrics = undefined;
        }
    }

</script>
<div>
    {metrics}
</div>