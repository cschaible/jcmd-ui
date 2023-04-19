<script>
    import {Line} from "svelte-chartjs";

    import {
        CategoryScale,
        Chart as ChartJS, Filler,
        Legend,
        LinearScale,
        LineElement,
        PointElement,
        Title,
        Tooltip
    } from "chart.js";

    ChartJS.register(
        Filler,
        Title,
        Tooltip,
        Legend,
        LineElement,
        LinearScale,
        PointElement,
        CategoryScale
    );

    export let metrics = undefined;

    let charts = [];

    function totalMemory(m) {
        otherMemory(m);
        return true;
    }

    function otherMemory(m) {
        if (m !== undefined && m.totalMemory !== undefined && m.totalMemory.values !== undefined) {
            charts = [];

            let total = reservedCommittedMemoryChart(m.totalMemory.values, "Total");
            if (total != null) {
                charts = charts.concat(total);
            }
            let clazz = reservedCommittedMemoryChart(m.class.values, "Class");
            if (clazz != null) {
                charts = charts.concat(clazz);
            }
            let heap = reservedCommittedMemoryChart(m.heap.values, "Heap");
            if (heap != null) {
                charts = charts.concat(heap);
            }
            let metaspace = reservedCommittedMemoryChart(m.metaspace.values, "Metaspace");
            if (metaspace != null) {
                charts = charts.concat(metaspace);
            }
            let thread = reservedCommittedMemoryChart(m.thread.values, "Thread");
            if (thread != null) {
                charts = charts.concat(thread);
            }
            for (let t of m.other) {
                let o = reservedCommittedMemoryChart(t.values, t.name);
                if (o != null) {
                    charts = charts.concat(o);
                }
            }

            charts = charts.sort(function (a, b) {
                let x = arrayMaxCommitted(a.datasets[1].data);
                let xUnitMultiplicator = getChartUnitMultiplicator(a.datasets[1].label)
                let y = arrayMaxCommitted(b.datasets[1].data);
                let yUnitMultiplicator = getChartUnitMultiplicator(b.datasets[1].label)
                return (y * yUnitMultiplicator) - (x * xUnitMultiplicator);
            });
        }
        return undefined;
    }

    function getChartUnitMultiplicator(value) {
        if (value.includes("(GB)")) {
            return 1073741824;
        } else if (value.includes("(MB)")) {
            return 1048576;
        } else if (value.includes("KB")) {
            return 1024
        } else {
            return 1;
        }
    }

    function arrayMaxCommitted(arr) {
        return Math.max(...arr);
    }

    function reservedCommittedMemoryChart(values, type) {
        if (values !== undefined) {

            let min = Math.pow(2, 63) - 1;
            for (const v of values) {
                if (v.reserved < min) {
                    min = v.reserved
                }
                if (v.committed < min) {
                    min = v.committed
                }
                if (v.used !== undefined && v.used < min) {
                    min = v.used
                }
            }

            let divisor = 1;
            let unit = "B";
            if (min > 1073741824) {
                divisor = 1073741824
                unit = "GB";
            } else if (min > 1048576) {
                divisor = 1048576
                unit = "MB";
            } else if (min > 1024) {
                divisor = 1024;
                unit = "KB";
            }

            if (unit === "B" || unit === "KB") {
                return null;
            }

            let labels = [];
            let reserved = [];
            let committed = [];
            let used = [];
            for (const v of values) {
                let d = new Date(0)
                d.setUTCMilliseconds(v.time);
                labels.push(d.toLocaleTimeString());
                reserved.push(v.reserved / divisor);
                committed.push(v.committed / divisor);
                if (v.used !== undefined) {
                    used.push(v.used / divisor);
                }
            }

            let datasets;
            if (used.length > 0) {
                let reservedDataset = newDataSet("Reserved (" + unit + ")", reserved, 'rgba(123,123,123,0.75)', 'rgba(123,123,123,0.05)', "+1");
                let committedDataset = newDataSet("Committed (" + unit + ")", committed, 'rgba(243,101,12,0.75)', 'rgba(243,101,12,0.1)', "+1");
                let usedDataset = newDataSet(" Used (" + unit + ")", used, 'rgba(125,176,227,0.75)', 'rgba(125,176,227,0.35)', true);
                datasets = [reservedDataset, committedDataset, usedDataset];
            } else {
                let reservedDataset = newDataSet("Reserved (" + unit + ")", reserved, 'rgba(123,123,123,0.75)', 'rgba(123,123,123,0.05)', "+1");
                let committedDataset = newDataSet("Committed (" + unit + ")", committed, 'rgba(243,101,12,0.75)', 'rgba(243,101,12,0.1)', true);
                datasets = [reservedDataset, committedDataset]
            }

            return {
                labels: labels,
                title: type,
                datasets: datasets
            };
        }
        return null
    }

    $: total = totalMemory(metrics);

    function newDataSet(label, data, borderColor, backgroundColor, fill) {
        let pointRadius
        if (data.length <= 25) {
            pointRadius = 2;
        } else {
            pointRadius = 1;
        }
        return {
            label: label,
            data: data,
            fill: fill,
            backgroundColor: backgroundColor,
            borderColor: borderColor,
            pointHoverRadius: 5,
            pointHoverBackgroundColor: "rgb(0, 0, 0)",
            pointHoverBorderColor: "rgba(220, 220, 220, 1)",
            pointHoverBorderWidth: 1,
            pointHitRadius: 10,
            pointRadius: pointRadius,
            tension: 0.1
        }
    }

</script>

<div class="memory-dashboard">
    <div class="columns">
        {#each charts as m}
            <div class="column">
                <Line data={m} class="metric_chart" options={{
                responsive: true,
                maintainAspectRatio: false,
                animation: { duration: 0 },
                plugins: {
                    legend: {
                        display: true,
                        title: {
                            display: true,
                            text: m.title,
                            font: {
                                size: 14,
                                weight: 'bold'
                            }
                        }
                    }
                },
                scale: { ticks: { precision: 1, beginAtZero: true } }}}/>
            </div>
        {/each}
        {#if !(Array.isArray(charts) && charts.length > 0)}
            No data to available
        {/if}
    </div>
</div>


<style>
    .columns {
        display: flex;
        flex-direction: row;
        flex-wrap: wrap;
    }

    .column {
        width: calc(100% / 2);
        min-height: 300px;
    }

    .column :global(.metric_chart) {
        padding: 5px;
    }
</style>