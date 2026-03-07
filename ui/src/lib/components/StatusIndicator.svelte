<script lang="ts">
	import { getStatusColor } from '$lib/utils';

	export let value: number = 0;
	export let low: number = 50;
	export let high: number = 80;
</script>

<span class="status-dot" style="background: {getStatusColor(value, low, high)}"></span>

<style>
	.status-dot {
		display: inline-block;
		width: 7px;
		height: 7px;
		border-radius: 50%;
		flex-shrink: 0;
	}
</style>
