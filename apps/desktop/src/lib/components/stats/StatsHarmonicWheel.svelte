<script lang="ts">
	import type { HarmonicStatsItem } from '$shared/types'
	import { Icon, KeyBadge } from '$lib/components/common'
	import { getCamelotColor } from '$shared/utils/camelot'

	type Props = {
		harmonicStats: HarmonicStatsItem[]
		isLoading: boolean
	}

	let { harmonicStats, isLoading }: Props = $props()

	let sortedItems = $derived([...harmonicStats].sort((a, b) => b.plays - a.plays))

	let maxPlays = $derived(Math.max(...harmonicStats.map((item) => item.plays), 1))
</script>

<div class="flex h-full flex-col rounded-2xl border border-stroke/60 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl">
	<div class="mb-4 flex items-center justify-between">
		<div class="flex items-center gap-2">
			<div class="flex h-7 w-7 items-center justify-center rounded-lg bg-cyan-500/15 text-cyan-400">
				<Icon name="disc" class="h-4 w-4" />
			</div>
			<div>
				<h3 class="text-sm font-bold text-text-primary">Roue Harmonique & Tonalités</h3>
				<p class="text-[11px] text-text-tertiary">Distribution des clés Camelot & Mixed In Key</p>
			</div>
		</div>
		<span
			class="rounded-full border border-stroke bg-surface-2 px-2 py-0.5 font-mono text-[10px] font-medium text-text-secondary"
		>
			{harmonicStats.length} tonalités
		</span>
	</div>

	{#if isLoading && harmonicStats.length === 0}
		<div class="space-y-3 py-4">
			{#each Array(4) as _, i (i)}
				<div class="h-8 animate-pulse rounded-lg bg-surface-3 motion-reduce:animate-none"></div>
			{/each}
		</div>
	{:else if harmonicStats.length === 0}
		<div class="flex flex-1 items-center justify-center py-10 text-center text-xs text-text-tertiary">
			Aucune donnée harmonique disponible pour cette période.
		</div>
	{:else}
		<div class="max-h-[360px] flex-1 space-y-2.5 overflow-y-auto pr-1">
			{#each sortedItems as item (item.key)}
				<!-- The key colour also fills the play-share bar below (chart use of the palette) -->
				{@const colorInfo = getCamelotColor(item.key)}
				{@const fillPercent = Math.min(100, Math.round((item.plays / maxPlays) * 100))}
				<div
					class="group rounded-xl border border-stroke/40 bg-surface-2/40 p-2.5 transition-all hover:border-stroke hover:bg-surface-2/80"
				>
					<div class="mb-1.5 flex items-center justify-between">
						<div class="flex items-center gap-2">
							<KeyBadge value={item.key} variant="tag-wide" />
							{#if colorInfo?.name}
								<span class="max-w-[150px] truncate text-xs font-medium text-text-secondary">
									{colorInfo.name}
								</span>
							{/if}
						</div>

						<div class="flex items-center gap-2 font-mono text-xs">
							<span class="font-bold text-text-primary">{item.plays} plays</span>
							<span class="text-[11px] text-text-tertiary">({item.percentage.toFixed(1)}%)</span>
						</div>
					</div>

					<!-- Progress bar matching key color -->
					<div class="relative h-2 w-full overflow-hidden rounded-full bg-surface-3">
						<div
							class="h-full rounded-full transition-all duration-500"
							style="width: {fillPercent}%; background-color: {colorInfo?.dot ?? '#3b82f6'};"
						></div>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
