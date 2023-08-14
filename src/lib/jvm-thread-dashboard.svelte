<script>
    import {Line} from "svelte-chartjs";

    import {
        CategoryScale,
        Chart as ChartJS,
        Filler,
        Legend,
        LinearScale,
        LineElement,
        PointElement,
        Title,
        Tooltip
    } from "chart.js";

    import {Button, ButtonGroup, Table} from "sveltestrap";

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

    export let threads = undefined;

    let chartData = undefined;
    let tableData = undefined;
    let chartActive = true;
    let tableActive = false;

    function toggleVisualization(event) {
        chartActive = !chartActive;
        tableActive = !tableActive;
    }

    function threadData(t) {
        threadInformation(t);
        return true;
    }

    function threadInformation(t) {
        if (t !== undefined) {
            if (t.jvmThreads !== undefined) {
                tableData = prepareTableData(t.jvmThreads)
            }
            console.log(t.threadCountJvm)
            if (t.threadCountJvm !== undefined && t.threadCountJvm.values !== undefined) {
                chartData = threadChart(t.threadCountJvm.values);
            }
        }
    }

    function median(arr) {
        const mid = Math.floor(arr.length / 2),
            nums = [...arr].sort((a, b) => a - b);
        return arr.length % 2 !== 0 ? nums[mid] : (nums[mid - 1] + nums[mid]) / 2;
    }

    function avg(arr) {
        return arr.reduce((p, c) => p + c, 0) / arr.length;
    }

    function prepareTableData(values) {
        if (values !== undefined) {
            // Sort data for the table below the chart
            return values.sort((a, b) => a.status.localeCompare(b.status) || a.name.localeCompare(b.name) || b.elapsed - a.elapsed);
        }
        return undefined;
    }

    function threadChart(values) {
        if (values !== undefined) {
            // Prepare chart data
            let newCountValues = [];
            let runnableCountValues = [];
            let waitingCountValues = [];
            let timedWaitingCountValues = [];
            let blockedCountValues = [];
            let totalCountValues = [];
            let labels = [];

            for (const v of values) {
                let d = new Date(0)
                d.setUTCMilliseconds(v.time);
                labels.push(d.toLocaleTimeString());

                newCountValues.push(v.newCount);
                runnableCountValues.push(v.runnableCount);
                waitingCountValues.push(v.waitingCount);
                timedWaitingCountValues.push(v.timedWaitingCount);
                blockedCountValues.push(v.blockedCount);
                totalCountValues.push(v.newCount + v.runnableCount + v.waitingCount + v.timedWaitingCount + v.blockedCount)
            }

            let newCountMin = Math.min(...newCountValues);
            let newCountMax = Math.max(...newCountValues);
            let newCountAvg = avg(newCountValues);
            let newCountMedian = median(newCountValues);

            let runnableCountMin = Math.min(...runnableCountValues);
            let runnableCountMax = Math.max(...runnableCountValues);
            let runnableCountAvg = avg(runnableCountValues);
            let runnableCountMedian = median(runnableCountValues);

            let waitingCountMin = Math.min(...waitingCountValues);
            let waitingCountMax = Math.max(...waitingCountValues);
            let waitingCountAvg = avg(waitingCountValues);
            let waitingCountMedian = median(waitingCountValues);

            let timedWaitingCountMin = Math.min(...timedWaitingCountValues);
            let timedWaitingCountMax = Math.max(...timedWaitingCountValues);
            let timedWaitingCountAvg = avg(timedWaitingCountValues);
            let timedWaitingCountMedian = median(timedWaitingCountValues);

            let blockedCountMin = Math.min(...blockedCountValues);
            let blockedCountMax = Math.max(...blockedCountValues);
            let blockedCountAvg = avg(blockedCountValues);
            let blockedCountMedian = median(blockedCountValues);

            let totalCountMin = Math.min(...totalCountValues);
            let totalCountMax = Math.max(...totalCountValues);
            let totalCountAvg = avg(totalCountValues);
            let totalCountMedian = median(totalCountValues);

            let newDataset = newDataSet("New", newCountValues, newCountMin, newCountMax, newCountAvg, newCountMedian, 'rgba(212,12,243,0.75)', 'rgba(212,12,243,0.1)', "+1");
            let runnableDataset = newDataSet("Runnable", runnableCountValues, runnableCountMin, runnableCountMax, runnableCountAvg, runnableCountMedian, 'rgba(158,243,12,0.75)', 'rgba(158,243,12,0.1)', true);
            let waitingDataset = newDataSet("Waiting", waitingCountValues, waitingCountMin, waitingCountMax, waitingCountAvg, waitingCountMedian, 'rgba(12,162,243,0.75)', 'rgba(12,162,243,0.1)', true);
            let timedWaitingDataset = newDataSet("Timed Waiting", timedWaitingCountValues, timedWaitingCountMin, timedWaitingCountMax, timedWaitingCountAvg, timedWaitingCountMedian, 'rgba(243,224,12,0.75)', 'rgba(243,224,12,0.1)', true);
            let blockedDataset = newDataSet("Blocked", blockedCountValues, blockedCountMin, blockedCountMax, blockedCountAvg, blockedCountMedian, 'rgba(243,101,12,0.75)', 'rgba(243,101,12,0.1)', true);
            let totalDataset = newDataSet("Total", totalCountValues, totalCountMin, totalCountMax, totalCountAvg, totalCountMedian, 'rgba(123,123,123,0.75)', 'rgba(123,123,123,0.05)', true);
            let datasets = [newDataset, runnableDataset, waitingDataset, timedWaitingDataset, blockedDataset, totalDataset]

            return {
                labels: labels,
                title: "Number of threads by status",
                datasets: datasets
            }
        }
        return undefined
    }

    $: total = threadData(threads);

    function newDataSet(label, data, min, max, avg, median, borderColor, backgroundColor, fill) {
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
            tension: 0.1,
            // Custom values
            min: min,
            max: max,
            avg: avg,
            median: median
        }
    }
