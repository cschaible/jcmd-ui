<script>
    import {invoke} from '@tauri-apps/api/tauri'

    let processes = [];

    async function getJvmProcesses() {
        let res = await invoke('get_jvm_processes');
        processes = res.processes;
    }
</script>
<div>
    <button on:click={getJvmProcesses}>Load</button>
    {#each processes as process}
        <p>{process.id} {process.name} {process.path}</p>
    {/each}
</div>