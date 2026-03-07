<script lang="ts">
	import { onMount, onDestroy } from 'svelte';

	export let x: number = 0;
	export let y: number = 0;
	export let items: { label: string; action: () => void }[] = [];
	export let onClose: () => void = () => {};

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}

	function handleClickOutside() {
		onClose();
	}

	onMount(() => {
		window.addEventListener('keydown', handleKeydown);
		// Delay to avoid closing immediately from the triggering right-click
		setTimeout(() => window.addEventListener('click', handleClickOutside), 10);
	});

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown);
		window.removeEventListener('click', handleClickOutside);
	});
</script>

<div class="context-menu glass" style="left: {x}px; top: {y}px;">
	{#each items as item}
		<button class="menu-item" on:click={item.action}>
			{item.label}
		</button>
	{/each}
</div>

<style>
	.context-menu {
		position: fixed;
		z-index: 1000;
		min-width: 180px;
		padding: 4px;
		background: var(--bg-popover);
		backdrop-filter: blur(var(--blur-popover)) saturate(1.8);
		-webkit-backdrop-filter: blur(var(--blur-popover)) saturate(1.8);
		border: 0.5px solid var(--border-glass);
		border-radius: var(--radius-m);
		box-shadow: var(--shadow-popover);
	}
	.menu-item {
		display: block;
		width: 100%;
		padding: 6px 12px;
		border: none;
		border-radius: var(--radius-s);
		background: transparent;
		color: var(--text-primary);
		font-size: 12px;
		font-family: inherit;
		text-align: left;
		cursor: pointer;
		transition: background var(--duration-fast) var(--ease-out);
	}
	.menu-item:hover {
		background: var(--bg-sidebar-hover);
	}
</style>
