export interface SystemSnapshot {
	cpu_usage: number;
	per_cpu: number[];
	memory: MemoryInfo;
	disks: DiskInfo[];
	networks: NetworkInfo[];
	processes: CombinedProcess[];
	self_usage: SelfUsage | null;
	uptime_seconds: number;
	load_average: [number, number, number];
}

export interface SelfUsage {
	cpu: number;
	memory: number;
}

export interface MemoryInfo {
	used: number;
	free: number;
	total: number;
	available: number;
	swap_used: number;
	swap_total: number;
}

export interface DiskInfo {
	name: string;
	filesystem: string;
	mount_point: string;
	used: number;
	available: number;
	total: number;
}

export interface NetworkInfo {
	interface: string;
	rx_bytes: number;
	tx_bytes: number;
	rx_rate: number;
	tx_rate: number;
}

export interface CombinedProcess {
	name: string;
	cpu_usage: number;
	memory_usage: number;
	pids: number[];
}

export interface ProcessDetails {
	command: string;
	start_time: number;
	parent: number | null;
}

export interface AppConfig {
	refresh_interval_seconds: number;
	theme: string;
	window_width: number | null;
	window_height: number | null;
	window_x: number | null;
	window_y: number | null;
}

export type SortColumn = 'name' | 'cpu' | 'memory' | 'pids';
export type SortDirection = 'asc' | 'desc';
export type ActiveView = 'overview' | 'cpu' | 'memory' | 'disks' | 'network' | 'processes';
export type DetachableView = 'cpu' | 'memory' | 'disks' | 'network' | 'processes';
