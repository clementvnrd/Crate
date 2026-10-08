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
	import { untrack } from 'svelte'
	import { translate } from '$shared/i18n'
	import Icon from './Icon.svelte'
	import IconButton from './IconButton.svelte'
	import {
		focusableIndex,
		indexForKey,
		revealScrollLeft,
		scrollEdges,
		selectedIndex,
		stepScrollLeft,
	} from './segmented'

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
		/**
		 * `boxed` and `deck` only: the options stay on one line inside whatever width the parent leaves. When they do
		 * not fit, the control scrolls horizontally, an arrow and an edge fade appear on each side that hides options,
		 * and the selected option is always scrolled into view (DESIGN.md "Scroll affordance"). Without overflow it
		 * looks exactly like the plain control. The parent must let it shrink (`min-w-0` up the flex chain).
		 */
		scrollable?: boolean
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
		scrollable = false,
	}: Props = $props()

	// The view switcher's equal columns and sliding thumb cannot scroll; it ignores `scrollable`.
	const isScrollable = $derived(scrollable && variant !== 'switcher')

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
		// A modified arrow is a global shortcut (Shift+→ next track, Cmd+→ fine seek), not a move in the group.
		if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return
		const next = indexForKey(event.key, active >= 0 ? active : index, options.length)
		if (next === null) return
		event.preventDefault()
		// The key belongs to the control: the global arrow shortcuts (seek, volume) must not also run.
		event.stopPropagation()
		select(next)
		// A scrollable control brings the option into view itself, clear of the arrows (see `revealSelected`).
		buttons[next]?.focus({ preventScroll: isScrollable })
	}

	// --- Scrollable mode -------------------------------------------------------------------------------------------

	/** Width kept clear at an edge that hides options: the arrow (`w-6`) plus the fade, as in style.css
	 *  (`.scroll-affordance`: `--scroll-arrow` 1.5rem + `--scroll-fade` 1.25rem). */
	const EDGE_INSET = 44

	let scroller: HTMLDivElement | undefined = $state()
	let edges = $state({ start: false, end: false })

	function measure() {
		if (!scroller) return
		const next = scrollEdges(scroller.scrollLeft, scroller.scrollWidth, scroller.clientWidth)
		if (next.start !== edges.start || next.end !== edges.end) edges = next
	}

	function scrollBehavior(): ScrollBehavior {
		return window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth'
	}

	function view(el: HTMLDivElement) {
		return { scrollLeft: el.scrollLeft, clientWidth: el.clientWidth, scrollWidth: el.scrollWidth }
	}

	function revealSelected(behavior: ScrollBehavior) {
		const button = active >= 0 ? buttons[active] : undefined
		if (!scroller || !button) return
		const left = revealScrollLeft({ start: button.offsetLeft, width: button.offsetWidth }, view(scroller), EDGE_INSET)
		if (Math.abs(left - scroller.scrollLeft) > 0.5) scroller.scrollTo({ left, behavior })
	}

	function scrollByPage(direction: -1 | 1) {
		if (!scroller) return
		scroller.scrollTo({ left: stepScrollLeft(direction, view(scroller), EDGE_INSET), behavior: scrollBehavior() })
	}

	// A new selection (click, arrow keys, or set from outside) scrolls into view; instantly the first time. Declared
	// before the resize effect so a selection that changes together with the options keeps its smooth scroll.
	let revealedOnce = false
	$effect(() => {
		if (!isScrollable || !scroller) return
		void active
		untrack(() => {
			revealSelected(revealedOnce ? scrollBehavior() : 'auto')
			revealedOnce = true
		})
	})

	// Size changes: any segment or the scroller resizing re-measures the edges; the scroller itself changing width
	// (window, header) also brings the selection back into view. A segment resizing alone (the selected one gains a
	// border) must not interrupt the smooth scroll of a new selection. Runs again when the scroller or the options
	// change (new labels after a language switch), and then reveals the selection at once.
	$effect(() => {
		if (!isScrollable || !scroller) return
		const el = scroller
		void options
		let width = el.clientWidth
		const observer = new ResizeObserver(() => {
			measure()
			if (el.clientWidth === width) return
			width = el.clientWidth
			revealSelected('auto')
		})
		untrack(() => {
			observer.observe(el)
			for (const segment of el.children) observer.observe(segment)
			measure()
			revealSelected('auto')
		})
		return () => observer.disconnect()
	})

	// Class strings copied verbatim from the controls these variants replace.
	const containerStyles: Record<SegmentedVariant, string> = {
		switcher: 'relative inline-grid auto-cols-fr grid-flow-col items-center rounded-lg bg-surface-2 p-0.5',
		deck: 'inline-flex items-center rounded-full border border-stroke-subtle p-0.5 shadow-inner',
		boxed: 'flex items-center rounded-xl border border-stroke bg-surface-2 p-1 shadow-xs',
	}

	const switcherLabel: Record<SegmentLabelTone, string> = {
		default: 'text-text-primary',
		deck: 'text-deck-live-text',
		beatport: 'text-beatport-text',
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
					selected ? 'bg-deck-live-tint font-bold text-black shadow-xs' : 'text-text-secondary hover:text-text-primary'
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

{#snippet segments()}
	{#each options as option, index (option.value)}
		<button
			bind:this={buttons[index]}
			type="button"
			role="radio"
			aria-checked={index === active}
			tabindex={index === tabStop ? 0 : -1}
			class="{segmentClass(index, option.labelTone)}{isScrollable ? ' shrink-0 whitespace-nowrap' : ''}"
			onclick={() => select(index)}
			onkeydown={(event) => handleKeydown(event, index)}
		>
			{#if option.icon}
				<Icon name={option.icon} class={iconClass(index, option.labelTone)} />
			{/if}
			<span>{option.label}</span>
		</button>
	{/each}
{/snippet}

{#if isScrollable}
	<!-- The box keeps the variant's look; the scroller inside spans its padding (negative margin), so the selected
	     segment's shadow and focus outline are not clipped. The arrows sit over the transparent part of the fade. -->
	<div class="{containerStyles[variant]} relative max-w-full min-w-0 {className}">
		<div
			bind:this={scroller}
			{id}
			role="radiogroup"
			aria-label={ariaLabel}
			class="scroll-affordance relative flex min-w-0 items-center overflow-x-auto {variant === 'deck'
				? '-m-0.5 p-0.5'
				: '-m-1 p-1'}"
			data-fade-start={edges.start || undefined}
			data-fade-end={edges.end || undefined}
			onscroll={measure}
		>
			{@render segments()}
		</div>
		{#if edges.start}
			<IconButton
				icon="chevron-left"
				size="sm"
				tabindex={-1}
				iconClass="h-3.5 w-3.5"
				class="absolute top-1/2 left-0 -translate-y-1/2"
				title={$translate('common.scrollBackward')}
				ariaLabel={$translate('common.scrollBackward')}
				onclick={() => scrollByPage(-1)}
			/>
		{/if}
		{#if edges.end}
			<IconButton
				icon="chevron-right"
				size="sm"
				tabindex={-1}
				iconClass="h-3.5 w-3.5"
				class="absolute top-1/2 right-0 -translate-y-1/2"
				title={$translate('common.scrollForward')}
				ariaLabel={$translate('common.scrollForward')}
				onclick={() => scrollByPage(1)}
			/>
		{/if}
	</div>
{:else}
	<div {id} role="radiogroup" aria-label={ariaLabel} class="{containerStyles[variant]} {className}">
		{#if thumbStyle}
			<div
				aria-hidden="true"
				class="absolute top-0.5 bottom-0.5 left-0.5 rounded-md bg-surface-0 shadow-sm transition-transform duration-200 ease-out motion-reduce:transition-none"
				style={thumbStyle}
			></div>
		{/if}

		{@render segments()}
	</div>
{/if}
