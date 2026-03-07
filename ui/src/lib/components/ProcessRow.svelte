<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import type { CombinedProcess, ProcessDetails } from '$lib/types';
	import { logError } from '$lib/log';

	export let process: CombinedProcess;
	export let expanded: boolean = false;

	const dispatch = createEventDispatcher();
	let details: ProcessDetails | null = null;
	let loadingDetails = false;

	async function toggleExpand() {
		expanded = !expanded;
		dispatch('toggle', { name: process.name, expanded });

		if (expanded && !details && process.pids.length > 0) {
			loadingDetails = true;
			try {
				details = await invoke<ProcessDetails | null>('get_process_details', {
					pid: process.pids[0]
				});
			} catch (e) {
				logError('Failed to load process details', e);
			} finally {
				loadingDetails = false;
			}
		}
	}

	function requestKill(pid: number) {
		dispatch('kill', { name: process.name, pid });
	}
</script>

<tr class="process-row" class:expanded on:click={toggleExpand}>
	<td class="name-cell">
		<svg class="chevron" class:open={expanded} viewBox="0 0 8 8" fill="none">
			<path d="M2 1.5L5.5 4L2 6.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
		</svg>
		{process.name}
	</td>
	<td class="num-cell mono">{process.cpu_usage.toFixed(1)}%</td>
	<td class="num-cell mono">{Math.floor(process.memory_usage / 1024 / 1024)} MB</td>
	<td class="num-cell mono">{process.pids.length}</td>
	<td class="actions-cell">
		{#if process.pids.length > 0}
			<button
				class="kill-btn"
				on:click|stopPropagation={() => requestKill(process.pids[0])}
				title="Terminate {process.name}"
			>
				<svg viewBox="0 0 10 10" fill="none">
					<line x1="2.5" y1="2.5" x2="7.5" y2="7.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
					<line x1="7.5" y1="2.5" x2="2.5" y2="7.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
				</svg>
			</button>
		{/if}
	</td>
</tr>

{#if expanded}
	<tr class="details-row">
		<td colspan="5">
			{#if loadingDetails}
				<div class="details-content">
					<span class="loading">Loading details...</span>
				</div>
			{:else if details}
				<div class="details-content">
					<div class="detail-row">
						<span class="detail-label">Command</span>
						<span class="detail-value">{details.command || '(empty)'}</span>
					</div>
					<div class="detail-row">
						<span class="detail-label">Start Time</span>
						<span class="detail-value mono">{details.start_time}</span>
					</div>
					{#if details.parent}
						<div class="detail-row">
							<span class="detail-label">Parent PID</span>
							<span class="detail-value mono">{details.parent}</span>
						</div>
					{/if}
					<div class="detail-row">
						<span class="detail-label">PIDs</span>
						<span class="detail-value mono">{process.pids.join(', ')}</span>
					</div>
				</div>
			{:else}
				<div class="details-content">
					<span class="loading">No details available</span>
				</div>
			{/if}
		</td>
	</tr>
{/if}

<style>
	.process-row {
		cursor: pointer;
		transition: background var(--duration-fast) var(--ease-out);
	}
	.process-row:hover {
		background: var(--bg-table-hover);
	}
	.process-row.expanded {
		background: var(--bg-table-alt);
	}
	td {
		padding: 5px 12px;
		font-size: 12px;
		border-bottom: 0.5px solid var(--border-subtle);
		color: var(--text-primary);
	}
	.name-cell {
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.chevron {
		width: 8px;
		height: 8px;
		flex-shrink: 0;
		color: var(--text-tertiary);
		transition: transform var(--duration-fast) var(--ease-out);
	}
	.chevron.open {
		transform: rotate(90deg);
	}
	.num-cell {
		text-align: right;
		color: var(--text-secondary);
	}
	.actions-cell {
		text-align: right;
		width: 80px;
	}
	.kill-btn {
		width: 22px;
		height: 22px;
		border: none;
		border-radius: 4px;
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		transition: all var(--duration-fast) var(--ease-out);
		padding: 0;
	}
	.kill-btn svg {
		width: 10px;
		height: 10px;
	}
	.kill-btn:hover {
		background: var(--red-subtle);
		color: var(--red);
	}

	/* ─── Details row ─── */
	.details-row td {
		padding: 0;
		background: var(--bg-table-alt);
		border-bottom: 0.5px solid var(--border-subtle);
	}
	.details-content {
		padding: 10px 12px 10px 32px;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.detail-row {
		display: flex;
		gap: 10px;
		font-size: 11px;
	}
	.detail-label {
		color: var(--text-tertiary);
		font-weight: 500;
		min-width: 70px;
		flex-shrink: 0;
	}
	.detail-value {
		color: var(--text-secondary);
		word-break: break-all;
	}
	.loading {
		font-size: 11px;
		color: var(--text-tertiary);
	}
</style>
