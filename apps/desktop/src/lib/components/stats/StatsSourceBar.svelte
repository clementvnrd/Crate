<script lang="ts">
	import type { StatsSummary } from '$shared/types'
	import { language } from '$lib/stores'
	import { formatNumber } from '$shared/utils/format'
	import { translate } from '$shared/i18n'

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
		bgClass: string
		textClass: string
		glowClass: string
	}

	const SOURCE_CONFIG: Record<string, { label: string; bgClass: string; textClass: string; glowClass: string }> = {
		spotify: {
			label: 'Spotify',
			bgClass: 'bg-source-spotify',
			textClass: 'text-source-spotify-text',
			glowClass: 'shadow-source-spotify/20',
		},
		crate_local: {
			label: 'Crate Local',
			bgClass: 'bg-source-crate',
			textClass: 'text-source-crate-text',
			glowClass: 'shadow-source-crate/20',
		},
		crate_beatport: {
			label: 'Beatport',
			bgClass: 'bg-beatport',
			textClass: 'text-beatport-text-strong',
			glowClass: 'shadow-beatport/20',
		},
		rekordbox: {
			label: 'Rekordbox DJ',
			bgClass: 'bg-source-rekordbox',
			textClass: 'text-source-rekordbox-text',
			glowClass: 'shadow-source-rekordbox/20',
		},
		mixed_in_key: {
			label: 'Mixed In Key',
			bgClass: 'bg-source-mik',
			textClass: 'text-source-mik-text',
			glowClass: 'shadow-source-mik/20',
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
				bgClass: 'bg-info',
				textClass: 'text-info',
				glowClass: 'shadow-info/20',
			}
			const percentage = Math.round((minutes / total) * 1000) / 10
			items.push({
				id: key,
				label: config.label,
				icon: key,
				minutes,
				percentage,
				bgClass: config.bgClass,
				textClass: config.textClass,
				glowClass: config.glowClass,
			})
		}

		// Sort by minutes descending
		return items.sort((a, b) => b.minutes - a.minutes)
	})

	function formatMinutes(minutes: number): string {
		if (minutes >= 60) {
			return $translate('stats.duration.hoursMinutes', {
				values: { hours: formatNumber(Math.floor(minutes / 60), $language), minutes: minutes % 60 },
			})
		}
		return $translate('stats.duration.minutesCompact', { values: { minutes } })
	}

	function segmentLabel(item: SourceItem): string {
		return $translate('stats.sourceBar.segment', {
			values: {
				source: item.label,
				minutes: formatNumber(item.minutes, $language),
				percent: formatNumber(item.percentage, $language),
			},
		})
	}

	let totalMinutes = $derived(
		summary?.total_minutes ?? Object.values(summary?.source_breakdown ?? {}).reduce((a, b) => a + b, 0)
	)
</script>

<div class="rounded-xl border border-stroke/60 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl">
	<div class="mb-3 flex flex-wrap items-center justify-between gap-2">
		<div class="flex items-center gap-2">
			<div class="h-2 w-2 animate-pulse rounded-full bg-pulse-listening-bright motion-reduce:animate-none"></div>
			<h3 class="text-xs font-bold tracking-wider text-text-secondary uppercase">
				{$translate('stats.sourceBar.title')}
			</h3>
		</div>
		<span class="font-mono text-xs text-text-tertiary">
			{totalMinutes > 0
				? $translate('stats.sourceBar.total', { values: { total: formatNumber(totalMinutes, $language) } })
				: $translate('stats.sourceBar.noPlays')}
		</span>
	</div>

	<!-- Stacked Proportional Horizontal Bar -->
	{#if sourceBreakdown.length > 0}
		<div class="relative flex h-3.5 w-full overflow-hidden rounded-full bg-surface-3 p-0.5 shadow-inner">
			{#each sourceBreakdown as item (item.id)}
				<div
					class="h-full transition-all duration-500 first:rounded-l-full last:rounded-r-full {item.bgClass} hover:opacity-90"
					style="width: {item.percentage}%"
					role="img"
					aria-label={segmentLabel(item)}
					title={segmentLabel(item)}
				></div>
			{/each}
		</div>

		<!-- Legend items -->
		<div class="mt-4 flex flex-wrap items-center gap-x-6 gap-y-2">
			{#each sourceBreakdown as item (item.id)}
				<div class="flex items-center gap-2 text-xs">
					<span class="h-2.5 w-2.5 rounded-full shadow-sm {item.bgClass}"></span>
					<span class="font-medium text-text-primary">{item.label}</span>
					<span class="font-mono text-[11px] text-text-tertiary">
						{$translate('stats.percent', { values: { percent: formatNumber(item.percentage, $language) } })}
					</span>
					<span class="font-mono text-[10px] text-text-tertiary/70">
						({formatMinutes(item.minutes)})
					</span>
				</div>
			{/each}
		</div>
	{:else}
		<div class="py-3 text-center text-xs text-text-tertiary">
			{$translate('stats.sourceBar.empty')}
		</div>
	{/if}
</div>