</script>

<div class="thread-dashboard">
    <ButtonGroup class="thread-visualization-selection">
        <Button outline color="primary" active={chartActive} on:click={toggleVisualization}>Chart</Button>
        <Button outline color="primary" active="{tableActive}" on:click={toggleVisualization}>Table</Button>
    </ButtonGroup>
    {#if chartActive && chartData !== undefined}
        <div class="chart">
            <Line data={chartData} class="metric_chart" options={{
                    responsive: true,
                    maintainAspectRatio: false,
                    animation: { duration: 0 },
                    plugins: {
                        legend: {
                            display: true,
                            title: {
                                display: true,
                                text: chartData.title,
                                font: {
                                    size: 14,
                                    weight: 'bold'
                                }
                            }
                        }
                    },
                    scale: { ticks: { precision: 1, beginAtZero: true } }}}/>
        </div>
        <div class="chart-values figure-caption">
            <div class="chart-values-types">
                <br/>
                New:<br/>
                Runnable:<br/>
                Waiting:<br/>
                Timed Waiting:<br/>
                Blocked:<br/>
                Total:<br/>
            </div>
            <div class="chart-values-column">
                Min:<br/>
                {chartData.datasets[0].min.toFixed(0)}<br/>
                {chartData.datasets[1].min.toFixed(0)}<br/>
                {chartData.datasets[2].min.toFixed(0)}<br/>
                {chartData.datasets[3].min.toFixed(0)}<br/>
                {chartData.datasets[4].min.toFixed(0)}<br/>
                {chartData.datasets[5].min.toFixed(0)}
            </div>
            <div class="chart-values-column">
                Max:<br/>
                {chartData.datasets[0].max.toFixed(0)}<br/>
                {chartData.datasets[1].max.toFixed(0)}<br/>
                {chartData.datasets[2].max.toFixed(0)}<br/>
                {chartData.datasets[3].max.toFixed(0)}<br/>
                {chartData.datasets[4].max.toFixed(0)}<br/>
                {chartData.datasets[5].max.toFixed(0)}
            </div>
            <div class="chart-values-column">
                Avg:<br/>
                {chartData.datasets[0].avg.toFixed(0)}<br/>
                {chartData.datasets[1].avg.toFixed(0)}<br/>
                {chartData.datasets[2].avg.toFixed(0)}<br/>
                {chartData.datasets[3].avg.toFixed(0)}<br/>
                {chartData.datasets[4].avg.toFixed(0)}<br/>
                {chartData.datasets[5].avg.toFixed(0)}
            </div>
            <div class="chart-values-column">
                Median:<br/>
                {chartData.datasets[0].median.toFixed(0)}<br/>
                {chartData.datasets[1].median.toFixed(0)}<br/>
                {chartData.datasets[2].median.toFixed(0)}<br/>
                {chartData.datasets[3].median.toFixed(0)}<br/>
                {chartData.datasets[4].median.toFixed(0)}<br/>
                {chartData.datasets[5].median.toFixed(0)}
            </div>
        </div>
    {/if}
    {#if tableActive && tableData !== undefined}
        <div class="thread-list">
            <Table borderless hover>
                <thead>
                <tr>
                    <!--th>#</th-->
                    <th>Name</th>
                    <th>CPU</th>
                    <th>Elapsed</th>
                    <th>Thread ID</th>
                    <th>OS Thread ID</th>
                    <th>OS Thread Prio</th>
                    <th>State</th>
                </tr>
                </thead>
                <tbody>
                {#each tableData as v}
                    <tr>
                        <!--th scope="row">{v.id}</th-->
                        <td>{v.name}</td>
                        <td>{v.cpu.toFixed(2)}ms</td>
                        <td>{(v.elapsed / 1000).toFixed(2)}s</td>
                        <td>{v.threadId}</td>
                        <td>{v.osThreadId}</td>
                        <td>{v.osThreadPrio}</td>
                        <td>{v.status}</td>
                    </tr>
                {/each}
                </tbody>
            </Table>
        </div>
    {/if}
    {#if tableData === undefined && chartData === undefined}
        No data available
    {/if}
</div>

<style>
    .thread-dashboard {
        display: flex;
        flex-direction: column;
        flex-grow: 1;
    }

    .thread-dashboard :global(.thread-visualization-selection) {
        width: 180px;
    }

    .thread-dashboard :global(.metric_chart) {
        padding-left: 5px;
        padding-top: 5px;
        padding-right: 5px;
    }

    .chart {
        height: calc(100vh - 31px - 42px - 38px - 167px - 100px);
        min-height: 250px;
    }

    .chart-values {
        display: flex;
        padding-left: 5px;
        padding-bottom: 20px;
        padding-right: 5px;
    }

    .chart-values-types {
        margin-left: 50px;
    }

    .chart-values-column {
        float: left;
        margin-left: 30px;
    }

    .thread-list {
        float: left;
    }
</style>