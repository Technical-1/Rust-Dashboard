<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { save } from '@tauri-apps/plugin-dialog';
	import { systemSnapshot } from '$lib/stores/system';
	import { logError } from '$lib/log';

	let exporting = false;

	function csvEscape(val: string): string {
		let safe = val;
		if (/^[=+\-@\t\r]/.test(safe)) {
			safe = "'" + safe;
		}
		return `"${safe.replace(/"/g, '""')}"`;
	}

	async function exportJSON() {
		if (!$systemSnapshot) return;
		exporting = true;
		try {
			const processes = $systemSnapshot.processes;
			const data = {
				timestamp: Math.floor(Date.now() / 1000),
				cpu_usage: $systemSnapshot.cpu_usage,
				memory: {
					used_gb: $systemSnapshot.memory.used / 1024 / 1024 / 1024,
					free_gb: $systemSnapshot.memory.free / 1024 / 1024 / 1024,
					total_gb: $systemSnapshot.memory.total / 1024 / 1024 / 1024
				},
				processes: processes.map((p) => ({
					name: p.name,
					cpu_usage: p.cpu_usage,
					memory_mb: Math.floor(p.memory_usage / 1024 / 1024),
					pids: p.pids
				}))
			};
			const jsonStr = JSON.stringify(data, null, 2);
			const path = await save({
				filters: [{ name: 'JSON', extensions: ['json'] }],
				defaultPath: 'dashboard-export.json'
			});
			if (path) {
				await invoke('export_to_file', { data: jsonStr, path });
			}
		} catch (e) {
			logError('Export JSON failed', e);
		} finally {
			exporting = false;
		}
	}

	async function exportCSV() {
		if (!$systemSnapshot) return;
		exporting = true;
		try {
			const processes = $systemSnapshot.processes;
			let csv = 'Type,Name,CPU Usage %,Memory MB,PIDs\n';
			csv += `${csvEscape('System')},${csvEscape('CPU')},${$systemSnapshot.cpu_usage.toFixed(2)},,\n`;
			csv += `${csvEscape('System')},${csvEscape('Memory')},,${Math.floor($systemSnapshot.memory.used / 1024 / 1024)},\n`;
			for (const p of processes) {
				const pids = p.pids.join(';');
				csv += `${csvEscape('Process')},${csvEscape(p.name)},${p.cpu_usage.toFixed(2)},${Math.floor(p.memory_usage / 1024 / 1024)},${csvEscape(pids)}\n`;
			}
			const path = await save({
				filters: [{ name: 'CSV', extensions: ['csv'] }],
				defaultPath: 'dashboard-export.csv'
			});
			if (path) {
				await invoke('export_to_file', { data: csv, path });
			}
		} catch (e) {
			logError('Export CSV failed', e);
		} finally {
			exporting = false;
		}
	}
</script>

<div class="export-buttons">
	<button class="export-btn" on:click={exportJSON} disabled={exporting || !$systemSnapshot}>
		<svg viewBox="0 0 14 14" fill="none">
			<path d="M3 1.5h5l3.5 3.5V12a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V2.5A1 1 0 0 1 3 1.5z" stroke="currentColor" stroke-width="1.1" stroke-linejoin="round"/>
			<path d="M7.5 1.5V5.5H11.5" stroke="currentColor" stroke-width="1.1" stroke-linejoin="round"/>
		</svg>
		JSON
	</button>
	<button class="export-btn" on:click={exportCSV} disabled={exporting || !$systemSnapshot}>
		<svg viewBox="0 0 14 14" fill="none">
			<path d="M3 1.5h5l3.5 3.5V12a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V2.5A1 1 0 0 1 3 1.5z" stroke="currentColor" stroke-width="1.1" stroke-linejoin="round"/>
			<path d="M7.5 1.5V5.5H11.5" stroke="currentColor" stroke-width="1.1" stroke-linejoin="round"/>
		</svg>
		CSV
	</button>
</div>

<style>
	.export-buttons {
		display: flex;
		gap: 6px;
	}
	.export-btn {
		display: flex;
		align-items: center;
		gap: 5px;
		padding: 5px 12px;
		border: 0.5px solid var(--border-input);
		border-radius: var(--radius-s);
		background: var(--bg-input);
		color: var(--text-secondary);
		font-size: 11px;
		font-weight: 500;
		font-family: inherit;
		cursor: pointer;
		transition: all var(--duration-fast) var(--ease-out);
	}
	.export-btn svg {
		width: 12px;
		height: 12px;
	}
	.export-btn:hover:not(:disabled) {
		background: var(--bg-sidebar-hover);
		color: var(--text-primary);
		border-color: var(--border-subtle);
	}
	.export-btn:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}
</style>
