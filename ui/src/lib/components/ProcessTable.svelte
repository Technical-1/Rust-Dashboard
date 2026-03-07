<script lang="ts">
	import { systemSnapshot } from '$lib/stores/system';
	import {
		searchQuery,
		cpuThreshold,
		memoryThresholdMB,
		sortColumn,
		sortDirection,
		expandedProcesses
	} from '$lib/stores/processes';
	import ProcessRow from './ProcessRow.svelte';
	import KillConfirmDialog from './KillConfirmDialog.svelte';
	import type { CombinedProcess, SortColumn } from '$lib/types';
	import { openDetachedWindow } from '$lib/windowManager';

	const MAX_VISIBLE_PROCESSES = 50;

	export let constrainHeight: boolean = true;
	export let showDetachButton: boolean = true;

	let killTarget: { name: string; pid: number } | null = null;

	$: processes = $systemSnapshot?.processes ?? [];

	$: filtered = processes
		.filter((p) => {
			const matchesSearch =
				$searchQuery === '' || p.name.toLowerCase().includes($searchQuery.toLowerCase());
			const matchesCpu = p.cpu_usage >= $cpuThreshold;
			const matchesMem = p.memory_usage / 1024 / 1024 >= $memoryThresholdMB;
			return matchesSearch && matchesCpu && matchesMem;
		})
		.sort((a, b) => {
			let cmp = 0;
			switch ($sortColumn) {
				case 'name':
					cmp = a.name.localeCompare(b.name);
					break;
				case 'cpu':
					cmp = a.cpu_usage - b.cpu_usage;
					break;
				case 'memory':
					cmp = a.memory_usage - b.memory_usage;
					break;
				case 'pids':
					cmp = a.pids.length - b.pids.length;
					break;
			}
			return $sortDirection === 'desc' ? -cmp : cmp;
		})
		.slice(0, MAX_VISIBLE_PROCESSES);

	function toggleSort(col: SortColumn) {
		if ($sortColumn === col) {
			sortDirection.update((d) => (d === 'asc' ? 'desc' : 'asc'));
		} else {
			sortColumn.set(col);
			sortDirection.set(col === 'name' ? 'asc' : 'desc');
		}
	}

	function handleSortKeydown(e: KeyboardEvent, col: SortColumn) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			toggleSort(col);
		}
	}

	function isActiveSort(col: SortColumn): boolean {
		return $sortColumn === col;
	}

	function handleToggle(e: CustomEvent<{ name: string; expanded: boolean }>) {
		expandedProcesses.update((set) => {
			const next = new Set(set);
			if (e.detail.expanded) {
				next.add(e.detail.name);
			} else {
				next.delete(e.detail.name);
			}
			return next;
		});
	}

	function handleKillRequest(e: CustomEvent<{ name: string; pid: number }>) {
		killTarget = e.detail;
	}

	function handleKillClose() {
		killTarget = null;
	}
</script>

