<script>
    import {invoke} from '@tauri-apps/api/tauri'
    import {onMount} from "svelte";
    import {Dropdown, DropdownItem, DropdownMenu, DropdownToggle, Icon, Spinner} from "sveltestrap";

    export let error = undefined;

    export let processId = undefined;
    export let showProgressSpinner = undefined;

    let selectedProcess
    let processes = [];

    async function getJvmProcesses() {
        let res = await invoke('get_jvm_processes').catch((e) => error = e);
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
        error = undefined;
        getJvmProcesses();
    }

    function selectedItem(item) {
        processId = item.id;
        selectedProcess = item;
        showProgressSpinner = true
        error = undefined;
        resetCache()
        return true
    }

    function resetCache() {
        invoke('reset');
    }

    let color = 'secondary';
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
    {#if showProgressSpinner !== undefined && showProgressSpinner === true}
        <Spinner class="progress-spinner align-middle" {color} size="sm" type="grow"/>
    {/if}
    {#if error !== undefined}
        <span class="process-error" title="{error}">
            <Icon name="exclamation-triangle" class="align-middle"/>
        </span>
    {/if}
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

    .jvm-process-list :global(.progress-spinner) {
        margin-left: 5px;
    }

    .jvm-process-list :global(.process-error) {
        margin-left: 5px;
    }
</style>