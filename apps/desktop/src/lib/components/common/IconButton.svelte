<script lang="ts">
	import Icon from './Icon.svelte'
	import Spinner from './Spinner.svelte'

	type Props = {
		title?: string
		/** Accessible name, for an icon button whose visible label is a `Tooltip` (no native `title` bubble). */
		ariaLabel?: string
		/** Toggle state, announced as pressed / not pressed (a shuffle or follow toggle). */
		pressed?: boolean
		disabled?: boolean
		active?: boolean
		size?: 'sm' | 'md' | 'lg'
		class?: string
		icon: string
		iconClass?: string
		fill?: boolean
		/** The icon turns (the common `Spinner`) while an action runs; `icon` must be `refresh`, `refresh-cw` or `loader`. */
		busy?: boolean
		onclick?: (e: MouseEvent) => void
	}

	let {
		title = '',
		ariaLabel,
		pressed,
		disabled = false,
		active = false,
		size = 'md',
		class: className = '',
		icon,
		iconClass = '',
		fill = false,
		busy = false,
		onclick,
	}: Props = $props()

	const sizeStyles = {
		sm: 'w-6 h-6 text-sm',
		md: 'w-8 h-8 text-base',
		lg: 'w-10 h-10 text-lg',
	}
</script>

<button
	type="button"
	title={title || undefined}
	aria-label={ariaLabel || undefined}
	aria-pressed={pressed}
	{disabled}
	class="inline-flex items-center justify-center rounded-md transition-colors focus:ring-2 focus:ring-brand-primary focus:outline-none disabled:cursor-not-allowed disabled:opacity-50 {active
		? 'bg-brand-muted text-brand-primary hover:cursor-pointer'
		: 'text-text-secondary hover:cursor-pointer hover:bg-surface-2 hover:text-text-primary'} {sizeStyles[
		size
	]} {className}"
	{onclick}
	ondblclick={(e) => e.stopPropagation()}
>
	{#if busy}
		<Spinner icon={icon as 'refresh' | 'refresh-cw' | 'loader'} color="current" class={iconClass || 'h-4 w-4'} />
	{:else}
		<Icon name={icon} class={iconClass || undefined} {fill} />
	{/if}
</button>
