<script lang="ts">
	import '../app.css';
	import { theme } from '$lib/stores/config';
	import { onMount } from 'svelte';

	// Apply theme class to <html> so :root selectors work correctly
	function applyTheme(t: 'Dark' | 'Light') {
		if (typeof document === 'undefined') return;
		const root = document.documentElement;
		if (t === 'Dark') {
			root.classList.add('dark');
			root.classList.remove('light');
		} else {
			root.classList.add('light');
			root.classList.remove('dark');
		}
	}

	onMount(() => {
		// Apply initial theme
		applyTheme($theme);
	});

	// React to theme changes
	$: applyTheme($theme);
</script>

<slot />
