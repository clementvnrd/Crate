<script lang="ts">
	import Icon from './Icon.svelte'

	type Props = {
		class?: string
		/** `muted` (default): tertiary text colour. `current`: the colour of the text around it (or of `class`). */
		color?: 'muted' | 'current'
		/**
		 * Glyph, each one the look a view already had:
		 * - `refresh` (default), `refresh-cw`, `loader`: an icon of the set that turns;
		 * - `arc`: a faint circle with a darker quarter (cloud sync);
		 * - `ring`: a CSS ring open at the top; its size, border width and colour come from `class`
		 *   (`h-4 w-4 border-2 border-brand-primary`).
		 */
		icon?: 'refresh' | 'refresh-cw' | 'loader' | 'arc' | 'ring'
	}

	let { class: className = 'h-4 w-4', color = 'muted', icon = 'refresh' }: Props = $props()

	const tint = $derived(color === 'muted' ? 'text-text-tertiary' : '')
</script>

{#if icon === 'ring'}
	<div class="{className} animate-spin rounded-full border-t-transparent motion-reduce:animate-none"></div>
{:else if icon === 'arc'}
	<svg
		class="{className} animate-spin motion-reduce:animate-none {tint}"
		fill="none"
		viewBox="0 0 24 24"
		aria-hidden="true"
	>
		<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
		<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
	</svg>
{:else}
	<Icon name={icon} class="{className} animate-spin motion-reduce:animate-none {tint}" />
{/if}
