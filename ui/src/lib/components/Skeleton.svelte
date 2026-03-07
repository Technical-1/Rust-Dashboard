<script lang="ts">
	export let width: string = '100%';
	export let height: string = '12px';
	export let borderRadius: string = '4px';
</script>

<div class="skeleton" style="width: {width}; height: {height}; border-radius: {borderRadius}"></div>

<style>
	.skeleton {
		background: var(--bg-progress);
		animation: skeleton-pulse 1.5s ease-in-out infinite;
	}
	@keyframes skeleton-pulse {
		0%, 100% { opacity: 0.4; }
		50% { opacity: 1; }
	}
</style>
