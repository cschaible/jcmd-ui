<script>
    import {Spinner} from "sveltestrap";

    export let vmInformation = undefined;

    function jvmArgs() {
        return vmInformation.vmArguments.jvmArgs.split(" ");
    }

    let color = 'primary';
</script>

<div class="process-information">
    {#if vmInformation !== undefined}
        <p class="h6">VM Parameter</p>
        {#if vmInformation.vmArguments !== undefined}
            <div>JVM Arguments:</div>
            {#if jvmArgs().length > 1}
                {#each jvmArgs() as arg}
                    <div class="attribute-value-indent">{arg}</div>
                {/each}
            {:else}
                <div class="attribute-value-indent">{jvmArgs()}</div>
            {/if}
            <div class="attribute-name-margin">Java Command:</div>
            <div class="attribute-value-indent">{vmInformation.vmArguments.javaCommand}</div>
        {/if}
        {#if vmInformation.vmResources !== undefined}
            <div class="attribute-name-margin">Resources:</div>
            {#if vmInformation.vmResources.cpus !== undefined}
                <div class="attribute-value-indent">CPUs: {vmInformation.vmResources.cpus}</div>
            {/if}
            {#if vmInformation.vmResources.memory !== undefined}
                <div class="attribute-value-indent">Memory: {vmInformation.vmResources.memory}</div>
            {/if}
            {#if vmInformation.vmResources.heapSizeMin !== undefined}
                <div class="attribute-value-indent">Min Heap Size: {vmInformation.vmResources.heapSizeMin}</div>
            {/if}
            {#if vmInformation.vmResources.heapSizeInit !== undefined}
                <div class="attribute-value-indent">Init Heap Size: {vmInformation.vmResources.heapSizeInit}</div>
            {/if}
            {#if vmInformation.vmResources.heapSizeMax !== undefined}
                <div class="attribute-value-indent">Max Heap Size: {vmInformation.vmResources.heapSizeMax}</div>
            {/if}
        {/if}
    {:else }
        No process information available
        <Spinner {color} size="sm"/>
    {/if}

</div>

<style>
    .attribute-name-margin {
        margin-top: 5px;
    }

    .attribute-value-indent {
        margin-left: 20px;
    }
</style>