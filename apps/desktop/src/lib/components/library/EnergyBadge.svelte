<script lang="ts">
	import { getEnergyInfo } from '$shared/utils'

	type Props = {
		energy: number | null | undefined
		class?: string
		size?: 'sm' | 'md'
	}

	let { energy, class: className = '', size = 'md' }: Props = $props()

	const info = $derived(getEnergyInfo(energy))
</script>

{#if info}
	<span
		class="inline-flex items-center justify-center gap-1 rounded font-bold tracking-tight select-none border transition-all {size === 'sm' ? 'px-1.5 py-0.5 text-[10px]' : 'px-2 py-0.5 text-[11px] min-w-[46px]'} {className}"
		style="
			background-color: {info.bg};
			color: {info.color};
			border-color: {info.border};
			box-shadow: {info.glow};
		"
		title="Energy {info.level}/10 : {info.descriptor} (Mixed In Key)"
	>
		<span class="text-xs leading-none">{info.symbol}</span>
		<span class="tabular-nums font-extrabold">{info.level}</span>
	</span>
{:else}
	<span class="text-xs text-text-tertiary select-none">-</span>
{/if}
