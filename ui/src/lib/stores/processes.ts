import { writable } from 'svelte/store';
import type { SortColumn, SortDirection } from '$lib/types';

export const searchQuery = writable<string>('');
export const cpuThreshold = writable<number>(0);
export const memoryThresholdMB = writable<number>(0);
export const sortColumn = writable<SortColumn>('cpu');
export const sortDirection = writable<SortDirection>('desc');
export const expandedProcesses = writable<Set<string>>(new Set());
