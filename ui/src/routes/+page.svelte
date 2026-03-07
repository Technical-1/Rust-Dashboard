<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { listen } from '@tauri-apps/api/event';
	import { initSystemListener, destroySystemListener } from '$lib/stores/system';
	import { loadConfig, activeView, sidebarCollapsed } from '$lib/stores/config';
	import { logError } from '$lib/log';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import TopBar from '$lib/components/TopBar.svelte';
	import CpuPanel from '$lib/components/CpuPanel.svelte';
	import MemoryPanel from '$lib/components/MemoryPanel.svelte';
	import DiskPanel from '$lib/components/DiskPanel.svelte';
	import NetworkPanel from '$lib/components/NetworkPanel.svelte';
	import ProcessTable from '$lib/components/ProcessTable.svelte';
	import ExportButtons from '$lib/components/ExportButtons.svelte';
	import DetachedHeader from '$lib/components/DetachedHeader.svelte';
	import TrayPopup from '$lib/components/TrayPopup.svelte';
	import ErrorBanner from '$lib/components/ErrorBanner.svelte';
	import type { DetachableView } from '$lib/types';

	let windowWidth = 1200;
	let mode: 'dashboard' | 'detached' | 'tray' = 'dashboard';
	let detachedView: DetachableView = 'cpu';
	let unlistenMerge: (() => void) | null = null;

	function handleResize() {
		windowWidth = window.innerWidth;
		if (windowWidth < 800) {
			sidebarCollapsed.set(true);
		}
	}

	onMount(async () => {
		const params = new URLSearchParams(window.location.search);

		if (params.get('tray') === 'true') {
			mode = 'tray';
			document.documentElement.classList.add('tray-mode');
			// No initSystemListener() — TrayPopup manages its own refresh
			return;
		}

		if (params.get('detached') === 'true') {
			mode = 'detached';
			detachedView = (params.get('view') as DetachableView) || 'cpu';
			await initSystemListener();
			return;
		}

		// Dashboard mode
		await loadConfig();
		await initSystemListener();
		windowWidth = window.innerWidth;
		if (windowWidth < 800) {
			sidebarCollapsed.set(true);
		}
		window.addEventListener('resize', handleResize);

		// Listen for merge-back events from detached windows
		try {
			unlistenMerge = await listen<{ view: string }>('merge-back', (event) => {
				const view = event.payload.view;
				if (['cpu', 'memory', 'disks', 'network', 'processes'].includes(view)) {
					activeView.set(view as DetachableView);
				}
			});
		} catch (e) {
			logError('Failed to listen for merge-back events', e);
		}
	});

	onDestroy(() => {
		destroySystemListener();
		if (typeof window !== 'undefined') {
			window.removeEventListener('resize', handleResize);
		}
		if (unlistenMerge) {
			unlistenMerge();
			unlistenMerge = null;
		}
	});
</script>

{#if mode === 'tray'}
	<TrayPopup />
{:else if mode === 'detached'}
	<div class="detached-layout">
		<DetachedHeader view={detachedView} />
		<div class="detached-content">
			{#if detachedView === 'cpu'}
				<CpuPanel showDetachButton={false} />
			{:else if detachedView === 'memory'}
				<MemoryPanel showDetachButton={false} />
			{:else if detachedView === 'disks'}
				<DiskPanel showDetachButton={false} />
			{:else if detachedView === 'network'}
				<NetworkPanel showDetachButton={false} />
			{:else if detachedView === 'processes'}
				<ProcessTable showDetachButton={false} />
			{/if}
		</div>
	</div>
{:else}
	<div class="layout">
		<Sidebar />
		<TopBar />

		<main class="content">
			<ErrorBanner />
			{#if $activeView === 'overview'}
				<div class="grid-2col">
					<CpuPanel />
					<MemoryPanel />
				</div>
				<div class="grid-2col">
					<DiskPanel />
					<NetworkPanel />
				</div>
				<ProcessTable constrainHeight={false} />
				<div class="export-row">
					<ExportButtons />
				</div>
			{:else if $activeView === 'cpu'}
				<CpuPanel />
			{:else if $activeView === 'memory'}
				<MemoryPanel />
			{:else if $activeView === 'disks'}
				<DiskPanel />
			{:else if $activeView === 'network'}
				<NetworkPanel />
			{:else if $activeView === 'processes'}
				<ProcessTable />
				<div class="export-row">
					<ExportButtons />
				</div>
			{/if}
		</main>
	</div>
{/if}

<style>
	.layout {
		display: flex;
		height: 100vh;
		overflow: hidden;
		position: relative;
		z-index: 1;
	}
	.content {
		flex: 1;
		padding: 64px 16px 20px;
		overflow-y: auto;
		overflow-x: hidden;
		display: flex;
		flex-direction: column;
		gap: 14px;
		min-width: 0;
	}
	.grid-2col {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
		gap: 14px;
	}
	.export-row {
		display: flex;
		justify-content: flex-end;
		padding-top: 2px;
	}

	/* Detached window layout — panel fills the entire window */
	.detached-layout {
		display: flex;
		flex-direction: column;
		height: 100vh;
		position: relative;
		z-index: 1;
	}
	.detached-content {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow-y: auto;
		overflow-x: hidden;
		min-height: 0;
	}
	/* Make the panel inside fill all available space */
	.detached-content > :global(.panel) {
		flex: 1;
		border-radius: 0;
		min-height: 0;
	}
</style>
