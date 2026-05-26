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

			// Append to histories (cap at 300). Return a fresh array each
			// update — mutating and returning the same reference still
			// notifies subscribers, but breaks any downstream consumer
			// that uses `===` to detect change (memoization, computed
			// stores, signal-style integrations).
			cpuHistory.update((hist) => {
				const next: [number, number][] = [
					...hist,
					[performance.now() / 1000, snapshot.cpu_usage]
				];
				return next.length > 300 ? next.slice(-300) : next;
			});

			memoryHistory.update((hist) => {
				const usedGb = snapshot.memory.used / 1024 / 1024 / 1024;
				const next: [number, number][] = [...hist, [performance.now() / 1000, usedGb]];
				return next.length > 300 ? next.slice(-300) : next;
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
