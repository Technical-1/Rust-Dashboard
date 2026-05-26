<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { listen } from '@tauri-apps/api/event';
	import { invoke } from '@tauri-apps/api/core';
	import { emit } from '@tauri-apps/api/event';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { getStatusColor, formatBytes, formatBytesPerSec, formatUptime } from '$lib/utils';
	import type { SystemSnapshot } from '$lib/types';
	import { logError } from '$lib/log';
	import { loadConfig } from '$lib/stores/config';

	const REFRESH_MS = 2000;

	let snapshot: SystemSnapshot | null = null;
	let refreshTimer: ReturnType<typeof setInterval> | null = null;
	let unlistenVisible: (() => void) | null = null;
	let unlistenPaused: (() => void) | null = null;
	// Local pause state for this window. Tauri webviews each have their own
	// Svelte runtime, so the main window's `paused` store doesn't propagate
	// here. We seed from the backend (paused starts false at app launch
	// since it's not persisted to AppConfig) and update from the
	// "paused-changed" event emitted by set_paused on every toggle.
	let pausedLocal = false;

	async function doRefresh() {
		// Skip the IPC call entirely when paused — avoids both wasted work
		// and mutex contention with the background thread.
		if (pausedLocal) return;
		try {
			snapshot = await invoke<SystemSnapshot>('tray_refresh');
		} catch (e) {
			logError('tray_refresh failed', e);
		}
	}

	function startRefresh() {
		doRefresh();
		if (!refreshTimer) {
			refreshTimer = setInterval(doRefresh, REFRESH_MS);
		}
	}

	function stopRefresh() {
		if (refreshTimer) {
			clearInterval(refreshTimer);
			refreshTimer = null;
		}
	}

	onMount(async () => {
		// Load config so the tray popup follows the user's theme
		await loadConfig();

		// Listen for visibility events from Rust
		unlistenVisible = await listen<boolean>('tray-visible', (event) => {
			if (event.payload) {
				startRefresh();
			} else {
				stopRefresh();
			}
		});

		// Listen for global paused-changed events. Updates pausedLocal so
		// the next doRefresh tick respects the new state.
		unlistenPaused = await listen<boolean>('paused-changed', (event) => {
			pausedLocal = event.payload;
		});

		// Start immediately — popup is visible when first mounted
		startRefresh();
	});

	onDestroy(() => {
		stopRefresh();
		if (unlistenVisible) {
			unlistenVisible();
			unlistenVisible = null;
		}
		if (unlistenPaused) {
			unlistenPaused();
			unlistenPaused = null;
		}
	});

	// --- Derived values ---
	$: cpu = snapshot?.cpu_usage ?? 0;
	$: coreCount = snapshot?.per_cpu?.length ?? 0;
	$: mem = snapshot?.memory;
	$: usedGb = mem ? mem.used / 1024 / 1024 / 1024 : 0;
	$: totalGb = mem ? mem.total / 1024 / 1024 / 1024 : 0;
	$: memPercent = totalGb > 0 ? (usedGb / totalGb) * 100 : 0;

	$: primaryDisk = snapshot?.disks?.[0];
	$: diskUsed = primaryDisk ? primaryDisk.used : 0;
	$: diskTotal = primaryDisk ? primaryDisk.total : 0;
	$: diskPercent = diskTotal > 0 ? (diskUsed / diskTotal) * 100 : 0;

	$: totalRx = (snapshot?.networks ?? []).reduce((sum, n) => sum + n.rx_rate, 0);
	$: totalTx = (snapshot?.networks ?? []).reduce((sum, n) => sum + n.tx_rate, 0);

	$: topProcesses = (snapshot?.processes ?? [])
		.sort((a, b) => b.cpu_usage - a.cpu_usage)
		.slice(0, 5);

	$: uptime = snapshot?.uptime_seconds ?? 0;
	$: load = snapshot?.load_average ?? [0, 0, 0];

	async function openDashboard() {
		await emit('show-main-window', {});
		await getCurrentWindow().hide();
	}
</script>

