<script lang="ts">
	import { systemSnapshot } from '$lib/stores/system';
	import ProgressBar from './ProgressBar.svelte';
	import Skeleton from './Skeleton.svelte';
	import { getStatusColor, formatGiB } from '$lib/utils';
	import { openDetachedWindow } from '$lib/windowManager';

	export let showDetachButton: boolean = true;

	$: disks = $systemSnapshot?.disks ?? [];
</script>

<div class="panel glass" role="region" aria-label="Disk usage">
	<div class="panel-header">
		<div class="panel-title">
			<svg class="panel-icon" viewBox="0 0 16 16" fill="none">
				<circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.2"/>
				<circle cx="8" cy="8" r="2" stroke="currentColor" stroke-width="1.2"/>
				<line x1="8" y1="2" x2="8" y2="5" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
			</svg>
			<h3>Disks</h3>
		</div>
		{#if showDetachButton}
			<button class="detach-btn" on:click={() => openDetachedWindow('disks')} title="Open in new window" aria-label="Open disks panel in new window">
				<svg viewBox="0 0 16 16" fill="none">
					<path d="M9 2h5v5M14 2L7 9M6 3H3a1 1 0 00-1 1v9a1 1 0 001 1h9a1 1 0 001-1v-3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
				</svg>
			</button>
		{/if}
	</div>

	{#if !$systemSnapshot}
		<div class="disk-list">
			{#each Array(2) as _}
				<div class="disk-item">
					<Skeleton width="40%" height="12px" />
					<Skeleton height="4px" borderRadius="2px" />
					<Skeleton width="70%" height="10px" />
				</div>
			{/each}
		</div>
	{:else}
		<div class="disk-list">
			{#each disks as disk}
				{@const percent = disk.total > 0 ? (disk.used / disk.total) * 100 : 0}
				<div class="disk-item">
					<div class="disk-header">
						<span class="disk-name">{disk.name || disk.mount_point}</span>
						<span class="disk-percent mono" style="color: {getStatusColor(percent)}">{percent.toFixed(0)}%</span>
					</div>
					<ProgressBar value={percent / 100} color={getStatusColor(percent)} label="Disk usage {percent.toFixed(0)}%" />
					<div class="disk-meta">
						<span class="meta-tag">{disk.filesystem}</span>
						<span class="meta-divider"></span>
						<span>{disk.mount_point}</span>
						<span class="meta-spacer"></span>
						<span class="mono">{formatGiB(disk.used)} / {formatGiB(disk.total)}</span>
					</div>
				</div>
			{/each}
			{#if disks.length === 0}
				<div class="empty">No disks detected</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	h3 {
		margin: 0;
		font-size: 13px;
		font-weight: 600;
		color: var(--text-primary);
	}
	.disk-list {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}
	.disk-item {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 6px 8px;
		border-radius: var(--radius-s);
		transition: background var(--duration-fast) var(--ease-out);
	}
	.disk-item:hover {
		background: var(--bg-table-hover);
	}
	.disk-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.disk-name {
		font-weight: 500;
		font-size: 12px;
		color: var(--text-primary);
	}
	.disk-percent {
		font-weight: 600;
		font-size: 12px;
	}
	.disk-meta {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 11px;
		color: var(--text-tertiary);
		flex-wrap: wrap;
		overflow: hidden;
	}
	.meta-tag {
		background: var(--bg-input);
		padding: 1px 5px;
		border-radius: 3px;
		font-size: 10px;
		font-weight: 500;
		color: var(--text-secondary);
	}
	.meta-divider {
		width: 1px;
		height: 10px;
		background: var(--border-subtle);
	}
	.meta-spacer {
		flex: 1;
	}
	.empty {
		font-size: 12px;
		color: var(--text-tertiary);
		text-align: center;
		padding: 20px;
	}
</style>
