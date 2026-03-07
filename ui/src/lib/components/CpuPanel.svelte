<script lang="ts">
	import { systemSnapshot, cpuHistory } from '$lib/stores/system';
	import StatusIndicator from './StatusIndicator.svelte';
	import ProgressBar from './ProgressBar.svelte';
	import HistoryChart from './HistoryChart.svelte';
	import PerCpuCores from './PerCpuCores.svelte';
	import Skeleton from './Skeleton.svelte';
	import { getStatusColor } from '$lib/utils';
	import { openDetachedWindow } from '$lib/windowManager';

	export let showDetachButton: boolean = true;

	$: cpu = $systemSnapshot?.cpu_usage ?? 0;
	$: perCpu = $systemSnapshot?.per_cpu ?? [];
</script>

<div class="panel glass" role="region" aria-label="CPU usage">
	<div class="panel-header">
		<div class="panel-title">
			<svg class="panel-icon" viewBox="0 0 16 16" fill="none">
				<rect x="1.5" y="3" width="13" height="10" rx="2" stroke="currentColor" stroke-width="1.2"/>
				<line x1="4.5" y1="6" x2="4.5" y2="10" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
				<line x1="8" y1="5" x2="8" y2="10" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
				<line x1="11.5" y1="7" x2="11.5" y2="10" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
			</svg>
			<h3>CPU</h3>
		</div>
		<div class="header-right">
			{#if $systemSnapshot}
				<StatusIndicator value={cpu} />
				<span class="value mono" style="color: {getStatusColor(cpu)}">{cpu.toFixed(1)}%</span>
			{/if}
			{#if showDetachButton}
				<button class="detach-btn" on:click={() => openDetachedWindow('cpu')} title="Open in new window" aria-label="Open CPU panel in new window">
					<svg viewBox="0 0 16 16" fill="none">
						<path d="M9 2h5v5M14 2L7 9M6 3H3a1 1 0 00-1 1v9a1 1 0 001 1h9a1 1 0 001-1v-3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
					</svg>
				</button>
			{/if}
		</div>
	</div>

	{#if !$systemSnapshot}
		<Skeleton height="4px" borderRadius="2px" />
		<Skeleton height="90px" borderRadius="6px" />
		<Skeleton width="60%" height="10px" />
	{:else}
		<ProgressBar value={cpu / 100} color={getStatusColor(cpu)} label="CPU usage {cpu.toFixed(0)}%" />
		<HistoryChart data={$cpuHistory} color="#0a84ff" label="CPU %" height={90} />

		{#if perCpu.length > 0}
			<div class="section-divider"></div>
			<div class="section-label">Per-Core Usage</div>
			<PerCpuCores cores={perCpu} />
		{/if}
	{/if}
</div>

<style>
	.section-divider {
		height: 0.5px;
		background: var(--border-subtle);
	}
	.section-label {
		font-size: 11px;
		color: var(--text-tertiary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		font-weight: 500;
	}
	.value {
		font-size: 22px;
		font-weight: 600;
		letter-spacing: -0.02em;
	}
</style>
