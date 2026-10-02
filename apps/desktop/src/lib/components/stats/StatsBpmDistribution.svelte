<script lang="ts">
	import type { BpmBucketItem } from '$shared/types'
	import Icon from '$lib/components/common/Icon.svelte'
	import { translate } from '$shared/i18n'
	import { language } from '$lib/stores'
	import { formatNumber } from '$shared/utils/format'

	type Props = {
		bpmStats: BpmBucketItem[]
		isLoading: boolean
	}

	let { bpmStats, isLoading }: Props = $props()

	function getGenreHint(range: string): string {
		if (range.includes('120-125')) return 'House & Melodic'
		if (range.includes('125-130')) return 'Tech House'
		if (range.includes('130-135')) return 'Peak Techno'
		if (range.includes('115-120') || range.includes('< 120')) return 'Deep & Organic'
		if (range.includes('135') || range.includes('140') || range.includes('170')) return 'Hard Dance / DnB'
		return $translate('stats.bpm.genreElectronic')
	}

	let maxCount = $derived(Math.max(...bpmStats.map((b) => b.count), 1))

	let totalTracks = $derived(bpmStats.reduce((acc, b) => acc + b.count, 0) || 1)
</script>

<div class="flex h-full flex-col rounded-xl border border-stroke/60 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl">
	<div class="mb-4 flex items-center justify-between">
		<div class="flex items-center gap-2">
			<div class="flex h-7 w-7 items-center justify-center rounded-lg bg-pulse-listening/15 text-pulse-listening-text">
				<Icon name="activity" class="h-4 w-4" />
			</div>
			<div>
				<h3 class="text-sm font-bold text-text-primary">{$translate('stats.bpm.title')}</h3>
				<p class="text-[11px] text-text-tertiary">{$translate('stats.bpm.subtitle')}</p>
			</div>
		</div>
		<span
			class="rounded-full border border-stroke bg-surface-2 px-2 py-0.5 font-mono text-[10px] font-medium text-text-secondary"
		>
			{$translate('stats.bpm.count', { values: { count: bpmStats.length } })}
		</span>
	</div>

	{#if isLoading && bpmStats.length === 0}
		<div class="space-y-3 py-4">
			{#each Array(4) as _, i (i)}
				<div class="h-8 animate-pulse rounded-lg bg-surface-3 motion-reduce:animate-none"></div>
			{/each}
		</div>
	{:else if bpmStats.length === 0}
		<div class="flex flex-1 items-center justify-center py-10 text-center text-xs text-text-tertiary">
			{$translate('stats.bpm.empty')}
		</div>
	{:else}
		<div class="max-h-[360px] flex-1 space-y-2.5 overflow-y-auto pr-1">
			{#each bpmStats as bucket (bucket.bpm_range)}
				{@const percentage = Math.round((bucket.count / totalTracks) * 1000) / 10}
				{@const fillPercent = Math.min(100, Math.round((bucket.count / maxCount) * 100))}
				{@const genreHint = getGenreHint(bucket.bpm_range)}
				<div
					class="group rounded-xl border border-stroke/40 bg-surface-2/40 p-2.5 transition-all hover:border-stroke hover:bg-surface-2/80"
				>
					<div class="mb-1.5 flex items-center justify-between">
						<div class="flex items-center gap-2">
							<span
								class="rounded border border-pulse-listening/30 bg-pulse-listening/15 px-2 py-0.5 font-mono text-xs font-bold text-pulse-listening-text shadow-xs"
							>
								{bucket.bpm_range} BPM
							</span>
							<span class="truncate text-xs font-medium text-text-secondary">
								{genreHint}
							</span>
						</div>

						<div class="flex items-center gap-2 font-mono text-xs">
							<span class="font-bold text-text-primary"
								>{$translate('stats.plays', { values: { count: bucket.count } })}</span
							>
							<span class="text-[11px] text-text-tertiary"
								>({$translate('stats.percent', { values: { percent: formatNumber(percentage, $language) } })})</span
							>
						</div>
					</div>

					<!-- Progress bar -->
					<div class="relative h-2 w-full overflow-hidden rounded-full bg-surface-3">
						<div
							class="h-full rounded-full bg-gradient-to-r from-pulse-listening to-pulse-listening-end shadow-sm transition-all duration-500"
							style="width: {fillPercent}%"
						></div>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
