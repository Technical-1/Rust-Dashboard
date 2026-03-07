<script lang="ts">
	import { getStatusColor } from '$lib/utils';

	export let cores: number[] = [];
</script>

<div class="cores-grid">
	{#each cores as usage, i}
		<div class="core-item">
			<span class="core-label mono">{i}</span>
			<div class="core-bar-track">
				<div
					class="core-bar-fill"
					style="width: {Math.min(usage, 100)}%; background: {getStatusColor(usage)}"
				></div>
			</div>
			<span class="core-value mono">{usage.toFixed(0)}%</span>
		</div>
	{/each}
</div>

<style>
	.cores-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
		gap: 4px;
	}
	.core-item {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 10px;
		padding: 2px 4px;
		border-radius: 4px;
		transition: background var(--duration-fast) var(--ease-out);
	}
	.core-item:hover {
		background: var(--bg-table-hover);
	}
	.core-label {
		color: var(--text-tertiary);
		width: 18px;
		text-align: right;
		flex-shrink: 0;
	}
	.core-bar-track {
		flex: 1;
		height: 3px;
		background: var(--bg-progress);
		border-radius: 1.5px;
		overflow: hidden;
	}
	.core-bar-fill {
		height: 100%;
		border-radius: 1.5px;
		transition: width 0.6s var(--ease-out);
	}
	.core-value {
		width: 28px;
		text-align: right;
		color: var(--text-tertiary);
		flex-shrink: 0;
	}
</style>
