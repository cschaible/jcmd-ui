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
        if (m !== undefined) {
            charts = [];

            let total = reservedCommittedMemoryChart(m.totalMemory.values, "Total");
            if (total != null) {
                charts.concat(total);
            }
            let clazz = reservedCommittedMemoryChart(m.class.values, "Class");
            if (clazz != null) {
                charts.concat(clazz);
            }
            let thread = reservedCommittedMemoryChart(m.thread.values, "Thread");
            if (thread != null) {
                charts.concat(thread);
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

            let reservedDataset = newDataSet(type + " - Reserved Memory (" + unit + ")", reserved, 'yellow');
            let committedDataset = newDataSet(type + " - Committed Memory(" + unit + ")", committed, 'green');

            return {
                labels: labels,
                datasets: [reservedDataset, committedDataset]
            };
        }
        return null
    }

    $: total = totalMemory(metrics);

    function newDataSet(label, data, color) {
        return {
            label: label,
            data: data,
            fill: false,
            borderColor: color,
            tension: 0.1
        }
    }

</script>

<div class="columns">
    <!--div class="column">
        <Line data={total} class="metric_chart" options={{ responsive: true, animation: { duration: 0 },
            scale: { ticks: { precision: 1 } }}}/>
    </div>
    <div class="column">
        <Line data={classes} class="metric_chart" options={{ responsive: true, animation: { duration: 0 },
            scale: { ticks: { precision: 1 } }}}/>
    </div>
    <div class="column">
        <Line data={thread} class="metric_chart" options={{ responsive: true, animation: { duration: 0 },
            scale: { ticks: { precision: 1 } }}}/>
    </div-->
    {#each charts as m}
        <div class="column">
            <Line data={m} class="metric_chart" options={{ responsive: true, animation: { duration: 0 },
                scale: { ticks: { precision: 1 } }}}/>
        </div>
    {/each}
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

</style>