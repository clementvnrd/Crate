<script lang="ts">
	import { translate } from '$shared/i18n'
	import { getEnergyInfo } from '$shared/utils/energy'

	type Props = {
		/** Mixed In Key energy level, 1 to 10. */
		energy: number | null | undefined
		/**
		 * The looks the energy badge has today (CRA-141 froze them): `badge` in the library (level symbol, glow),
		 * `pill` in the Player hero (fixed ⚡, no glow).
		 */
		variant?: 'badge' | 'pill'
		/** Size of the `badge` variant. */
		size?: 'sm' | 'md'
		class?: string
	}

	let { energy, variant = 'badge', size = 'md', class: className = '' }: Props = $props()

	// Colours come only from the energy palette (DESIGN.md, Data palettes).
	const info = $derived(getEnergyInfo(energy))
	// The descriptor shown in tooltips comes from i18n (`badges.energy.levels.N`), not from `info.descriptor`.
	const descriptor = $derived(info ? $translate(`badges.energy.levels.${info.level}`) : '')
	// The text takes the palette's light shade in the light theme: `light-dark()` follows `color-scheme`, which each
	// `[data-theme]` block sets, so a badge inside a panel that stays dark (Modal theme="dark") keeps the dark shade.
	const textColor = $derived(info ? `light-dark(${info.lightColor}, ${info.color})` : '')
</script>

{#if info && variant === 'pill'}
	<span
		class="inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 text-[10px] font-bold shadow-xs {className}"
		style="background-color: {info.bg}; color: {textColor}; border-color: {info.border};"
		title={$translate('badges.energy.titlePlayer', { values: { descriptor, level: energy } })}
	>
		<span aria-hidden="true">⚡</span>
		<span>{energy}</span>
	</span>
{:else if info}
	<span
		class="inline-flex items-center justify-center gap-1 rounded border font-bold tracking-tight select-none {size ===
		'sm'
			? 'px-1.5 py-0.5 text-[10px]'
			: 'min-w-[46px] px-2 py-0.5 text-[11px]'} {className}"
		style="background-color: {info.bg}; color: {textColor}; border-color: {info.border}; box-shadow: {info.glow};"
		title={$translate('badges.energy.title', { values: { level: info.level, descriptor } })}
	>
		<span class="text-xs leading-none" aria-hidden="true">{info.symbol}</span>
		<span class="font-extrabold tabular-nums">{info.level}</span>
	</span>
{:else}
	<span class="text-xs text-text-tertiary select-none">-</span>
{/if}
