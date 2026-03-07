import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type { ActiveView, AppConfig } from '$lib/types';
import { logError } from '$lib/log';

export const refreshInterval = writable<number>(2);
export const paused = writable<boolean>(false);
export const theme = writable<'Dark' | 'Light'>('Light');
export const activeView = writable<ActiveView>('overview');
export const sidebarCollapsed = writable<boolean>(false);

export async function loadConfig() {
	try {
		const config = await invoke<AppConfig>('load_config');
		refreshInterval.set(config.refresh_interval_seconds);
		theme.set(config.theme === 'Light' ? 'Light' : 'Dark');
	} catch (e) {
		logError('Failed to load config', e);
	}
}

export async function updateRefreshInterval(seconds: number) {
	refreshInterval.set(seconds);
	try {
		await invoke('set_refresh_interval', { seconds });
	} catch (e) {
		logError('Failed to set refresh interval', e);
	}
}

export async function togglePause(value: boolean) {
	paused.set(value);
	try {
		await invoke('set_paused', { paused: value });
	} catch (e) {
		logError('Failed to toggle pause', e);
	}
}

export async function saveCurrentConfig() {
	const config: AppConfig = {
		refresh_interval_seconds: get(refreshInterval),
		theme: get(theme),
		window_width: null,
		window_height: null,
		window_x: null,
		window_y: null
	};
	try {
		await invoke('save_config', { config });
	} catch (e) {
		logError('Failed to save config', e);
	}
}
