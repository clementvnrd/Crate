<script lang="ts" module>
	/**
	 * The looks the Camelot key badge has in each view today. The owner froze the visible look (CRA-141), so the
	 * badge reproduces each one exactly instead of imposing a single size; merging them is a later decision.
	 * - `cell`: library table (with the Mixed In Key ring and the neutral "not analysed" badge)
	 * - `cell-compact`: Beatport table
	 * - `chip`: Duplicate Killer · `chip-plain`: Beatport Upgrader
	 * - `tag`: Pulse top tracks · `tag-wide`: Pulse harmonic wheel
	 * - `pill`: Player hero · `pill-xs`: Player recent files and album detail rows
	 */
	export type KeyBadgeVariant =
		| 'cell'
		| 'cell-compact'
		| 'chip'
		| 'chip-plain'
		| 'tag'
		| 'tag-wide'
		| 'pill'
		| 'pill-xs'
</script>

<script lang="ts">
	import { translate } from '$shared/i18n'
	import { keyBadgeAppearance, type KeyAnalysis } from './keyBadge'

	type Props = {
		/** The key, in Camelot ("8A") or standard ("Am") notation. */
		value: string | null | undefined
		variant: KeyBadgeVariant
		/** Text to show instead of the Camelot notation (the user's key notation setting). */
		label?: string
		zeroPad?: boolean
		/** `mik`: coloured, with the Mixed In Key ring. `other`: neutral. Omitted: coloured when the key is known. */
		analysis?: KeyAnalysis
	}

	let { value, variant, label, zeroPad = false, analysis }: Props = $props()

	type Fallback = { kind: 'box'; class: string; style?: string } | { kind: 'text'; class: string } | { kind: 'none' }

	type Spec = {
		class: string
		bordered: boolean
		/** Rendering when the key is not coloured (unknown key, or not analysed by Mixed In Key). */
		fallback: Fallback
		/** Rendering when there is no key; `none` where the view hides the badge itself. */
		empty: 'dash' | 'fallback' | 'none'
		/** Hover title: `library` (Mixed In Key or not), `source` (name and source), `name`, or none. */
		title: 'library' | 'source' | 'name' | 'none'
		source?: string
	}

	// Copied verbatim from the views they come from (see the variant list above).
	const SPECS: Record<KeyBadgeVariant, Spec> = {
		cell: {
			class:
				'relative inline-flex h-[22px] w-11 items-center justify-center rounded font-mono text-[11px] font-bold tracking-tight shadow-sm select-none',
			bordered: false,
			fallback: {
				kind: 'box',
				class:
					'relative inline-flex h-[22px] w-11 items-center justify-center rounded border border-stroke bg-surface-3 font-mono text-[11px] font-medium tracking-tight text-text-secondary select-none',
			},
			empty: 'dash',
			title: 'library',
		},
		'cell-compact': {
			class:
				'relative inline-flex h-[20px] w-10 items-center justify-center rounded font-mono text-[11px] font-bold tracking-tight shadow-sm select-none',
			bordered: false,
			fallback: { kind: 'text', class: 'font-mono text-[11px] text-text-tertiary' },
			empty: 'fallback',
			title: 'source',
			source: 'Beatport / Camelot',
		},
		chip: {
			class:
				'inline-flex h-[20px] min-w-[32px] items-center justify-center rounded px-1.5 font-mono text-[10px] font-bold tracking-tight shadow-sm select-none',
			bordered: false,
			fallback: {
				kind: 'box',
				class:
					'inline-flex h-[20px] items-center justify-center rounded border border-stroke/50 bg-surface-3 px-1.5 font-mono text-[10px] text-text-secondary select-none',
			},
			empty: 'none',
			title: 'source',
			source: 'Camelot Key',
		},
		'chip-plain': {
			class:
				'inline-flex h-[20px] min-w-[30px] items-center justify-center rounded px-1.5 font-mono text-[10px] font-bold',
			bordered: false,
			fallback: { kind: 'none' },
			empty: 'none',
			title: 'none',
		},
		tag: {
			class: 'rounded border px-1.5 py-0.5 font-mono text-[10px] font-bold shadow-xs',
			bordered: true,
			fallback: { kind: 'box', class: 'rounded border px-1.5 py-0.5 font-mono text-[10px] font-bold shadow-xs' },
			empty: 'none',
			title: 'none',
		},
		'tag-wide': {
			class: 'min-w-[36px] rounded border px-2 py-0.5 text-center font-mono text-xs font-bold shadow-xs',
			bordered: true,
			fallback: {
				kind: 'box',
				class: 'min-w-[36px] rounded border px-2 py-0.5 text-center font-mono text-xs font-bold shadow-xs',
				style: 'background-color: #3b82f6; color: white;',
			},
			empty: 'none',
			title: 'none',
		},
		pill: {
			class:
				'inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 text-[10px] font-extrabold shadow-xs select-none',
			bordered: true,
			fallback: {
				kind: 'box',
				class:
					'rounded-full border border-stroke-subtle bg-surface-2 px-2.5 py-0.5 font-mono text-[10px] font-bold text-text-primary',
			},
			empty: 'none',
			title: 'name',
		},
		'pill-xs': {
			class: 'py-0.2 rounded-full border px-1.5 text-[9px] font-extrabold shadow-xs',
			bordered: true,
			fallback: { kind: 'text', class: 'font-mono text-[10px] text-text-secondary' },
			empty: 'none',
			title: 'none',
		},
	}

	const spec = $derived(SPECS[variant])
	const look = $derived(keyBadgeAppearance(value, { label, zeroPad, analysis, bordered: spec.bordered }))

	const title = $derived.by(() => {
		if (spec.title === 'library') {
			return look.mikRing && look.name
				? $translate('badges.key.titleMik', { values: { key: look.label, name: look.name } })
				: $translate('badges.key.titleNotMik', { values: { key: look.label } })
		}
		if (!look.style || !look.name) return undefined
		if (spec.title === 'source') {
			return $translate('badges.key.titleSource', {
				values: { key: look.label, name: look.name, source: spec.source ?? '' },
			})
		}
		if (spec.title === 'name') return $translate('badges.key.titleName', { values: { name: look.name } })
		return undefined
	})
</script>

{#snippet fallback()}
	{#if spec.fallback.kind === 'box'}
		<span class={spec.fallback.class} style={spec.fallback.style} {title}>{look.label}</span>
	{:else if spec.fallback.kind === 'text'}
		<span class={spec.fallback.class}>{look.label}</span>
	{/if}
{/snippet}

{#if look.empty}
	{#if spec.empty === 'dash'}
		<span class="text-xs text-text-tertiary">-</span>
	{:else if spec.empty === 'fallback'}
		{@render fallback()}
	{/if}
{:else if look.style}
	<span class={spec.class} style={look.style} {title}>
		{look.label}
		{#if look.mikRing}
			<img
				src="/mik-ring.png"
				alt=""
				aria-hidden="true"
				class="pointer-events-none absolute -top-1.5 -right-1.5 h-3.5 w-3.5 object-contain drop-shadow-[0_0_3px_rgba(56,189,248,0.8)] select-none"
			/>
		{/if}
	</span>
{:else}
	{@render fallback()}
{/if}
