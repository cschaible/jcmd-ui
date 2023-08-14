<script>
    import {invoke} from "@tauri-apps/api/tauri";
    import {TabContent, TabPane} from "sveltestrap";
    import {onDestroy, onMount} from "svelte";
    import ProcessInformation from "$lib/process-information.svelte";
    import MetricsDashboard from "$lib/metrics-dashboard.svelte";
    import ApplicationThreadDashboard from "$lib/application-thread-dashboard.svelte";
    import JvmThreadDashboard from "$lib/jvm-thread-dashboard.svelte";

    export let error = undefined;

    export let processId = undefined;

    export let showProgressSpinner;

    let metrics
    let threads
    let vmInformation

    const refreshMetrics = () => getJvmMetrics()
    const refreshThreads = () => getThreads()
    const ms_2000 = 2000
    const ms_5000 = 5000
    const ms_10000 = 10000
    let visibleTab = 'processInformation';

    let clearMetrics
    let clearThreads;
    let clearVmInfo

    $: {
        clearInterval(clearMetrics)
        clearInterval(clearVmInfo)
        clearMetrics = setInterval(refreshMetrics, ms_5000)
        clearThreads = setInterval(refreshThreads, ms_2000);
        clearVmInfo = setInterval(getVmInformation, ms_10000)
    }

    onMount(() => {
        getVmInformation();
        getThreads();
        refreshMetrics();
    })
    onDestroy(() => {
        clearInterval(clearVmInfo);
        clearInterval(clearThreads);
        clearInterval(clearMetrics);
    });

    async function getJvmMetrics() {
        let pid = await processId;
        if (pid !== undefined) {
            metrics = await invoke('get_jvm_metrics', {pid}).catch((e) => error = e);
        } else {
            if (metrics !== undefined) {
                error = "The connection to the process has been stopped";
            }
            // Do not unset metrics to keep them visible when the connection was interrupted
        }
    }

    async function getVmInformation() {
        let pid = await processId;
        if (pid !== undefined) {
            vmInformation = await invoke('get_vm_information', {pid}).catch((e) => error = e);
            showProgressSpinner = false;
        } else {
            vmInformation = undefined;
            // Unset metrics to remove metrics if a new process was selected which doesn't
            // have native memory tracking enabled.
            metrics = undefined;
        }
    }

    async function getThreads() {
        let pid = await processId;
        if (pid !== undefined) {
            threads = await invoke('get_threads', {pid}).catch((e) => error = e);
        } else {
            if (threads !== undefined) {
                error = "The connection to the process has been stopped";
                // Do not unset threads to keep them visible when the connection was interrupted
            }
        }
    }

</script>

<div class="tab-bar">
    <TabContent on:tab={(e) => (visibleTab = e.detail)}>
        <TabPane tabId="processInformation" tab="Process Information" active>
            <div class="tab-pane-content">
                <ProcessInformation bind:vmInformation/>
            </div>
        </TabPane>
        <TabPane tabId="memory" tab="Memory">
            <div class="tab-pane-content">
                <MetricsDashboard bind:metrics/>
            </div>
        </TabPane>
        <TabPane tabId="applicationThreads" tab="Application Threads">
            <div class="tab-pane-content">
                <ApplicationThreadDashboard bind:threads/>
            </div>
        </TabPane>
        <TabPane tabId="jvmThreads" tab="JVM Threads">
            <div class="tab-pane-content">
                <JvmThreadDashboard bind:threads/>
            </div>
        </TabPane>
    </TabContent>
</div>

<style>
    .tab-bar :global(.tab-pane) {
        height: calc(100%);
    }

    .tab-bar :global(.tab-content) {
        height: calc(100%);
    }

    .tab-bar :global(.nav-tabs) {
        padding-left: 10px;
    }

    .tab-bar :global(.process-information) {
        padding: 10px;
    }

    .tab-bar :global(.memory-dashboard) {
        padding: 10px;
    }

    .tab-bar :global(.thread-dashboard) {
        padding: 10px;
    }

    .tab-pane-content {
        display: flex;
        flex-direction: column;
        height: calc(100%);
    }
</style>