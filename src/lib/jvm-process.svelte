<script>
    import {invoke} from "@tauri-apps/api/tauri";
    import {TabContent, TabPane} from "sveltestrap";
    import {onDestroy, onMount} from "svelte";
    import ProcessInformation from "$lib/process-information.svelte";
    import MetricsDashboard from "$lib/metrics-dashboard.svelte";

    export let processId = undefined;

    let metrics
    let vmInformation

    const refreshMetrics = () => getJvmMetrics()
    const ms_5000 = 5000
    const ms_10000 = 10000
    let visibleTab = 'processInformation';

    let clearMetrics
    let clearVmInfo

    $: {
        clearInterval(clearMetrics)
        clearInterval(clearVmInfo)
        clearMetrics = setInterval(refreshMetrics, ms_5000)
        clearVmInfo = setInterval(getVmInformation, ms_10000)
    }

    onMount(() => {
        getVmInformation();
        refreshMetrics();
    })
    onDestroy(() => {
        clearInterval(clearVmInfo);
        clearInterval(clearMetrics);
    });

    async function getJvmMetrics() {
        let pid = await processId;
        if (pid !== undefined) {
            metrics = await invoke('get_jvm_metrics', {pid});
        } else {
            metrics = undefined;
        }
    }

    async function getVmInformation() {
        let pid = await processId;
        if (pid !== undefined) {
            vmInformation = await invoke('get_vm_information', {pid});
        } else {
            vmInformation = undefined;
        }
    }

</script>

<div class="tab-bar">
    <TabContent on:tab={(e) => (visibleTab = e.detail)}>
        <TabPane tabId="processInformation" tab="Process Information" active>
            <ProcessInformation bind:vmInformation/>
        </TabPane>
        <TabPane tabId="memory" tab="Memory">
            <MetricsDashboard bind:metrics/>
        </TabPane>
    </TabContent>
</div>

<style>
    .tab-bar :global(.nav-tabs) {
        padding-left: 10px;
    }

    .tab-bar :global(.process-information) {
        padding: 10px;
    }
</style>