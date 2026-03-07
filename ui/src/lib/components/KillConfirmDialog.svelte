<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { logError } from '$lib/log';

	export let processName: string = '';
	export let pid: number = 0;
	export let open: boolean = false;

	const dispatch = createEventDispatcher();
	let killing = false;

	async function handleKill() {
		killing = true;
		try {
			await invoke('kill_process', { pid });
			dispatch('killed', { pid, name: processName });
		} catch (e) {
			logError('Failed to kill process', e);
		} finally {
			killing = false;
			open = false;
			dispatch('close');
		}
	}

	function handleCancel() {
		open = false;
		dispatch('close');
	}
</script>

{#if open}
	<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
	<div class="backdrop" on:click={handleCancel}>
		<div class="dialog" on:click|stopPropagation>
			<div class="dialog-icon">
				<svg viewBox="0 0 24 24" fill="none">
					<circle cx="12" cy="12" r="10" stroke="var(--red)" stroke-width="1.5"/>
					<line x1="8" y1="8" x2="16" y2="16" stroke="var(--red)" stroke-width="1.5" stroke-linecap="round"/>
					<line x1="16" y1="8" x2="8" y2="16" stroke="var(--red)" stroke-width="1.5" stroke-linecap="round"/>
				</svg>
			</div>
			<h3>Terminate Process?</h3>
			<p>
				<strong>{processName}</strong><br/>
				<span class="pid-label">PID {pid}</span>
			</p>
			<div class="actions">
				<button class="btn btn-cancel" on:click={handleCancel}>Cancel</button>
				<button class="btn btn-kill" on:click={handleKill} disabled={killing}>
					{killing ? 'Terminating...' : 'Terminate'}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.3);
		backdrop-filter: blur(8px) saturate(1.5);
		-webkit-backdrop-filter: blur(8px) saturate(1.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}
	.dialog {
		background: var(--bg-popover);
		backdrop-filter: blur(var(--blur-popover)) saturate(1.5);
		-webkit-backdrop-filter: blur(var(--blur-popover)) saturate(1.5);
		border: 0.5px solid var(--border-glass);
		border-radius: var(--radius-l);
		padding: 24px;
		max-width: 320px;
		width: 85%;
		box-shadow: var(--shadow-popover);
		text-align: center;
	}
	.dialog-icon {
		margin-bottom: 12px;
	}
	.dialog-icon svg {
		width: 36px;
		height: 36px;
	}
	h3 {
		margin: 0 0 8px;
		font-size: 15px;
		font-weight: 600;
		color: var(--text-primary);
	}
	p {
		margin: 0 0 20px;
		font-size: 13px;
		color: var(--text-secondary);
		line-height: 1.5;
	}
	.pid-label {
		font-size: 11px;
		color: var(--text-tertiary);
		font-variant-numeric: tabular-nums;
	}
	.actions {
		display: flex;
		gap: 8px;
	}
	.btn {
		flex: 1;
		padding: 7px 16px;
		border-radius: var(--radius-s);
		border: none;
		font-size: 13px;
		font-weight: 500;
		font-family: inherit;
		cursor: pointer;
		transition: all var(--duration-fast) var(--ease-out);
	}
	.btn:hover {
		filter: brightness(1.05);
	}
	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.btn-cancel {
		background: var(--bg-input);
		color: var(--text-primary);
		border: 0.5px solid var(--border-input);
	}
	.btn-cancel:hover {
		background: var(--bg-sidebar-hover);
	}
	.btn-kill {
		background: var(--red);
		color: white;
	}
</style>
