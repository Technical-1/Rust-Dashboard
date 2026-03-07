import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { DetachableView } from '$lib/types';

const VIEW_TITLES: Record<DetachableView, string> = {
	cpu: 'CPU Monitor',
	memory: 'Memory Monitor',
	disks: 'Disk Monitor',
	network: 'Network Monitor',
	processes: 'Process Monitor'
};

export async function openDetachedWindow(view: DetachableView) {
	const label = `detached-${view}`;

	// Focus existing window if already open
	const existing = await WebviewWindow.getByLabel(label);
	if (existing) {
		await existing.setFocus();
		return;
	}

	new WebviewWindow(label, {
		url: `/?view=${view}&detached=true`,
		title: VIEW_TITLES[view],
		width: 600,
		height: 500,
		minWidth: 400,
		minHeight: 300,
		decorations: true,
		resizable: true,
		alwaysOnTop: false
	});
}