<div class="panel glass" role="region" aria-label="Process list">
	<div class="panel-header">
		<div class="panel-title">
			<svg class="panel-icon" viewBox="0 0 16 16" fill="none">
				<line x1="2" y1="4" x2="14" y2="4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
				<line x1="2" y1="8" x2="14" y2="8" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
				<line x1="2" y1="12" x2="10" y2="12" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
			</svg>
			<h3>Processes</h3>
		</div>
		<div class="header-right">
			<span class="count mono">{filtered.length} / {processes.length}</span>
			{#if showDetachButton}
				<button class="detach-btn" on:click={() => openDetachedWindow('processes')} title="Open in new window" aria-label="Open processes in new window">
					<svg viewBox="0 0 16 16" fill="none">
						<path d="M9 2h5v5M14 2L7 9M6 3H3a1 1 0 00-1 1v9a1 1 0 001 1h9a1 1 0 001-1v-3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
					</svg>
				</button>
			{/if}
		</div>
	</div>

	{#if !$systemSnapshot}
		<div class="skeleton-table">
			{#each Array(5) as _}
				<div class="skeleton-row">
					<div class="skeleton-cell wide"></div>
					<div class="skeleton-cell"></div>
					<div class="skeleton-cell"></div>
					<div class="skeleton-cell narrow"></div>
				</div>
			{/each}
		</div>
	{:else}
		<div class="filters">
			<div class="search-wrap">
				<svg class="search-icon" viewBox="0 0 16 16" fill="none">
					<circle cx="7" cy="7" r="4.5" stroke="currentColor" stroke-width="1.2"/>
					<line x1="10.5" y1="10.5" x2="14" y2="14" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
				</svg>
				<input
					class="search-input"
					type="text"
					placeholder="Filter processes..."
					bind:value={$searchQuery}
					aria-label="Filter processes by name"
				/>
			</div>
			<div class="filter-group">
				<span class="filter-label">CPU {$cpuThreshold > 0 ? `\u2265 ${$cpuThreshold.toFixed(0)}%` : ''}</span>
				<input type="range" min="0" max="100" step="1" bind:value={$cpuThreshold} aria-label="Minimum CPU usage filter" />
			</div>
			<div class="filter-group">
				<span class="filter-label">Mem {$memoryThresholdMB > 0 ? `\u2265 ${$memoryThresholdMB} MB` : ''}</span>
				<input type="range" min="0" max="10000" step="50" bind:value={$memoryThresholdMB} aria-label="Minimum memory usage filter" />
			</div>
		</div>

		<div class="table-wrapper" class:constrained={constrainHeight}>
			<table>
				<thead>
					<tr>
						<th class="sortable" class:active-sort={isActiveSort('name')} on:click={() => toggleSort('name')} on:keydown={(e) => handleSortKeydown(e, 'name')} tabindex="0" role="columnheader" aria-sort={isActiveSort('name') ? ($sortDirection === 'asc' ? 'ascending' : 'descending') : 'none'}>
							<span>Process</span>
							{#if isActiveSort('name')}
								<svg class="sort-chevron" class:sort-asc={$sortDirection === 'asc'} viewBox="0 0 10 6" fill="none">
									<path d="M1 1L5 5L9 1" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
								</svg>
							{/if}
						</th>
						<th class="sortable num" class:active-sort={isActiveSort('cpu')} on:click={() => toggleSort('cpu')} on:keydown={(e) => handleSortKeydown(e, 'cpu')} tabindex="0" role="columnheader" aria-sort={isActiveSort('cpu') ? ($sortDirection === 'asc' ? 'ascending' : 'descending') : 'none'}>
							<span>CPU</span>
							{#if isActiveSort('cpu')}
								<svg class="sort-chevron" class:sort-asc={$sortDirection === 'asc'} viewBox="0 0 10 6" fill="none">
									<path d="M1 1L5 5L9 1" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
								</svg>
							{/if}
						</th>
						<th class="sortable num" class:active-sort={isActiveSort('memory')} on:click={() => toggleSort('memory')} on:keydown={(e) => handleSortKeydown(e, 'memory')} tabindex="0" role="columnheader" aria-sort={isActiveSort('memory') ? ($sortDirection === 'asc' ? 'ascending' : 'descending') : 'none'}>
							<span>Memory</span>
							{#if isActiveSort('memory')}
								<svg class="sort-chevron" class:sort-asc={$sortDirection === 'asc'} viewBox="0 0 10 6" fill="none">
									<path d="M1 1L5 5L9 1" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
								</svg>
							{/if}
						</th>
						<th class="sortable num" class:active-sort={isActiveSort('pids')} on:click={() => toggleSort('pids')} on:keydown={(e) => handleSortKeydown(e, 'pids')} tabindex="0" role="columnheader" aria-sort={isActiveSort('pids') ? ($sortDirection === 'asc' ? 'ascending' : 'descending') : 'none'}>
							<span>PIDs</span>
							{#if isActiveSort('pids')}
								<svg class="sort-chevron" class:sort-asc={$sortDirection === 'asc'} viewBox="0 0 10 6" fill="none">
									<path d="M1 1L5 5L9 1" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
								</svg>
							{/if}
						</th>
						<th class="actions-col"></th>
					</tr>
				</thead>
				<tbody>
					{#each filtered as proc (proc.name)}
						<ProcessRow
							process={proc}
							expanded={$expandedProcesses.has(proc.name)}
							on:toggle={handleToggle}
							on:kill={handleKillRequest}
						/>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>

{#if killTarget}
	<KillConfirmDialog
		open={true}
		processName={killTarget.name}
		pid={killTarget.pid}
		on:close={handleKillClose}
	/>
{/if}

<style>
	/* ─── Filters ─── */
	.filters {
		display: flex;
		gap: 10px;
		align-items: center;
		flex-wrap: wrap;
	}
	.search-wrap {
		flex: 1;
		min-width: 120px;
		position: relative;
	}
	.search-icon {
		width: 13px;
		height: 13px;
		position: absolute;
		left: 8px;
		top: 50%;
		transform: translateY(-50%);
		color: var(--text-tertiary);
		pointer-events: none;
	}
	.search-input {
		width: 100%;
		padding: 5px 10px 5px 26px;
		border: 0.5px solid var(--border-input);
		border-radius: var(--radius-s);
		background: var(--bg-input);
		color: var(--text-primary);
		font-size: 12px;
		font-family: inherit;
		outline: none;
		transition: all var(--duration-fast) var(--ease-out);
	}
	.search-input:focus {
		background: var(--bg-input-focus);
		border-color: var(--accent);
		box-shadow: 0 0 0 3px var(--accent-subtle);
	}
	.search-input::placeholder {
		color: var(--text-tertiary);
	}
	.filter-group {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.filter-label {
		font-size: 11px;
		color: var(--text-tertiary);
		white-space: nowrap;
		min-width: 28px;
	}
	.filter-group input[type='range'] {
		width: 70px;
		accent-color: var(--accent);
		height: 3px;
	}

	/* ─── Table ─── */
	.table-wrapper {
		overflow-x: auto;
		overflow-y: auto;
		border-radius: var(--radius-s);
	}
	.table-wrapper.constrained {
		max-height: 500px;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 12px;
	}
	thead {
		position: sticky;
		top: 0;
		z-index: 1;
	}
	th {
		padding: 6px 12px;
		text-align: left;
		font-weight: 500;
		font-size: 11px;
		color: var(--text-tertiary);
		text-transform: uppercase;
		letter-spacing: 0.4px;
		background: var(--bg-glass);
		backdrop-filter: blur(20px);
		-webkit-backdrop-filter: blur(20px);
		border-bottom: 0.5px solid var(--border-subtle);
		user-select: none;
		white-space: nowrap;
	}
	th span {
		vertical-align: middle;
	}
	th.num {
		text-align: right;
	}
	th.sortable {
		cursor: pointer;
	}
	th.sortable:hover {
		color: var(--accent);
	}
	th.active-sort {
		color: var(--accent);
	}
	th.actions-col {
		width: 80px;
	}

	/* ─── Sort chevrons ─── */
	.sort-chevron {
		width: 10px;
		height: 6px;
		display: inline-block;
		vertical-align: middle;
		margin-left: 3px;
		transition: transform var(--duration-fast) var(--ease-out);
	}
	.sort-chevron.sort-asc {
		transform: rotate(180deg);
	}

	/* ─── Skeletons ─── */
	.skeleton-table {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 4px 0;
	}
	.skeleton-row {
		display: flex;
		gap: 12px;
		align-items: center;
	}
	.skeleton-cell {
		height: 12px;
		width: 60px;
		border-radius: 4px;
		background: var(--bg-progress);
		animation: skeleton-pulse 1.5s ease-in-out infinite;
	}
	.skeleton-cell.wide {
		flex: 1;
	}
	.skeleton-cell.narrow {
		width: 36px;
	}

	.count {
		font-size: 11px;
		color: var(--text-tertiary);
	}

	@keyframes skeleton-pulse {
		0%, 100% { opacity: 0.4; }
		50% { opacity: 1; }
	}
</style>
