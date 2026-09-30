<script lang="ts" module>
	/**
	 * Colour of a segment's label when it is selected (view switcher only). `deck` and `beatport` are the only
	 * family colours a shell entry point carries.
	 */
	export type SegmentLabelTone = 'default' | 'deck' | 'beatport'

	export type SegmentOption<T extends string> = {
		value: T
		label: string
		icon?: string
		labelTone?: SegmentLabelTone
	}

	/**
	 * The looks the segmented controls have today (CRA-141 froze them):
	 * - `switcher`: the toolbar's view switcher, equal widths and a sliding raised thumb
	 * - `deck`: the Player's Recent / Album mode pill, the selected segment filled in cyan (no thumb);
	 *   the container's background comes from `class`
	 * - `boxed`: Pulse's period selector, the selected segment raised with a border (no thumb)
	 */
	export type SegmentedVariant = 'switcher' | 'deck' | 'boxed'
</script>

<script lang="ts" generics="T extends string">
	import Icon from './Icon.svelte'
	import { focusableIndex, indexForKey, selectedIndex } from './segmented'

	type Props = {
		options: SegmentOption<T>[]
		/** The selected value; a value that matches no option selects nothing. */
		value: T | null | undefined
		onchange: (value: T) => void
		/** Accessible name of the group (translated). */
		ariaLabel: string
		variant: SegmentedVariant
		id?: string
		class?: string
		/**
		 * `boxed` only: colour of the labels that are not selected. `tertiary` (default) is the look of Pulse's period
		 * selector today; `secondary` reads at 4.5:1 in both themes, for new controls (the recap's week/year switch).
		 */
		unselectedTone?: 'tertiary' | 'secondary'
	}

	let {
		options,
		value,
		onchange,
		ariaLabel,
		variant,
		id,
		class: className = '',
		unselectedTone = 'tertiary',
	}: Props = $props()

	const active = $derived(
		selectedIndex(
			options.map((o) => o.value),
			value
		)
	)
	const tabStop = $derived(focusableIndex(active, options.length))

	const buttons: HTMLButtonElement[] = $state([])

	// The switcher's sliding thumb, with the geometry the view switcher always had: equal columns, so the thumb
	// is one column wide (minus the 2 px padding) and moves by whole widths. Hidden when nothing is selected.
	const thumbStyle = $derived(
		variant === 'switcher' && active >= 0 && options.length > 0
			? `width: calc(${(100 / options.length).toFixed(3)}% - 2px); transform: translateX(${active * 100}%)`
			: null
	)

	function select(index: number) {
		const option = options[index]
		if (option) onchange(option.value)
	}

	function handleKeydown(event: KeyboardEvent, index: number) {
		const next = indexForKey(event.key, active >= 0 ? active : index, options.length)
		if (next === null) return
		event.preventDefault()
		select(next)
		buttons[next]?.focus()
	}

	// Class strings copied verbatim from the controls these variants replace.
	const containerStyles: Record<SegmentedVariant, string> = {
		switcher: 'relative inline-grid auto-cols-fr grid-flow-col items-center rounded-lg bg-surface-2 p-0.5',
		deck: 'inline-flex items-center rounded-full border border-stroke-subtle p-0.5 shadow-inner',
		boxed: 'flex items-center rounded-xl border border-stroke bg-surface-2 p-1 shadow-xs',
	}

	const switcherLabel: Record<SegmentLabelTone, string> = {
		default: 'text-text-primary',
		deck: 'text-cyan-400',
		beatport: 'text-emerald-500 dark:text-emerald-400',
	}

	function segmentClass(index: number, labelTone: SegmentLabelTone = 'default'): string {
		const selected = index === active
		switch (variant) {
			case 'switcher':
				return `relative z-10 flex items-center justify-center gap-1.5 rounded-md px-3 py-1 text-center text-xs font-medium transition-colors ${
					selected
						? `font-semibold ${switcherLabel[labelTone]}`
						: 'text-text-tertiary hover:cursor-pointer hover:text-text-secondary'
				}`
			case 'deck':
				return `flex cursor-pointer items-center gap-1.5 rounded-full px-3 py-1 text-xs font-semibold transition-all ${
					selected ? 'bg-cyan-500 font-bold text-black shadow-xs' : 'text-text-secondary hover:text-text-primary'
				}`
			case 'boxed':
				return `cursor-pointer rounded-lg px-3 py-1 text-xs font-semibold transition-all ${
					selected
						? 'border border-stroke-strong/60 bg-surface-0 text-text-primary shadow-sm'
						: unselectedTone === 'secondary'
							? 'text-text-secondary hover:text-text-primary'
							: 'text-text-tertiary hover:text-text-secondary'
				}`
		}
	}

	function iconClass(index: number, labelTone: SegmentLabelTone = 'default'): string {
		// The view switcher colours its icons explicitly (they stay tertiary on hover); the others inherit.
		if (variant !== 'switcher') return 'h-3 w-3'
		return `h-3 w-3 ${index === active ? switcherLabel[labelTone] : 'text-text-tertiary'}`
	}
</script>

<div {id} role="radiogroup" aria-label={ariaLabel} class="{containerStyles[variant]} {className}">
	{#if thumbStyle}
		<div
			aria-hidden="true"
			class="absolute top-0.5 bottom-0.5 left-0.5 rounded-md bg-surface-0 shadow-sm transition-transform duration-200 ease-out motion-reduce:transition-none"
			style={thumbStyle}
		></div>
	{/if}

	{#each options as option, index (option.value)}
		<button
			bind:this={buttons[index]}
			type="button"
			role="radio"
			aria-checked={index === active}
			tabindex={index === tabStop ? 0 : -1}
			class={segmentClass(index, option.labelTone)}
			onclick={() => select(index)}
			onkeydown={(event) => handleKeydown(event, index)}
		>
			{#if option.icon}
				<Icon name={option.icon} class={iconClass(index, option.labelTone)} />
			{/if}
			<span>{option.label}</span>
		</button>
	{/each}
</div>
