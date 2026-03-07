<script lang="ts">
	import { systemError } from '$lib/stores/system';

	function dismiss() {
		systemError.set(null);
	}
</script>

{#if $systemError}
	<div class="error-banner" role="alert">
		<svg class="error-icon" viewBox="0 0 16 16" fill="none">
			<circle cx="8" cy="8" r="6.5" stroke="currentColor" stroke-width="1.2"/>
			<line x1="8" y1="5" x2="8" y2="9" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
			<circle cx="8" cy="11.5" r="0.75" fill="currentColor"/>
		</svg>
		<span class="error-text">{$systemError}</span>
		<button class="dismiss-btn" on:click={dismiss} aria-label="Dismiss error">
			<svg viewBox="0 0 10 10" fill="none">
				<path d="M2 2L8 8M8 2L2 8" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
			</svg>
		</button>
	</div>
{/if}

<style>
	.error-banner {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		background: var(--red-subtle);
		border: 0.5px solid var(--red);
		border-radius: var(--radius-m);
		animation: slide-in 200ms var(--ease-out);
	}
	.error-icon {
		width: 16px;
		height: 16px;
		color: var(--red);
		flex-shrink: 0;
	}
	.error-text {
		flex: 1;
		font-size: 12px;
		color: var(--text-primary);
	}
	.dismiss-btn {
		width: 20px;
		height: 20px;
		border: none;
		border-radius: 4px;
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}
	.dismiss-btn:hover {
		background: var(--bg-sidebar-hover);
		color: var(--text-primary);
	}
	.dismiss-btn svg {
		width: 10px;
		height: 10px;
	}
	@keyframes slide-in {
		from { opacity: 0; transform: translateY(-4px); }
		to { opacity: 1; transform: translateY(0); }
	}
</style>
