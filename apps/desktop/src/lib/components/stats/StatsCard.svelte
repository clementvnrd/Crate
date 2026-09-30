<script lang="ts">
	import type { Snippet } from 'svelte'
	import { Icon } from '$lib/components/common'

	type Props = {
		/** Id of the heading, so the card is a named region. */
		headingId: string
		title: string
		subtitle?: string
		icon: string
		/** Controls on the right of the header (period selector, arrows). */
		actions?: Snippet
		class?: string
		children: Snippet
	}

	let { headingId, title, subtitle, icon, actions, class: className = '', children }: Props = $props()

	// The Pulse neon-glass card, exactly as DESIGN.md ("C. Pulse — glass dashboard") writes the recipe for the
	// top-level cards of Pulse: rounded-xl, border-stroke/60, bg-surface-1/70, shadow-lg, backdrop-blur-xl. The
	// newer Pulse cards (recap, funnel, sets) share it from here, so the recipe is written once.
	// design-scan-ignore: sanctioned Pulse glass recipe (rounded-xl, backdrop-blur-xl), scoped to components/stats.
	const GLASS = 'rounded-xl border border-stroke/60 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl'
</script>

<section aria-labelledby={headingId} class="flex min-w-0 flex-col {GLASS} {className}">
	<div class="mb-4 flex flex-wrap items-center justify-between gap-3">
		<div class="flex min-w-0 items-center gap-2">
			<div
				class="flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-lg bg-brand-primary/15 text-brand-primary"
			>
				<Icon name={icon} class="h-4 w-4" />
			</div>
			<div class="min-w-0">
				<h3 id={headingId} class="truncate text-sm font-bold text-text-primary">{title}</h3>
				{#if subtitle}
					<p class="truncate text-xs text-text-secondary">{subtitle}</p>
				{/if}
			</div>
		</div>
		{#if actions}
			<div class="flex flex-wrap items-center gap-2">
				{@render actions()}
			</div>
		{/if}
	</div>
	{@render children()}
</section>
