import { writable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import type { SystemSnapshot } from '$lib/types';
import { logError } from '$lib/log';

export const systemSnapshot = writable<SystemSnapshot | null>(null);
export const cpuHistory = writable<[number, number][]>([]);
export const memoryHistory = writable<[number, number][]>([]);
export const systemError = writable<string | null>(null);

let unlisten: (() => void) | null = null;
let unlistenError: (() => void) | null = null;

export async function initSystemListener() {
	// Fetch initial data
	try {
		const snapshot = await invoke<SystemSnapshot>('get_system_snapshot');
		systemSnapshot.set(snapshot);
		systemError.set(null);

		const cpuHist = await invoke<[number, number][]>('get_cpu_history');
		cpuHistory.set(cpuHist);

		const memHist = await invoke<[number, number][]>('get_memory_history');
		memoryHistory.set(memHist);
	} catch (e) {
		logError('Failed to fetch initial data', e);
		systemError.set(`Failed to connect to system monitor: ${e}`);
	}

	// Listen for push updates
	try {
		unlisten = await listen<SystemSnapshot>('system-update', (event) => {
			const snapshot = event.payload;
			systemSnapshot.set(snapshot);
			systemError.set(null);

			// Append to histories (cap at 300)
			cpuHistory.update((hist) => {
				const now = performance.now() / 1000;
				hist.push([now, snapshot.cpu_usage]);
				if (hist.length > 300) hist.shift();
				return hist;
			});

			memoryHistory.update((hist) => {
				const now = performance.now() / 1000;
				const usedGb = snapshot.memory.used / 1024 / 1024 / 1024;
				hist.push([now, usedGb]);
				if (hist.length > 300) hist.shift();
				return hist;
			});
		});
	} catch (e) {
		logError('Failed to listen for system updates', e);
		systemError.set('Failed to connect to system event stream');
	}

	// Listen for explicit system-error events from the backend (e.g.
	// mutex-poisoning recovery). The next successful system-update will
	// auto-clear systemError, so the banner disappears once the
	// monitor is healthy again.
	try {
		unlistenError = await listen<string>('system-error', (event) => {
			systemError.set(event.payload);
		});
	} catch (e) {
		logError('Failed to listen for system-error events', e);
	}
}

export function destroySystemListener() {
	if (unlisten) {
		unlisten();
		unlisten = null;
	}
	if (unlistenError) {
		unlistenError();
		unlistenError = null;
	}
}
