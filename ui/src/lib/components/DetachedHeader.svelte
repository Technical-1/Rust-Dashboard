<script lang="ts">
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { emit } from '@tauri-apps/api/event';
	import ContextMenu from './ContextMenu.svelte';
	import type { DetachableView } from '$lib/types';

	export let view: DetachableView;

	const TITLES: Record<DetachableView, string> = {
		cpu: 'CPU Monitor',
		memory: 'Memory Monitor',
		disks: 'Disk Monitor',
		network: 'Network Monitor',
		processes: 'Process Monitor'
	};

	let contextMenu: { x: number; y: number } | null = null;

	function handleDrag() {
		getCurrentWindow().startDragging();
	}

	async function mergeBack() {
		await emit('merge-back', { view });
		await getCurrentWindow().close();
	}

	function handleContextMenu(e: MouseEvent) {
		e.preventDefault();
		contextMenu = { x: e.clientX, y: e.clientY };
	}
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="detached-header" on:mousedown={handleDrag} on:contextmenu={handleContextMenu}>
	<span class="title">{TITLES[view]}</span>
	<button class="merge-btn" on:mousedown|stopPropagation on:click={mergeBack} title="Merge back to Dashboard">
		<svg viewBox="0 0 16 16" fill="none">
			<path d="M9 7H13V3M13 7L8 2M3 9v4a1 1 0 001 1h4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
		</svg>
	</button>
</div>

{#if contextMenu}
	<ContextMenu
		x={contextMenu.x}
		y={contextMenu.y}
		items={[{ label: 'Merge back to Dashboard', action: mergeBack }]}
		onClose={() => (contextMenu = null)}
	/>
{/if}

<style>
	.detached-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px 14px;
		background: var(--bg-glass);
		backdrop-filter: blur(var(--blur-glass)) saturate(1.8);
		-webkit-backdrop-filter: blur(var(--blur-glass)) saturate(1.8);
		border-bottom: 0.5px solid var(--border-subtle);
		cursor: grab;
		user-select: none;
		-webkit-app-region: drag;
	}
	.detached-header:active {
		cursor: grabbing;
	}
	.title {
		font-size: 12px;
		font-weight: 600;
		color: var(--text-primary);
		letter-spacing: -0.01em;
	}
	.merge-btn {
		width: 24px;
		height: 24px;
		padding: 4px;
		border: none;
		border-radius: var(--radius-s);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all var(--duration-fast) var(--ease-out);
		-webkit-app-region: no-drag;
	}
	.merge-btn:hover {
		background: var(--bg-sidebar-hover);
		color: var(--accent);
	}
	.merge-btn svg {
		width: 14px;
		height: 14px;
	}
</style>
