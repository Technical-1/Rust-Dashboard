<script lang="ts">
	import { systemSnapshot, memoryHistory } from '$lib/stores/system';
	import StatusIndicator from './StatusIndicator.svelte';
	import ProgressBar from './ProgressBar.svelte';
	import HistoryChart from './HistoryChart.svelte';
	import Skeleton from './Skeleton.svelte';
	import { getStatusColor } from '$lib/utils';
	import { openDetachedWindow } from '$lib/windowManager';

	export let showDetachButton: boolean = true;

	$: mem = $systemSnapshot?.memory;
	$: usedGb = mem ? mem.used / 1024 / 1024 / 1024 : 0;
	$: totalGb = mem ? mem.total / 1024 / 1024 / 1024 : 0;
	$: freeGb = mem ? mem.free / 1024 / 1024 / 1024 : 0;
	$: availableGb = mem ? mem.available / 1024 / 1024 / 1024 : 0;
	$: percent = totalGb > 0 ? (usedGb / totalGb) * 100 : 0;
	$: swapUsedGb = mem ? mem.swap_used / 1024 / 1024 / 1024 : 0;
	$: swapTotalGb = mem ? mem.swap_total / 1024 / 1024 / 1024 : 0;

	// Memory breakdown: App (used - cached), Cached (total - available - free roughly), Free
	$: cachedGb = mem ? Math.max(0, totalGb - availableGb - (totalGb - availableGb - freeGb > 0 ? 0 : freeGb)) : 0;
	$: appGb = mem ? Math.max(0, usedGb - cachedGb) : 0;
	$: appPercent = totalGb > 0 ? (appGb / totalGb) * 100 : 0;
	$: cachedPercent = totalGb > 0 ? (cachedGb / totalGb) * 100 : 0;
	$: freePercent = totalGb > 0 ? (freeGb / totalGb) * 100 : 0;
</script>

<div class="panel glass" role="region" aria-label="Memory usage">
	<div class="panel-header">
		<div class="panel-title">
			<svg class="panel-icon" viewBox="0 0 16 16" fill="none">
				<rect x="3" y="1.5" width="10" height="13" rx="1.5" stroke="currentColor" stroke-width="1.2"/>
				<line x1="5.5" y1="4.5" x2="10.5" y2="4.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
				<line x1="5.5" y1="7" x2="10.5" y2="7" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
				<line x1="5.5" y1="9.5" x2="10.5" y2="9.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
			</svg>
			<h3>Memory</h3>
		</div>
		<div class="header-right">
			{#if $systemSnapshot}
				<StatusIndicator value={percent} />
				<span class="value mono" style="color: {getStatusColor(percent)}">{usedGb.toFixed(1)}<span class="unit"> / {totalGb.toFixed(1)} GB</span></span>
			{/if}
			{#if showDetachButton}
				<button class="detach-btn" on:click={() => openDetachedWindow('memory')} title="Open in new window" aria-label="Open memory panel in new window">
					<svg viewBox="0 0 16 16" fill="none">
						<path d="M9 2h5v5M14 2L7 9M6 3H3a1 1 0 00-1 1v9a1 1 0 001 1h9a1 1 0 001-1v-3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
					</svg>
				</button>
			{/if}
		</div>
	</div>

	{#if !$systemSnapshot}
		<Skeleton height="4px" borderRadius="2px" />
		<Skeleton height="24px" borderRadius="4px" />
		<Skeleton height="90px" borderRadius="6px" />
	{:else}
		<ProgressBar value={percent / 100} color={getStatusColor(percent)} label="Memory usage {percent.toFixed(0)}%" />

		<!-- Memory breakdown bar -->
		<div class="breakdown-bar" role="img" aria-label="Memory breakdown: {appGb.toFixed(1)} GB app, {cachedGb.toFixed(1)} GB cached, {freeGb.toFixed(1)} GB free">
			<div class="segment app" style="width: {appPercent}%" title="App Memory: {appGb.toFixed(1)} GB"></div>
			<div class="segment cached" style="width: {cachedPercent}%" title="Cached: {cachedGb.toFixed(1)} GB"></div>
			<div class="segment free" style="width: {freePercent}%" title="Free: {freeGb.toFixed(1)} GB"></div>
		</div>
		<div class="breakdown-legend">
			<span class="legend-item"><span class="legend-dot app"></span> App {appGb.toFixed(1)}G</span>
			<span class="legend-item"><span class="legend-dot cached"></span> Cached {cachedGb.toFixed(1)}G</span>
			<span class="legend-item"><span class="legend-dot free"></span> Free {freeGb.toFixed(1)}G</span>
		</div>

		<div class="swap-row">
			<span class="swap-label">Swap</span>
			<span class="swap-value mono">{swapUsedGb.toFixed(2)} / {swapTotalGb.toFixed(2)} GB</span>
		</div>

		<HistoryChart data={$memoryHistory} color="#30d158" label="Memory GiB" height={90} />
	{/if}
</div>

<style>
	.value {
		font-size: 18px;
		font-weight: 600;
		letter-spacing: -0.02em;
		white-space: nowrap;
	}
	.unit {
		font-size: 12px;
		font-weight: 500;
		color: var(--text-secondary);
	}
	.swap-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.swap-label {
		font-size: 11px;
		color: var(--text-tertiary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		font-weight: 500;
	}
	.swap-value {
		font-size: 11px;
		color: var(--text-secondary);
	}

	/* ─── Memory breakdown ─── */
	.breakdown-bar {
		display: flex;
		height: 8px;
		border-radius: 4px;
		overflow: hidden;
		background: var(--bg-progress);
		gap: 1px;
	}
	.segment {
		height: 100%;
		transition: width 0.6s var(--ease-out);
		min-width: 0;
	}
	.segment.app {
		background: var(--accent);
		border-radius: 4px 0 0 4px;
	}
	.segment.cached {
		background: var(--yellow);
	}
	.segment.free {
		background: var(--green);
		border-radius: 0 4px 4px 0;
	}
	.breakdown-legend {
		display: flex;
		gap: 12px;
		font-size: 10px;
		color: var(--text-tertiary);
	}
	.legend-item {
		display: flex;
		align-items: center;
		gap: 4px;
	}
	.legend-dot {
		width: 6px;
		height: 6px;
		border-radius: 2px;
		flex-shrink: 0;
	}
	.legend-dot.app {
		background: var(--accent);
	}
	.legend-dot.cached {
		background: var(--yellow);
	}
	.legend-dot.free {
		background: var(--green);
	}
</style>