<div class="tray-popup">
	<!-- Header: Uptime + Load -->
	<div class="tray-header">
		<div class="header-left">
			<svg class="app-icon" viewBox="0 0 16 16" fill="none">
				<rect x="1.5" y="3" width="13" height="10" rx="2" stroke="currentColor" stroke-width="1.2"/>
				<line x1="4.5" y1="6" x2="4.5" y2="10" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
				<line x1="8" y1="5" x2="8" y2="10" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
				<line x1="11.5" y1="7" x2="11.5" y2="10" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
			</svg>
			<span class="tray-title">Rust Dashboard</span>
		</div>
		<span class="uptime-badge mono">{formatUptime(uptime)}</span>
	</div>

	<div class="load-row mono">
		<span class="load-label">Load</span>
		<span>{load[0].toFixed(2)}</span>
		<span class="load-sep">/</span>
		<span>{load[1].toFixed(2)}</span>
		<span class="load-sep">/</span>
		<span>{load[2].toFixed(2)}</span>
	</div>

	<!-- CPU -->
	<div class="stat-card">
		<div class="stat-header">
			<span class="stat-label">CPU</span>
			<span class="stat-sub mono">{coreCount} cores</span>
		</div>
		<div class="stat-row">
			<div class="stat-bar-wrap" role="progressbar" aria-valuenow={cpu} aria-valuemin={0} aria-valuemax={100} aria-label="CPU usage">
				<div class="stat-bar" style="width: {Math.min(cpu, 100)}%; background: {getStatusColor(cpu)}"></div>
			</div>
			<span class="stat-value mono" style="color: {getStatusColor(cpu)}">{cpu.toFixed(1)}%</span>
		</div>
	</div>

	<!-- Memory -->
	<div class="stat-card">
		<div class="stat-header">
			<span class="stat-label">Memory</span>
			<span class="stat-sub mono">{usedGb.toFixed(1)} / {totalGb.toFixed(0)} GB</span>
		</div>
		<div class="stat-row">
			<div class="stat-bar-wrap" role="progressbar" aria-valuenow={memPercent} aria-valuemin={0} aria-valuemax={100} aria-label="Memory usage">
				<div class="stat-bar" style="width: {Math.min(memPercent, 100)}%; background: {getStatusColor(memPercent)}"></div>
			</div>
			<span class="stat-value mono" style="color: {getStatusColor(memPercent)}">{memPercent.toFixed(0)}%</span>
		</div>
	</div>

	<!-- Disk -->
	<div class="stat-card">
		<div class="stat-header">
			<span class="stat-label">Disk</span>
			<span class="stat-sub mono">{formatBytes(diskUsed)} / {formatBytes(diskTotal)}</span>
		</div>
		<div class="stat-row">
			<div class="stat-bar-wrap" role="progressbar" aria-valuenow={diskPercent} aria-valuemin={0} aria-valuemax={100} aria-label="Disk usage">
				<div class="stat-bar" style="width: {Math.min(diskPercent, 100)}%; background: {getStatusColor(diskPercent)}"></div>
			</div>
			<span class="stat-value mono" style="color: {getStatusColor(diskPercent)}">{diskPercent.toFixed(0)}%</span>
		</div>
	</div>

	<!-- Network -->
	<div class="stat-card net-card">
		<div class="stat-header">
			<span class="stat-label">Network</span>
		</div>
		<div class="net-row">
			<span class="net-arrow down">&#8595;</span>
			<span class="net-value mono">{formatBytesPerSec(totalRx)}</span>
			<span class="net-spacer"></span>
			<span class="net-arrow up">&#8593;</span>
			<span class="net-value mono">{formatBytesPerSec(totalTx)}</span>
		</div>
	</div>

	<!-- Divider -->
	<div class="divider"></div>

	<!-- Top Processes -->
	<div class="process-card">
		<div class="section-title">Top Processes</div>
		{#each topProcesses as proc, i}
			<div class="process-row" class:alt={i % 2 === 1}>
				<span class="proc-name">{proc.name}</span>
				<span class="proc-cpu mono" style="color: {getStatusColor(proc.cpu_usage)}">{proc.cpu_usage.toFixed(1)}%</span>
				<span class="proc-mem mono">{(proc.memory_usage / 1024 / 1024).toFixed(0)} MB</span>
			</div>
		{/each}
		{#if topProcesses.length === 0}
			<div class="empty-row">Loading...</div>
		{/if}
	</div>

	<!-- Footer -->
	<button class="open-btn" on:click={openDashboard} aria-label="Open full dashboard">
		Open Dashboard
	</button>
</div>

<style>
	.tray-popup {
		padding: 12px;
		display: flex;
		flex-direction: column;
		gap: 6px;
		height: 100vh;
		background: var(--bg-canvas);
		overflow: hidden;
	}

	/* Header */
	.tray-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 2px 2px;
	}
	.header-left {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.app-icon {
		width: 14px;
		height: 14px;
		color: var(--accent);
	}
	.tray-title {
		font-size: 12px;
		font-weight: 700;
		color: var(--text-primary);
		letter-spacing: -0.01em;
	}
	.uptime-badge {
		font-size: 10px;
		color: var(--text-tertiary);
		background: var(--bg-glass);
		border: 0.5px solid var(--border-glass);
		padding: 2px 6px;
		border-radius: var(--radius-s);
	}

	/* Load average */
	.load-row {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 0 2px;
		font-size: 10px;
		color: var(--text-secondary);
	}
	.load-label {
		font-weight: 600;
		color: var(--text-tertiary);
		margin-right: 2px;
	}
	.load-sep {
		color: var(--text-tertiary);
	}

	/* Stat cards */
	.stat-card {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 8px 10px;
		background: var(--bg-glass);
		border: 0.5px solid var(--border-glass);
		border-radius: var(--radius-m);
	}
	.stat-header {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
	}
	.stat-label {
		font-size: 11px;
		font-weight: 600;
		color: var(--text-secondary);
	}
	.stat-sub {
		font-size: 10px;
		color: var(--text-tertiary);
	}
	.stat-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.stat-bar-wrap {
		flex: 1;
		height: 4px;
		background: var(--bg-progress);
		border-radius: 2px;
		overflow: hidden;
	}
	.stat-bar {
		height: 100%;
		border-radius: 2px;
		transition: width 300ms ease-out;
	}
	.stat-value {
		font-size: 11px;
		font-weight: 700;
		min-width: 38px;
		text-align: right;
	}

	/* Network */
	.net-card {
		padding: 8px 10px;
	}
	.net-row {
		display: flex;
		align-items: center;
		gap: 4px;
		font-size: 11px;
	}
	.net-arrow {
		font-weight: 700;
		font-size: 12px;
	}
	.net-arrow.down {
		color: var(--green);
	}
	.net-arrow.up {
		color: var(--accent);
	}
	.net-value {
		color: var(--text-primary);
		font-weight: 600;
	}
	.net-spacer {
		flex: 1;
	}

	/* Divider */
	.divider {
		height: 0.5px;
		background: var(--border-glass);
		margin: 2px 0;
	}

	/* Processes */
	.process-card {
		display: flex;
		flex-direction: column;
		gap: 1px;
		flex: 1;
		min-height: 0;
	}
	.section-title {
		font-size: 10px;
		font-weight: 700;
		color: var(--text-tertiary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin-bottom: 2px;
		padding: 0 2px;
	}
	.process-row {
		display: flex;
		align-items: center;
		padding: 4px 6px;
		border-radius: var(--radius-s);
		transition: background var(--duration-fast) var(--ease-out);
	}
	.process-row.alt {
		background: var(--bg-table-alt);
	}
	.process-row:hover {
		background: var(--bg-table-hover);
	}
	.proc-name {
		font-size: 11px;
		font-weight: 500;
		color: var(--text-primary);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		flex: 1;
		min-width: 0;
	}
	.proc-cpu {
		font-size: 10px;
		font-weight: 600;
		flex-shrink: 0;
		width: 44px;
		text-align: right;
	}
	.proc-mem {
		font-size: 10px;
		font-weight: 500;
		color: var(--text-secondary);
		flex-shrink: 0;
		width: 48px;
		text-align: right;
	}
	.empty-row {
		font-size: 11px;
		color: var(--text-tertiary);
		text-align: center;
		padding: 12px;
	}

	/* Open Dashboard button */
	.open-btn {
		width: 100%;
		padding: 7px;
		border: none;
		border-radius: var(--radius-s);
		background: var(--accent);
		color: white;
		font-size: 12px;
		font-weight: 600;
		font-family: inherit;
		cursor: pointer;
		transition: all var(--duration-fast) var(--ease-out);
		margin-top: auto;
	}
	.open-btn:hover {
		background: var(--accent-hover);
		transform: translateY(-0.5px);
	}
	.open-btn:active {
		transform: translateY(0);
	}
</style>
