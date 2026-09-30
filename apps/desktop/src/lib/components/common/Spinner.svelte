<script lang="ts">
	import Icon from './Icon.svelte'

	type Props = {
		class?: string
		/** `muted` (default): tertiary text colour. `current`: the colour of the text around it (or of `class`). */
		color?: 'muted' | 'current'
		/** Glyph: `refresh` (default) or `loader`, the one the Player views use. */
		icon?: 'refresh' | 'loader'
	}

	let { class: className = 'h-4 w-4', color = 'muted', icon = 'refresh' }: Props = $props()
</script>

<Icon
	name={icon}
	class="{className} animate-spin motion-reduce:animate-none {color === 'muted' ? 'text-text-tertiary' : ''}"
/>
