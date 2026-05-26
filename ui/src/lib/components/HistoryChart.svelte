<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Chart, registerables } from 'chart.js';

	export let data: [number, number][] = [];
	export let color: string = '#0a84ff';
	export let label: string = '';
	export let height: number = 100;

	let canvas: HTMLCanvasElement;
	let chart: Chart | null = null;

	Chart.register(...registerables);

	function hexToRgba(hex: string, alpha: number): string {
		const r = parseInt(hex.slice(1, 3), 16);
		const g = parseInt(hex.slice(3, 5), 16);
		const b = parseInt(hex.slice(5, 7), 16);
		return `rgba(${r}, ${g}, ${b}, ${alpha})`;
	}

	onMount(() => {
		const ctx = canvas.getContext('2d');
		if (!ctx) return;

		const gradient = ctx.createLinearGradient(0, 0, 0, height);
		gradient.addColorStop(0, hexToRgba(color, 0.25));
		gradient.addColorStop(0.7, hexToRgba(color, 0.05));
		gradient.addColorStop(1, hexToRgba(color, 0));

		chart = new Chart(canvas, {
			type: 'line',
			data: {
				labels: data.map((d) => d[0]),
				datasets: [
					{
						label,
						data: data.map((d) => d[1]),
						borderColor: color,
						backgroundColor: gradient,
						fill: true,
						tension: 0.4,
						pointRadius: 0,
						pointHoverRadius: 0,
						borderWidth: 1.5
					}
				]
			},
			options: {
				responsive: true,
				maintainAspectRatio: false,
				animation: false,
				plugins: {
					legend: { display: false },
					tooltip: { enabled: false }
				},
				scales: {
					x: {
						display: false,
						grid: { display: false }
					},
					y: {
						display: false,
						beginAtZero: true,
						grid: { display: false }
					}
				},
				interaction: { intersect: false, mode: 'index' },
				elements: {
					line: {
						capBezierPoints: true
					}
				}
			}
		});
	});

	onDestroy(() => {
		chart?.destroy();
	});

	$: if (chart && data.length > 0 && !document.hidden) {
		// No throttle — chart.update('none') skips animation and is
		// cheap. The old `> 1000ms` strict-greater-than guard dropped
		// alternating updates at the 1s minimum refresh interval due
		// to timing jitter, causing visible chart stutter.
		chart.data.labels = data.map((d) => d[0]);
		chart.data.datasets[0].data = data.map((d) => d[1]);
		chart.update('none');
	}
</script>

<div class="chart-container" style="height: {height}px">
	<canvas bind:this={canvas}></canvas>
</div>

<style>
	.chart-container {
		width: 100%;
		position: relative;
		border-radius: var(--radius-s);
		overflow: hidden;
	}
</style>
