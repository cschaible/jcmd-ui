<script>
	import { Line } from 'svelte-chartjs';

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
	} from 'chart.js';

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

			let total = reservedCommittedMemoryChart(m.totalMemory.values, 'Total');
			if (total != null) {
				charts = charts.concat(total);
			}
			let clazz = reservedCommittedMemoryChart(m.class.values, 'Class');
			if (clazz != null) {
				charts = charts.concat(clazz);
			}
			let heap = reservedCommittedMemoryChart(m.heap.values, 'Heap');
			if (heap != null) {
				charts = charts.concat(heap);
			}
			let metaspace = reservedCommittedMemoryChart(m.metaspace.values, 'Metaspace');
			if (metaspace != null) {
				charts = charts.concat(metaspace);
			}
			let thread = reservedCommittedMemoryChart(m.thread.values, 'Thread');
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
				let x = arrayMax(a.datasets[1].data);
				let xUnitMultiplicator = getChartUnitMultiplicator(a.datasets[1].label);
				let y = arrayMax(b.datasets[1].data);
				let yUnitMultiplicator = getChartUnitMultiplicator(b.datasets[1].label);
				return y * yUnitMultiplicator - x * xUnitMultiplicator;
			});
		}
		return undefined;
	}

	function getChartUnitMultiplicator(value) {
		if (value.includes('(GB)')) {
			return 1073741824;
		} else if (value.includes('(MB)')) {
			return 1048576;
		} else if (value.includes('KB')) {
			return 1024;
		} else {
			return 1;
		}
	}

	function arrayMax(arr) {
		return Math.max(...arr);
	}

	function median(arr) {
		const mid = Math.floor(arr.length / 2),
			nums = [...arr].sort((a, b) => a - b);
		return arr.length % 2 !== 0 ? nums[mid] : (nums[mid - 1] + nums[mid]) / 2;
	}

	function avg(arr) {
		return arr.reduce((p, c) => p + c, 0) / arr.length;
	}

	function reservedCommittedMemoryChart(values, type) {
		if (values !== undefined) {
			let reservedValues = [];
			let committedValues = [];
			let usedValues = [];
			let usedAvailable = false;

			for (const v of values) {
				reservedValues.push(v.reserved);
				committedValues.push(v.committed);
				if (v.used !== undefined) {
					usedAvailable = true;
					usedValues.push(v.used);
				}
			}

			let reservedMin = Math.min(...reservedValues);
			let reservedMax = Math.max(...reservedValues);
			let reservedAvg = avg(reservedValues);
			let reservedMedian = median(reservedValues);
			let committedMin = Math.min(...committedValues);
			let committedMax = Math.max(...committedValues);
			let committedAvg = avg(committedValues);
			let committedMedian = median(committedValues);
			let usedMin;
			let usedMax;
			let usedAvg;
			let usedMedian;
			if (usedAvailable) {
				usedMin = Math.min(...usedValues);
				usedMax = Math.max(...usedValues);
				usedAvg = avg(usedValues);
				usedMedian = median(usedValues);
			}

			let min;
			if (usedAvailable) {
				min = Math.min(reservedMin, committedMin, usedMin);
			} else {
				min = Math.min(reservedMin, committedMin);
			}

			let divisor = 1;
			let unit = 'B';
			if (min > 1073741824) {
				divisor = 1073741824;
				unit = 'GB';
			} else if (min > 1048576) {
				divisor = 1048576;
				unit = 'MB';
			} else if (min > 1024) {
				divisor = 1024;
				unit = 'KB';
			}

			if (unit === 'B' || unit === 'KB') {
				return null;
			}

			let labels = [];
			let reserved = [];
			let committed = [];
			let used = [];
			for (const v of values) {
				let d = new Date(0);
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
				let reservedDataset = newDataSet(
					'Reserved (' + unit + ')',
					unit,
					reserved,
					reservedMin / divisor,
					reservedMax / divisor,
					reservedAvg / divisor,
					reservedMedian / divisor,
					'rgba(123,123,123,0.75)',
					'rgba(123,123,123,0.05)',
					'+1'
				);
				let committedDataset = newDataSet(
					'Committed (' + unit + ')',
					unit,
					committed,
					committedMin / divisor,
					committedMax / divisor,
					committedAvg / divisor,
					committedMedian / divisor,
					'rgba(243,101,12,0.75)',
					'rgba(243,101,12,0.1)',
					'+1'
				);
				let usedDataset = newDataSet(
					' Used (' + unit + ')',
					unit,
					used,
					usedMin / divisor,
					usedMax / divisor,
					usedAvg / divisor,
					usedMedian / divisor,
					'rgba(125,176,227,0.75)',
					'rgba(125,176,227,0.35)',
					true
				);
				datasets = [reservedDataset, committedDataset, usedDataset];
			} else {
				let reservedDataset = newDataSet(
					'Reserved (' + unit + ')',
					unit,
					reserved,
					reservedMin / divisor,
					reservedMax / divisor,
					reservedAvg / divisor,
					reservedMedian / divisor,
					'rgba(123,123,123,0.75)',
					'rgba(123,123,123,0.05)',
					'+1'
				);
				let committedDataset = newDataSet(
					'Committed (' + unit + ')',
					unit,
					committed,
					committedMin / divisor,
					committedMax / divisor,
					committedAvg / divisor,
					committedMedian / divisor,
					'rgba(243,101,12,0.75)',
					'rgba(243,101,12,0.1)',
					true
				);
				datasets = [reservedDataset, committedDataset];
			}

			return {
				labels: labels,
				title: type,
				datasets: datasets
			};
		}
		return null;
	}

	$: total = totalMemory(metrics);

	function newDataSet(
		label,
		unit,
		data,
		min,
		max,
		avg,
		median,
		borderColor,
		backgroundColor,
		fill
	) {
		let pointRadius;
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
			pointHoverBackgroundColor: 'rgb(0, 0, 0)',
			pointHoverBorderColor: 'rgba(220, 220, 220, 1)',
			pointHoverBorderWidth: 1,
			pointHitRadius: 10,
			pointRadius: pointRadius,
			tension: 0.1,
			// Custom values
			unit: unit,
			min: min,
			max: max,
			avg: avg,
			median: median
		};
	}
