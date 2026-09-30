<script lang="ts">
	import type { StatsSummary } from '$shared/types'
	import { language } from '$lib/stores'
	import { formatNumber } from '$shared/utils/format'

	type Props = {
		summary: StatsSummary | null
	}

	let { summary }: Props = $props()

	interface SourceItem {
		id: string
		label: string
		icon: string
		minutes: number
		percentage: number
		color: string
		bgClass: string
		textClass: string
		glowClass: string
	}

	const SOURCE_CONFIG: Record<
		string,
		{ label: string; color: string; bgClass: string; textClass: string; glowClass: string }
	> = {
		spotify: {
			label: 'Spotify',
			color: '#1DB954',
			bgClass: 'bg-[#1DB954]',
			textClass: 'text-[#1DB954]',
			glowClass: 'shadow-[#1DB954]/20',
		},
		crate_local: {
			label: 'Crate Local',
			color: '#8B5CF6',
			bgClass: 'bg-violet-500',
			textClass: 'text-violet-400',
			glowClass: 'shadow-violet-500/20',
		},
		crate_beatport: {
			label: 'Beatport',
			color: '#00FF96',
			bgClass: 'bg-[#00FF96]',
			textClass: 'text-[#00FF96]',
			glowClass: 'shadow-[#00FF96]/20',
		},
		rekordbox: {
			label: 'Rekordbox DJ',
			color: '#EF4444',
			bgClass: 'bg-red-500',
			textClass: 'text-red-400',
			glowClass: 'shadow-red-500/20',
		},
		mixed_in_key: {
			label: 'Mixed In Key',
			color: '#00D2FF',
			bgClass: 'bg-[#00D2FF]',
			textClass: 'text-[#00D2FF]',
			glowClass: 'shadow-[#00D2FF]/20',
		},
	}

	let sourceBreakdown = $derived.by(() => {
		const raw = summary?.source_breakdown ?? {}
		const total = Object.values(raw).reduce((a, b) => a + b, 0) || 1

		const items: SourceItem[] = []
		for (const [key, minutes] of Object.entries(raw)) {
			if (minutes <= 0) continue
			const config = SOURCE_CONFIG[key] ?? {
				label: key.replace(/_/g, ' '),
				color: '#3B82F6',
				bgClass: 'bg-blue-500',
				textClass: 'text-blue-400',
				glowClass: 'shadow-blue-500/20',
			}
			const percentage = Math.round((minutes / total) * 1000) / 10
			items.push({
				id: key,
				label: config.label,
				icon: key,
				minutes,
				percentage,
				color: config.color,
				bgClass: config.bgClass,
				textClass: config.textClass,
				glowClass: config.glowClass,
			})
		}

		// Sort by minutes descending
		return items.sort((a, b) => b.minutes - a.minutes)
	})

	let totalMinutes = $derived(
		summary?.total_minutes ?? Object.values(summary?.source_breakdown ?? {}).reduce((a, b) => a + b, 0)
	)
</script>

<div class="rounded-2xl border border-stroke/60 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl">
	<div class="mb-3 flex flex-wrap items-center justify-between gap-2">
		<div class="flex items-center gap-2">
			<div class="h-2 w-2 animate-pulse rounded-full bg-emerald-400 motion-reduce:animate-none"></div>
			<h3 class="text-xs font-bold tracking-wider text-text-secondary uppercase">Répartition Multi-Sources</h3>
		</div>
		<span class="font-mono text-xs text-text-tertiary">
			{totalMinutes > 0 ? `${formatNumber(totalMinutes, $language)} min d'écoute totale` : 'Aucune écoute'}
		</span>
	</div>

	<!-- Stacked Proportional Horizontal Bar -->
	{#if sourceBreakdown.length > 0}
		<div class="relative flex h-3.5 w-full overflow-hidden rounded-full bg-surface-3 p-0.5 shadow-inner">
			{#each sourceBreakdown as item (item.id)}
				<div
					class="h-full transition-all duration-500 first:rounded-l-full last:rounded-r-full {item.bgClass} cursor-pointer hover:opacity-90"
					style="width: {item.percentage}%"
					role="img"
					aria-label="{item.label} : {formatNumber(item.minutes, $language)} min ({item.percentage}%)"
					title="{item.label} : {formatNumber(item.minutes, $language)} min ({item.percentage}%)"
				></div>
			{/each}
		</div>

		<!-- Legend items -->
		<div class="mt-4 flex flex-wrap items-center gap-x-6 gap-y-2">
			{#each sourceBreakdown as item (item.id)}
				<div class="flex items-center gap-2 text-xs">
					<span class="h-2.5 w-2.5 rounded-full shadow-sm" style="background-color: {item.color}"></span>
					<span class="font-medium text-text-primary">{item.label}</span>
					<span class="font-mono text-[11px] text-text-tertiary">
						{item.percentage}%
					</span>
					<span class="font-mono text-[10px] text-text-tertiary/70">
						({item.minutes >= 60 ? `${Math.floor(item.minutes / 60)}h ${item.minutes % 60}m` : `${item.minutes}m`})
					</span>
				</div>
			{/each}
		</div>
	{:else}
		<div class="py-3 text-center text-xs text-text-tertiary">
			Aucune donnée d'écoute multi-sources pour cette période.
		</div>
	{/if}
</div>
