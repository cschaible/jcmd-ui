<script>
    import {invoke} from '@tauri-apps/api/tauri'
    import {onMount} from "svelte";
    import {Dropdown, DropdownItem, DropdownMenu, DropdownToggle, Icon} from "sveltestrap";

    export let processId;

    let selectedProcess
    let processes = [];

    async function getJvmProcesses() {
        let res = await invoke('get_jvm_processes');
        processes = res.processes;
        processes = processes.sort(function (a, b) {
            return b.id - a.id;
        });
        selectedProcess = undefined;
    }

    onMount(async () => {
        await getJvmProcesses();
    })

    function reloadProcessed() {
        processId = undefined;
        getJvmProcesses();
    }

    function selectedItem(item) {
        processId = item.id;
        selectedProcess = item;
        resetCache()
        return true
    }

    function resetCache() {
        invoke('reset');
    }

</script>

<div class="jvm-process-list mb-3">
    <Dropdown group size="sm">
        <DropdownToggle caret>
            {#if selectedProcess === undefined}
                Select process
            {:else }
                <div class="dropDownItemSelected"
                     title="{selectedProcess.name} {selectedProcess.path}">{selectedProcess.id} {selectedProcess.name}</div>
            {/if}
        </DropdownToggle>
        <DropdownMenu>
            {#each processes as process, i}
                <DropdownItem on:click={selectedItem(process)}>
                    <div>PID: {process.id}</div>
                    <div class="dropDownItem" title="{process.name}">Name: {process.name}</div>
                    {#if process.path !== null}
                        <div class="dropDownItem" title="{process.path}">Params: {process.path}</div>
                    {/if}
                </DropdownItem>
                {#if i !== processes.length - 1}
                    <DropdownItem divider/>
                {/if}
            {/each}
        </DropdownMenu>
    </Dropdown>
    <span class="dropDownIcon" on:click={reloadProcessed}>
        <Icon name="arrow-clockwise" class="align-middle"/>
    </span>
</div>

<style>
    .dropDownIcon {
        margin-left: 5px !important;
    }

    .dropDownItem {
        max-width: 450px;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .dropDownItemSelected {
        max-width: 200px;
        overflow: hidden;
        text-overflow: ellipsis;
        float: left;
        margin-right: 6px;
    }
</style>