const KB = 1024;
const MB = KB * 1024;
const GB = MB * 1024;
const TB = GB * 1024;

export function formatBytes(bytes: number): string {
	if (bytes >= TB) return `${(bytes / TB).toFixed(2)} TB`;
	if (bytes >= GB) return `${(bytes / GB).toFixed(2)} GB`;
	if (bytes >= MB) return `${(bytes / MB).toFixed(2)} MB`;
	if (bytes >= KB) return `${(bytes / KB).toFixed(2)} KB`;
	return `${bytes} B`;
}

export function formatGiB(bytes: number): string {
	return `${(bytes / GB).toFixed(2)} GiB`;
}

export function getStatusColor(value: number, low = 50, high = 80): string {
	if (value < low) return 'var(--green)';
	if (value < high) return 'var(--yellow)';
	return 'var(--red)';
}

export function getStatusBg(value: number, low = 50, high = 80): string {
	if (value < low) return 'var(--green-subtle)';
	if (value < high) return 'var(--yellow-subtle)';
	return 'var(--red-subtle)';
}

export function formatBytesPerSec(bytesPerSec: number): string {
	if (bytesPerSec >= GB) return `${(bytesPerSec / GB).toFixed(2)} GB/s`;
	if (bytesPerSec >= MB) return `${(bytesPerSec / MB).toFixed(2)} MB/s`;
	if (bytesPerSec >= KB) return `${(bytesPerSec / KB).toFixed(2)} KB/s`;
	return `${bytesPerSec.toFixed(0)} B/s`;
}

export function formatUptime(seconds: number): string {
	const days = Math.floor(seconds / 86400);
	const hours = Math.floor((seconds % 86400) / 3600);
	const mins = Math.floor((seconds % 3600) / 60);
	if (days > 0) return `${days}d ${hours}h`;
	if (hours > 0) return `${hours}h ${mins}m`;
	return `${mins}m`;
}