</script>

<div class="memory-dashboard">
	<div class="columns">
		{#each charts as m}
			<div class="column">
				<div class="chart">
					<Line
						data={m}
						class="metric_chart"
						options={{
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
							scale: { ticks: { precision: 1, beginAtZero: true } }
						}}
					/>
				</div>
				<div class="chart-values figure-caption">
					<div class="chart-values-types">
						<br />
						Reserved:<br />
						Committed
						{#if m.datasets.length === 3}
							<br />
							Used:
						{/if}
					</div>
					<div class="chart-values-column">
						Min:<br />
						{m.datasets[0].min.toFixed(2)}
						{m.datasets[0].unit}<br />
						{m.datasets[1].min.toFixed(2)}
						{m.datasets[1].unit}
						{#if m.datasets.length === 3}
							<br />
							{m.datasets[2].min.toFixed(2)}
							{m.datasets[2].unit}
						{/if}
					</div>
					<div class="chart-values-column">
						Max:<br />
						{m.datasets[0].max.toFixed(2)}
						{m.datasets[0].unit}<br />
						{m.datasets[1].max.toFixed(2)}
						{m.datasets[1].unit}
						{#if m.datasets.length === 3}
							<br />
							{m.datasets[2].max.toFixed(2)}
							{m.datasets[2].unit}
						{/if}
					</div>
					<div class="chart-values-column">
						Avg:<br />
						{m.datasets[0].avg.toFixed(2)}
						{m.datasets[0].unit}<br />
						{m.datasets[1].avg.toFixed(2)}
						{m.datasets[1].unit}
						{#if m.datasets.length === 3}
							<br />
							{m.datasets[2].avg.toFixed(2)}
							{m.datasets[2].unit}
						{/if}
					</div>
					<div class="chart-values-column">
						Median:<br />
						{m.datasets[0].median.toFixed(2)}
						{m.datasets[0].unit}<br />
						{m.datasets[1].median.toFixed(2)}
						{m.datasets[1].unit}
						{#if m.datasets.length === 3}
							<br />
							{m.datasets[2].median.toFixed(2)}
							{m.datasets[2].unit}
						{/if}
					</div>
				</div>
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
		min-height: 400px;
		display: flex;
		flex-direction: column;
	}

	.chart {
		flex-grow: 1;
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
		margin-left: 15px;
	}

	.column :global(.metric_chart) {
		padding-left: 5px;
		padding-top: 5px;
		padding-right: 5px;
	}
</style>
