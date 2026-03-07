<script lang="ts">
	export let value: number = 0; // 0-1
	export let color: string = 'var(--accent)';
	export let label: string = 'Progress';
</script>

<div class="progress-track" role="progressbar" aria-valuenow={Math.round(value * 100)} aria-valuemin={0} aria-valuemax={100} aria-label={label}>
	<div
		class="progress-fill"
		style="width: {Math.min(Math.max(value, 0), 1) * 100}%; background: {color}"
	></div>
</div>

<style>
	.progress-track {
		width: 100%;
		height: 4px;
		background: var(--bg-progress);
		border-radius: 2px;
		overflow: hidden;
	}
	.progress-fill {
		height: 100%;
		border-radius: 2px;
		transition: width 0.6s var(--ease-out);
		min-width: 0;
	}
</style>
