<script lang="ts">
	import { activeView, sidebarCollapsed } from '$lib/stores/config';
	import type { ActiveView } from '$lib/types';

	const items: { id: ActiveView; label: string }[] = [
		{ id: 'overview', label: 'Overview' },
		{ id: 'cpu', label: 'CPU' },
		{ id: 'memory', label: 'Memory' },
		{ id: 'disks', label: 'Disks' },
		{ id: 'network', label: 'Network' },
		{ id: 'processes', label: 'Processes' }
	];

	function toggle() {
		sidebarCollapsed.update((v) => !v);
	}
</script>

<aside class="sidebar" class:collapsed={$sidebarCollapsed}>
	<div class="sidebar-inner">
		<div class="sidebar-header">
			{#if !$sidebarCollapsed}
				<div class="sidebar-title">Dashboard</div>
			{/if}
			<button class="toggle-btn" on:click={toggle} title={$sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}>
				<svg class="toggle-icon" class:flipped={$sidebarCollapsed} viewBox="0 0 16 16" fill="none">
					<path d="M10 3L5 8L10 13" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
				</svg>
			</button>
		</div>
		<nav aria-label="Main navigation">
			{#each items as item}
				<button
					class="nav-item"
					class:active={$activeView === item.id}
					on:click={() => activeView.set(item.id)}
					title={$sidebarCollapsed ? item.label : ''}
					aria-current={$activeView === item.id ? 'page' : undefined}
				>
					<svg class="nav-icon" viewBox="0 0 16 16" fill="none">
						{#if item.id === 'overview'}
							<rect x="1.5" y="1.5" width="5" height="5" rx="1.2" stroke="currentColor" stroke-width="1.2"/>
							<rect x="9.5" y="1.5" width="5" height="5" rx="1.2" stroke="currentColor" stroke-width="1.2"/>
							<rect x="1.5" y="9.5" width="5" height="5" rx="1.2" stroke="currentColor" stroke-width="1.2"/>
							<rect x="9.5" y="9.5" width="5" height="5" rx="1.2" stroke="currentColor" stroke-width="1.2"/>
						{:else if item.id === 'cpu'}
							<rect x="1.5" y="3" width="13" height="10" rx="2" stroke="currentColor" stroke-width="1.2"/>
							<line x1="4.5" y1="6" x2="4.5" y2="10" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
							<line x1="8" y1="5" x2="8" y2="10" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
							<line x1="11.5" y1="7" x2="11.5" y2="10" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
						{:else if item.id === 'memory'}
							<rect x="3" y="1.5" width="10" height="13" rx="1.5" stroke="currentColor" stroke-width="1.2"/>
							<line x1="5.5" y1="4.5" x2="10.5" y2="4.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
							<line x1="5.5" y1="7" x2="10.5" y2="7" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
							<line x1="5.5" y1="9.5" x2="10.5" y2="9.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
							<line x1="5.5" y1="12" x2="10.5" y2="12" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
						{:else if item.id === 'disks'}
							<circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.2"/>
							<circle cx="8" cy="8" r="2" stroke="currentColor" stroke-width="1.2"/>
							<line x1="8" y1="2" x2="8" y2="5" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
						{:else if item.id === 'network'}
							<path d="M2 11 L5.5 5 L9 9 L14 3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
							<line x1="11" y1="3" x2="14" y2="3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
							<line x1="14" y1="3" x2="14" y2="6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
						{:else if item.id === 'processes'}
							<line x1="2" y1="4" x2="14" y2="4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
							<line x1="2" y1="8" x2="14" y2="8" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
							<line x1="2" y1="12" x2="10" y2="12" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
						{/if}
					</svg>
					{#if !$sidebarCollapsed}
						<span class="nav-label">{item.label}</span>
					{/if}
				</button>
			{/each}
		</nav>
	</div>
</aside>

<style>
	.sidebar {
		width: var(--sidebar-width);
		height: 100vh;
		background: var(--bg-sidebar);
		backdrop-filter: blur(var(--blur-sidebar)) saturate(1.8);
		-webkit-backdrop-filter: blur(var(--blur-sidebar)) saturate(1.8);
		border-right: 0.5px solid var(--border-subtle);
		padding-top: var(--topbar-height);
		flex-shrink: 0;
		overflow-y: auto;
		overflow-x: hidden;
		position: relative;
		z-index: 10;
		transition: width var(--duration-normal) var(--ease-out);
	}
	.sidebar.collapsed {
		width: 52px;
	}
	.sidebar-inner {
		padding: 8px 6px;
	}
	.sidebar.collapsed .sidebar-inner {
		padding: 8px 4px;
	}
	.sidebar-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 6px 6px 14px;
		min-height: 34px;
	}
	.sidebar.collapsed .sidebar-header {
		justify-content: center;
		padding: 6px 0 14px;
	}
	.sidebar-title {
		font-size: 11px;
		font-weight: 600;
		color: var(--text-tertiary);
		text-transform: uppercase;
		letter-spacing: 0.6px;
		white-space: nowrap;
	}
	.toggle-btn {
		width: 24px;
		height: 24px;
		border: none;
		border-radius: var(--radius-s);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0;
		flex-shrink: 0;
		transition: all var(--duration-fast) var(--ease-out);
	}
	.toggle-btn:hover {
		background: var(--bg-sidebar-hover);
		color: var(--text-primary);
	}
	.toggle-icon {
		width: 14px;
		height: 14px;
		transition: transform var(--duration-normal) var(--ease-out);
	}
	.toggle-icon.flipped {
		transform: rotate(180deg);
	}
	nav {
		display: flex;
		flex-direction: column;
		gap: 1px;
	}
	.nav-item {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 10px;
		border: none;
		border-radius: var(--radius-s);
		background: transparent;
		color: var(--text-secondary);
		font-size: 13px;
		cursor: pointer;
		transition: all var(--duration-fast) var(--ease-out);
		text-align: left;
		width: 100%;
		font-family: inherit;
		letter-spacing: inherit;
		white-space: nowrap;
		overflow: hidden;
	}
	.sidebar.collapsed .nav-item {
		justify-content: center;
		padding: 8px;
	}
	.nav-item:hover {
		background: var(--bg-sidebar-hover);
		color: var(--text-primary);
	}
	.nav-item.active {
		background: var(--bg-sidebar-active);
		color: var(--accent);
	}
	.nav-icon {
		width: 16px;
		height: 16px;
		flex-shrink: 0;
		opacity: 0.7;
	}
	.nav-item.active .nav-icon {
		opacity: 1;
	}
	.nav-label {
		font-weight: 500;
	}
</style>
