<script lang="ts">
	import { systemSnapshot } from '$lib/stores/system';
	import Skeleton from './Skeleton.svelte';
	import { formatBytes, formatBytesPerSec } from '$lib/utils';
	import { openDetachedWindow } from '$lib/windowManager';

	export let showDetachButton: boolean = true;

	$: networks = $systemSnapshot?.networks ?? [];
</script>

<div class="panel glass" role="region" aria-label="Network activity">
	<div class="panel-header">
		<div class="panel-title">
			<svg class="panel-icon" viewBox="0 0 16 16" fill="none">
				<path d="M2 11 L5.5 5 L9 9 L14 3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
				<line x1="11" y1="3" x2="14" y2="3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
				<line x1="14" y1="3" x2="14" y2="6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
			</svg>
			<h3>Network</h3>
		</div>
		{#if showDetachButton}
			<button class="detach-btn" on:click={() => openDetachedWindow('network')} title="Open in new window" aria-label="Open network panel in new window">
				<svg viewBox="0 0 16 16" fill="none">
					<path d="M9 2h5v5M14 2L7 9M6 3H3a1 1 0 00-1 1v9a1 1 0 001 1h9a1 1 0 001-1v-3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
				</svg>
			</button>
		{/if}
	</div>

	{#if !$systemSnapshot}
		<div class="network-list">
			{#each Array(3) as _}
				<div class="net-item">
					<Skeleton width="30%" height="12px" />
					<Skeleton width="50%" height="10px" />
				</div>
			{/each}
		</div>
	{:else}
		<div class="network-list">
			{#each networks as net}
				<div class="net-item">
					<span class="net-name">{net.interface}</span>
					<div class="net-stats">
						<div class="stat">
							<svg class="arrow-icon rx" viewBox="0 0 10 10" fill="none">
								<path d="M5 2L5 8M5 8L2.5 5.5M5 8L7.5 5.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
							</svg>
							<span class="stat-value mono">{formatBytes(net.rx_bytes)}</span>
							{#if net.rx_rate > 0}
								<span class="stat-rate mono">{formatBytesPerSec(net.rx_rate)}</span>
							{/if}
						</div>
						<div class="stat">
							<svg class="arrow-icon tx" viewBox="0 0 10 10" fill="none">
								<path d="M5 8L5 2M5 2L2.5 4.5M5 2L7.5 4.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
							</svg>
							<span class="stat-value mono">{formatBytes(net.tx_bytes)}</span>
							{#if net.tx_rate > 0}
								<span class="stat-rate mono">{formatBytesPerSec(net.tx_rate)}</span>
							{/if}
						</div>
					</div>
				</div>
			{/each}
			{#if networks.length === 0}
				<div class="empty">No active interfaces</div>
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
	.network-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.net-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px 10px;
		background: var(--bg-table-alt);
		border-radius: var(--radius-s);
		transition: background var(--duration-fast) var(--ease-out);
	}
	.net-item:hover {
		background: var(--bg-table-hover);
	}
	.net-name {
		font-weight: 500;
		font-size: 12px;
		color: var(--text-primary);
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		flex: 1;
	}
	.net-stats {
		display: flex;
		gap: 14px;
		flex-shrink: 0;
	}
	.stat {
		display: flex;
		align-items: center;
		gap: 4px;
	}
	.arrow-icon {
		width: 11px;
		height: 11px;
	}
	.arrow-icon.rx {
		color: var(--green);
	}
	.arrow-icon.tx {
		color: var(--accent);
	}
	.stat-value {
		font-size: 11px;
		color: var(--text-secondary);
	}
	.stat-rate {
		font-size: 10px;
		color: var(--text-tertiary);
		padding: 1px 4px;
		background: var(--bg-input);
		border-radius: 3px;
	}
	.empty {
		font-size: 12px;
		color: var(--text-tertiary);
		text-align: center;
		padding: 20px;
	}
</style>
