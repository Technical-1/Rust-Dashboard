<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { paused, theme, sidebarCollapsed, togglePause } from '$lib/stores/config';
	import { systemSnapshot } from '$lib/stores/system';
	import { formatBytes, formatUptime } from '$lib/utils';
	import { logError } from '$lib/log';

	function toggleTheme() {
		theme.update((t) => (t === 'Dark' ? 'Light' : 'Dark'));
	}

	async function manualRefresh() {
		try {
			await invoke('manual_refresh');
		} catch (e) {
			logError('Manual refresh failed', e);
		}
	}

	function startDrag(e: MouseEvent) {
		// Only drag on left click, and not on interactive elements
		if (e.button !== 0) return;
		const target = e.target as HTMLElement;
		if (target.closest('button') || target.closest('input')) return;
		getCurrentWindow().startDragging();
	}

	$: selfCpu = $systemSnapshot?.self_usage?.cpu ?? 0;
	$: selfMem = $systemSnapshot?.self_usage?.memory ?? 0;
	$: uptime = $systemSnapshot?.uptime_seconds ?? 0;
	$: load = $systemSnapshot?.load_average ?? [0, 0, 0];
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<header class="topbar" style="left: {$sidebarCollapsed ? '52px' : 'var(--sidebar-width)'}" on:mousedown={startDrag}>
	<div class="topbar-left"></div>

	<div class="topbar-center">
		<!-- Self-usage indicator -->
		<div class="self-usage mono">
			<div class="usage-dot" style="background: var(--green)"></div>
			<span>{selfCpu.toFixed(1)}% CPU</span>
			<span class="divider"></span>
			<span>{formatBytes(selfMem)}</span>
			{#if uptime > 0}
				<span class="divider"></span>
				<span class="uptime-label">Up {formatUptime(uptime)}</span>
			{/if}
			{#if load[0] > 0}
				<span class="divider"></span>
				<span class="load-label">Load {load[0].toFixed(2)}</span>
			{/if}
		</div>
	</div>

	<div class="topbar-right">
		{#if $paused}
			<span class="paused-badge">PAUSED</span>
		{/if}

		<button
			class="icon-btn"
			class:active={$paused}
			class:paused-btn={$paused}
			on:click={() => togglePause(!$paused)}
			title={$paused ? 'Resume' : 'Pause'}
			aria-pressed={$paused}
			aria-label={$paused ? 'Resume monitoring' : 'Pause monitoring'}
		>
			{#if $paused}
				<svg viewBox="0 0 16 16" fill="none">
					<path d="M5 3L13 8L5 13V3Z" fill="currentColor"/>
				</svg>
			{:else}
				<svg viewBox="0 0 16 16" fill="none">
					<rect x="3" y="2.5" width="3.5" height="11" rx="1" fill="currentColor"/>
					<rect x="9.5" y="2.5" width="3.5" height="11" rx="1" fill="currentColor"/>
				</svg>
			{/if}
		</button>

		<button
			class="icon-btn"
			on:click={manualRefresh}
			disabled={$paused}
			title="Refresh now"
			aria-label="Refresh system data now"
		>
			<svg viewBox="0 0 16 16" fill="none">
				<path d="M13.5 8A5.5 5.5 0 1 1 8 2.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
				<path d="M8 1L10.5 3.5L8 6" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
			</svg>
		</button>

		<button class="icon-btn" on:click={toggleTheme} title="Toggle theme" aria-label="Toggle dark/light theme" aria-pressed={$theme === 'Dark'}>
			{#if $theme === 'Dark'}
				<svg viewBox="0 0 16 16" fill="none">
					<path d="M13.5 9.5a5.5 5.5 0 1 1-7-7 4.5 4.5 0 0 0 7 7z" stroke="currentColor" stroke-width="1.2" fill="none"/>
				</svg>
			{:else}
				<svg viewBox="0 0 16 16" fill="none">
					<circle cx="8" cy="8" r="3" stroke="currentColor" stroke-width="1.2"/>
					<line x1="8" y1="1.5" x2="8" y2="3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
					<line x1="8" y1="13" x2="8" y2="14.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
					<line x1="1.5" y1="8" x2="3" y2="8" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
					<line x1="13" y1="8" x2="14.5" y2="8" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
					<line x1="3.4" y1="3.4" x2="4.5" y2="4.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
					<line x1="11.5" y1="11.5" x2="12.6" y2="12.6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
					<line x1="3.4" y1="12.6" x2="4.5" y2="11.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
					<line x1="11.5" y1="4.5" x2="12.6" y2="3.4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
				</svg>
			{/if}
		</button>
	</div>
</header>

<style>
	.topbar {
		height: var(--topbar-height);
		position: fixed;
		top: 0;
		right: 0;
		transition: left var(--duration-normal) var(--ease-out);
		z-index: 100;
		background: var(--bg-glass);
		backdrop-filter: blur(var(--blur-glass)) saturate(1.5);
		-webkit-backdrop-filter: blur(var(--blur-glass)) saturate(1.5);
		border-bottom: 0.5px solid var(--border-subtle);
		display: flex;
		align-items: flex-end;
		padding-bottom: 8px;
		padding-left: 12px;
		padding-right: 12px;
	}
	.topbar-left,
	.topbar-center,
	.topbar-right {
		display: flex;
		align-items: center;
	}
	.topbar-left {
		flex: 1;
		min-width: 0;
		overflow: hidden;
	}
	.topbar-center {
		flex: 0 0 auto;
		margin: 0 8px;
	}
	.topbar-right {
		flex: 1;
		justify-content: flex-end;
		gap: 2px;
		flex-shrink: 0;
	}

	/* ─── Self-usage ─── */
	.self-usage {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 11px;
		color: var(--text-secondary);
	}
	.usage-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		flex-shrink: 0;
	}
	.divider {
		width: 1px;
		height: 10px;
		background: var(--border-subtle);
	}
	.uptime-label,
	.load-label {
		color: var(--text-tertiary);
	}

	/* ─── Paused badge ─── */
	.paused-badge {
		font-size: 10px;
		font-weight: 700;
		color: var(--yellow);
		letter-spacing: 0.5px;
		padding: 2px 6px;
		border-radius: 4px;
		background: var(--yellow-subtle);
		animation: paused-pulse 2s ease-in-out infinite;
		margin-right: 4px;
	}
	@keyframes paused-pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.5; }
	}

	/* ─── Icon buttons ─── */
	.icon-btn {
		width: 28px;
		height: 28px;
		border: none;
		border-radius: var(--radius-s);
		background: transparent;
		color: var(--text-secondary);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all var(--duration-fast) var(--ease-out);
		padding: 0;
	}
	.icon-btn svg {
		width: 15px;
		height: 15px;
	}
	.icon-btn:hover:not(:disabled) {
		background: var(--bg-sidebar-hover);
		color: var(--text-primary);
	}
	.icon-btn:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}
	.icon-btn.active {
		background: var(--accent-subtle);
		color: var(--accent);
	}
	.icon-btn.paused-btn {
		background: var(--yellow-subtle);
		color: var(--yellow);
	}
</style>
