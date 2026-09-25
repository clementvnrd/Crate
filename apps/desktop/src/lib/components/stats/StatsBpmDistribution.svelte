<script lang="ts">
	import type { BpmBucketItem } from '$shared/types'
	import Icon from '$lib/components/common/Icon.svelte'

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
		return 'Électronique'
	}

	let maxCount = $derived(
		Math.max(...bpmStats.map((b) => b.count), 1)
	)

	let totalTracks = $derived(
		bpmStats.reduce((acc, b) => acc + b.count, 0) || 1
	)
</script>

<div class="flex flex-col h-full rounded-2xl border border-stroke/60 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl">
	<div class="mb-4 flex items-center justify-between">
		<div class="flex items-center gap-2">
			<div class="flex h-7 w-7 items-center justify-center rounded-lg bg-emerald-500/15 text-emerald-400">
				<Icon name="activity" class="h-4 w-4" />
			</div>
			<div>
				<h3 class="text-sm font-bold text-text-primary">Distribution du Tempo (BPM)</h3>
				<p class="text-[11px] text-text-tertiary">Analyse des plages de tempo & styles musicaux</p>
			</div>
		</div>
		<span class="rounded-full bg-surface-2 border border-stroke px-2 py-0.5 text-[10px] font-mono font-medium text-text-secondary">
			{bpmStats.length} plages
		</span>
	</div>

	{#if isLoading && bpmStats.length === 0}
		<div class="space-y-3 py-4">
			{#each Array(4) as _}
				<div class="h-8 rounded-lg bg-surface-3 animate-pulse"></div>
			{/each}
		</div>
	{:else if bpmStats.length === 0}
		<div class="flex flex-1 items-center justify-center py-10 text-center text-xs text-text-tertiary">
			Aucune donnée BPM disponible pour cette période.
		</div>
	{:else}
		<div class="flex-1 overflow-y-auto max-h-[360px] pr-1 space-y-2.5">
			{#each bpmStats as bucket (bucket.bpm_range)}
				{@const percentage = Math.round((bucket.count / totalTracks) * 1000) / 10}
				{@const fillPercent = Math.min(100, Math.round((bucket.count / maxCount) * 100))}
				{@const genreHint = getGenreHint(bucket.bpm_range)}
				<div class="group rounded-xl border border-stroke/40 bg-surface-2/40 p-2.5 transition-all hover:bg-surface-2/80 hover:border-stroke">
					<div class="flex items-center justify-between mb-1.5">
						<div class="flex items-center gap-2">
							<span class="rounded bg-emerald-500/15 border border-emerald-500/30 px-2 py-0.5 text-xs font-mono font-bold text-emerald-400 shadow-xs">
								{bucket.bpm_range} BPM
							</span>
							<span class="text-xs font-medium text-text-secondary truncate">
								{genreHint}
							</span>
						</div>

						<div class="flex items-center gap-2 font-mono text-xs">
							<span class="font-bold text-text-primary">{bucket.count} morceaux</span>
							<span class="text-text-tertiary text-[11px]">({percentage}%)</span>
						</div>
					</div>

					<!-- Progress bar -->
					<div class="relative h-2 w-full overflow-hidden rounded-full bg-surface-3">
						<div
							class="h-full rounded-full bg-gradient-to-r from-emerald-500 to-teal-400 transition-all duration-500 shadow-sm"
							style="width: {fillPercent}%"
						></div>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
