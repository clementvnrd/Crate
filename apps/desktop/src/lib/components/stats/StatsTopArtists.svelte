<script lang="ts">
	import type { TopArtistItem } from '$shared/types'
	import { getArtworkUrl } from '$shared/utils/artwork'
	import { appDataDir, language } from '$lib/stores'
	import { formatNumber } from '$shared/utils/format'
	import Icon from '$lib/components/common/Icon.svelte'
	import { translate } from '$shared/i18n'

	type Props = {
		artists: TopArtistItem[]
		isLoading: boolean
	}

	let { artists, isLoading }: Props = $props()

	function formatMinutes(minutes: number): string {
		if (minutes >= 60) {
			return $translate('stats.duration.hoursMinutes', {
				values: { hours: formatNumber(Math.floor(minutes / 60), $language), minutes: minutes % 60 },
			})
		}
		return $translate('stats.duration.minutes', { values: { minutes } })
	}
</script>

<div class="flex h-full flex-col rounded-xl border border-stroke/60 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl">
	<div class="mb-4 flex items-center justify-between">
		<div class="flex items-center gap-2">
			<div class="flex h-7 w-7 items-center justify-center rounded-lg bg-pulse-artists/15 text-pulse-artists-text">
				<Icon name="user" class="h-4 w-4" />
			</div>
			<div>
				<h3 class="text-sm font-bold text-text-primary">{$translate('stats.topArtists.title')}</h3>
				<p class="text-[11px] text-text-tertiary">{$translate('stats.topArtists.subtitle')}</p>
			</div>
		</div>
		<span
			class="rounded-full border border-stroke bg-surface-2 px-2 py-0.5 font-mono text-[10px] font-medium text-text-secondary"
		>
			{$translate('stats.topArtists.count', { values: { count: artists.length } })}
		</span>
	</div>

	{#if isLoading && artists.length === 0}
		<div class="space-y-3 py-4">
			{#each Array(5) as _, i (i)}
				<div class="flex animate-pulse items-center gap-3 motion-reduce:animate-none">
					<div class="h-10 w-10 rounded-full bg-surface-3"></div>
					<div class="flex-1 space-y-1.5">
						<div class="h-3.5 w-3/4 rounded bg-surface-3"></div>
						<div class="h-2.5 w-1/2 rounded bg-surface-3"></div>
					</div>
				</div>
			{/each}
		</div>
	{:else if artists.length === 0}
		<div class="flex flex-1 items-center justify-center py-12 text-center text-xs text-text-tertiary">
			{$translate('stats.topArtists.empty')}
		</div>
	{:else}
		<div class="max-h-[440px] flex-1 space-y-1.5 divide-y divide-stroke/20 overflow-y-auto pr-1">
			{#each artists as artist, index (artist.artist + index)}
				{@const rank = index + 1}
				{@const artUrl = getArtworkUrl(artist.artwork_url, $appDataDir)}
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

					<!-- Artist Avatar -->
					<div
						class="relative h-10 w-10 flex-shrink-0 overflow-hidden rounded-full border border-stroke/60 bg-surface-3 shadow-sm"
					>
						{#if artUrl}
							<img
								src={artUrl}
								alt={artist.artist}
								class="h-full w-full object-cover transition-transform group-hover:scale-105"
								loading="lazy"
							/>
						{:else}
							<div class="flex h-full w-full items-center justify-center text-text-tertiary">
								<Icon name="user" class="h-5 w-5" />
							</div>
						{/if}
					</div>

					<!-- Artist Name & Top Track -->
					<div class="min-w-0 flex-1 space-y-0.5">
						<div
							class="truncate text-xs font-bold text-text-primary transition-colors group-hover:text-pulse-artists-text"
						>
							{artist.artist}
						</div>
						{#if artist.top_track}
							<div class="flex items-center gap-1.5 text-[11px] text-text-secondary">
								<span class="text-[10px] text-text-tertiary">{$translate('stats.topArtists.topTrack')}</span>
								<span class="truncate text-text-secondary italic">{artist.top_track}</span>
							</div>
						{/if}
					</div>

					<!-- Plays & Minutes -->
					<div class="min-w-[70px] flex-shrink-0 space-y-0.5 text-right">
						<div class="font-mono text-xs font-bold text-text-primary">
							{$translate('stats.plays', { values: { count: artist.plays } })}
						</div>
						<div class="font-mono text-[10px] text-text-tertiary">
							{formatMinutes(artist.total_minutes)}
						</div>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
