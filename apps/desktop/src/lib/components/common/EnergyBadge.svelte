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
</script>

{#if info && variant === 'pill'}
	<span
		class="inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 text-[10px] font-bold shadow-xs {className}"
		style="background-color: {info.bg}; color: {info.color}; border-color: {info.border};"
		title={$translate('badges.energy.titlePlayer', { values: { descriptor: info.descriptor, level: energy } })}
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
		style="background-color: {info.bg}; color: {info.color}; border-color: {info.border}; box-shadow: {info.glow};"
		title={$translate('badges.energy.title', { values: { level: info.level, descriptor: info.descriptor } })}
	>
		<span class="text-xs leading-none" aria-hidden="true">{info.symbol}</span>
		<span class="font-extrabold tabular-nums">{info.level}</span>
	</span>
{:else}
	<span class="text-xs text-text-tertiary select-none">-</span>
{/if}
