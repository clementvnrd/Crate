<script lang="ts">
	import type { TopTrackItem } from '$shared/types'
	import { getArtworkUrl } from '$shared/utils/artwork'
	import { appDataDir, language } from '$lib/stores'
	import { translate } from '$shared/i18n'
	import { formatNumber } from '$shared/utils/format'
	import { Icon, KeyBadge } from '$lib/components/common'

	type Props = {
		tracks: TopTrackItem[]
		isLoading: boolean
	}

	let { tracks, isLoading }: Props = $props()

	function formatSourceLabel(source: string): { label: string; class: string } {
		switch (source) {
			case 'spotify':
				return { label: 'Spotify', class: 'bg-source-spotify/15 text-source-spotify-text border-source-spotify/30' }
			case 'crate_local':
				return { label: 'Crate', class: 'bg-source-crate/15 text-source-crate-text border-source-crate/30' }
			case 'crate_beatport':
				return { label: 'Beatport', class: 'bg-beatport/15 text-beatport-text-strong border-beatport/30' }
			case 'rekordbox':
				return {
					label: 'Rekordbox',
					class: 'bg-source-rekordbox/15 text-source-rekordbox-text border-source-rekordbox/30',
				}
			case 'mixed_in_key':
				return { label: 'MIK', class: 'bg-source-mik/15 text-source-mik-text border-source-mik/30' }
			default:
				return { label: source, class: 'bg-surface-3 text-text-secondary border-stroke' }
		}
	}
</script>

<div class="flex h-full flex-col rounded-xl border border-stroke/60 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl">
	<div class="mb-4 flex items-center justify-between">
		<div class="flex items-center gap-2">
			<div class="flex h-7 w-7 items-center justify-center rounded-lg bg-brand-primary/15 text-brand-primary">
				<Icon name="music-note" class="h-4 w-4" />
			</div>
			<div>
				<h3 class="text-sm font-bold text-text-primary">{$translate('stats.topTracks.title')}</h3>
				<p class="text-[11px] text-text-tertiary">{$translate('stats.topTracks.subtitle')}</p>
			</div>
		</div>
		<span
			class="rounded-full border border-stroke bg-surface-2 px-2 py-0.5 font-mono text-[10px] font-medium text-text-secondary"
		>
			{$translate('stats.topTracks.count', { values: { count: tracks.length } })}
		</span>
	</div>

	{#if isLoading && tracks.length === 0}
		<div class="space-y-3 py-4">
			{#each Array(5) as _, i (i)}
				<div class="flex animate-pulse items-center gap-3 motion-reduce:animate-none">
					<div class="h-10 w-10 rounded-lg bg-surface-3"></div>
					<div class="flex-1 space-y-1.5">
						<div class="h-3.5 w-3/4 rounded bg-surface-3"></div>
						<div class="h-2.5 w-1/2 rounded bg-surface-3"></div>
					</div>
				</div>
			{/each}
		</div>
	{:else if tracks.length === 0}
		<div class="flex flex-1 items-center justify-center py-12 text-center text-xs text-text-tertiary">
			{$translate('stats.topTracks.empty')}
		</div>
	{:else}
		<div class="max-h-[440px] flex-1 space-y-1.5 divide-y divide-stroke/20 overflow-y-auto pr-1">
			{#each tracks as track, index (track.title + track.artist + index)}
				{@const rank = index + 1}
				{@const primarySource = track.sources?.[0] ?? 'crate_local'}
				{@const sourceInfo = formatSourceLabel(primarySource)}
				{@const artUrl = getArtworkUrl(track.artwork_url, $appDataDir)}
				<div
					class="group flex items-center gap-3 rounded-xl px-2.5 py-2 transition-all duration-200 hover:bg-surface-2/80"
				>
					<!-- Rank Badge -->
					<div class="flex h-6 w-6 flex-shrink-0 items-center justify-center font-mono text-xs font-black">
						{#if rank === 1}
							<span
								class="flex h-5 w-5 items-center justify-center rounded-full border border-pulse-gold/40 bg-pulse-gold/20 text-[11px] text-pulse-gold-text"
								>1</span
							>
						{:else if rank === 2}
							<span
								class="flex h-5 w-5 items-center justify-center rounded-full border border-pulse-silver/40 bg-pulse-silver/20 text-[11px] text-pulse-silver-text"
								>2</span
							>
						{:else if rank === 3}
							<span
								class="flex h-5 w-5 items-center justify-center rounded-full border border-pulse-bronze/40 bg-pulse-bronze-fill/20 text-[11px] text-pulse-bronze-text"
								>3</span
							>
						{:else}
							<span class="text-[11px] text-text-tertiary">{rank}</span>
						{/if}
					</div>

					<!-- Artwork Thumbnail -->
					<div
						class="relative h-10 w-10 flex-shrink-0 overflow-hidden rounded-lg border border-stroke/60 bg-surface-3 shadow-sm"
					>
						{#if artUrl}
							<img
								src={artUrl}
								alt={track.title}
								class="h-full w-full object-cover transition-transform group-hover:scale-105"
								loading="lazy"
							/>
						{:else}
							<div class="flex h-full w-full items-center justify-center text-text-tertiary">
								<Icon name="disc" class="h-5 w-5" />
							</div>
						{/if}
					</div>

					<!-- Title & Artist -->
					<div class="min-w-0 flex-1 space-y-0.5">
						<div class="truncate text-xs font-bold text-text-primary transition-colors group-hover:text-brand-primary">
							{track.title}
						</div>
						<div class="flex items-center gap-2 text-[11px] text-text-secondary">
							<span class="truncate">{track.artist}</span>
							{#if track.album}
								<span class="text-text-tertiary/60">·</span>
								<span class="hidden truncate text-[10px] text-text-tertiary sm:inline">{track.album}</span>
							{/if}
						</div>
					</div>

					<!-- Tags & Metadata (BPM, Key, Source) -->
					<div class="flex flex-shrink-0 items-center gap-2">
						{#if track.bpm}
							<span
								class="rounded border border-stroke bg-surface-3 px-1.5 py-0.5 font-mono text-[10px] text-text-secondary"
							>
								{Math.round(track.bpm)} BPM
							</span>
						{/if}

						{#if track.key}
							<KeyBadge value={track.key} variant="tag" />
						{/if}

						<span
							class="hidden rounded-full border px-2 py-0.5 text-[9px] font-bold tracking-wider uppercase sm:inline-block {sourceInfo.class}"
						>
							{sourceInfo.label}
						</span>

						<!-- Plays & Duration -->
						<div class="min-w-[56px] space-y-0.5 text-right">
							<div class="font-mono text-xs font-bold text-text-primary">
								{$translate('stats.plays', { values: { count: track.plays } })}
							</div>
							<div class="font-mono text-[10px] text-text-tertiary">
								{$translate('stats.duration.minutesCompact', {
									values: { minutes: formatNumber(track.total_minutes, $language) },
								})}
							</div>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
