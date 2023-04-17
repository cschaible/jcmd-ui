<script>
    import {Line} from "svelte-chartjs";

    import {
        CategoryScale,
        Chart as ChartJS,
        Legend,
        LinearScale,
        LineElement,
        PointElement,
        Title,
        Tooltip
    } from "chart.js";

    ChartJS.register(
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
                let y = arrayMaxCommitted(b.datasets[1].data);
                return y - x;
            });
        }
        return undefined;
    }

    function arrayMaxCommitted(arr) {
        return Math.max(...arr);
    }

    function reservedCommittedMemoryChart(values, type) {
        if (values !== undefined) {
            let labels = [];
            let reserved = [];
            let committed = [];
            let min = Math.pow(2, 63) - 1;
            for (const v of values) {
                if (v.reserved < min) {
                    min = v.reserved
                }
                if (v.committed < min) {
                    min = v.committed
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

            for (const v of values) {
                let d = new Date(0)
                d.setUTCMilliseconds(v.time);
                labels.push(d.toLocaleTimeString());
                reserved.push(v.reserved / divisor);
                committed.push(v.committed / divisor);
            }

            let reservedDataset = newDataSet("Reserved (" + unit + ")", reserved, 'orange');
            let committedDataset = newDataSet("Committed (" + unit + ")", committed, 'green');

            return {
                labels: labels,
                title: type,
                datasets: [reservedDataset, committedDataset]
            };
        }
        return null
    }

    $: total = totalMemory(metrics);

    function newDataSet(label, data, color) {
        let pointRadius
        if (data.length <= 25) {
            pointRadius = 3;
        } else {
            pointRadius = 2;
        }
        return {
            label: label,
            data: data,
            fill: true,
            backgroundColor: color,
            borderColor: color,
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
                scale: { ticks: { precision: 1 } }}}/>
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