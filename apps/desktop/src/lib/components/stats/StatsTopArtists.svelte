<script lang="ts">
	import type { TopArtistItem } from '$shared/types'
	import { getArtworkUrl } from '$shared/utils/artwork'
	import { appDataDir } from '$lib/stores'
	import Icon from '$lib/components/common/Icon.svelte'

	type Props = {
		artists: TopArtistItem[]
		isLoading: boolean
	}

	let { artists, isLoading }: Props = $props()
</script>

<div class="flex flex-col h-full rounded-2xl border border-stroke/60 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl">
	<div class="mb-4 flex items-center justify-between">
		<div class="flex items-center gap-2">
			<div class="flex h-7 w-7 items-center justify-center rounded-lg bg-pink-500/15 text-pink-400">
				<Icon name="user" class="h-4 w-4" />
			</div>
			<div>
				<h3 class="text-sm font-bold text-text-primary">Top Artistes</h3>
				<p class="text-[11px] text-text-tertiary">Producteurs et créateurs favoris</p>
			</div>
		</div>
		<span class="rounded-full bg-surface-2 border border-stroke px-2 py-0.5 text-[10px] font-mono font-medium text-text-secondary">
			{artists.length} artistes
		</span>
	</div>

	{#if isLoading && artists.length === 0}
		<div class="space-y-3 py-4">
			{#each Array(5) as _}
				<div class="flex items-center gap-3 animate-pulse">
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
			Aucun artiste enregistré pour cette période.
		</div>
	{:else}
		<div class="flex-1 overflow-y-auto max-h-[440px] pr-1 space-y-1.5 divide-y divide-stroke/20">
			{#each artists as artist, index (artist.artist + index)}
				{@const rank = index + 1}
				{@const artUrl = getArtworkUrl(artist.artwork_url, $appDataDir)}
				<div
					class="group flex items-center gap-3 py-2 px-2.5 rounded-xl transition-all duration-200 hover:bg-surface-2/80"
				>
					<!-- Rank Badge -->
					<div class="flex h-6 w-6 flex-shrink-0 items-center justify-center text-xs font-black font-mono">
						{#if rank === 1}
							<span class="flex h-5 w-5 items-center justify-center rounded-full bg-amber-400/20 text-amber-300 border border-amber-400/40 text-[11px]">1</span>
						{:else if rank === 2}
							<span class="flex h-5 w-5 items-center justify-center rounded-full bg-slate-400/20 text-slate-300 border border-slate-400/40 text-[11px]">2</span>
						{:else if rank === 3}
							<span class="flex h-5 w-5 items-center justify-center rounded-full bg-amber-700/20 text-amber-600 border border-amber-600/40 text-[11px]">3</span>
						{:else}
							<span class="text-text-tertiary text-[11px]">{rank}</span>
						{/if}
					</div>

					<!-- Artist Avatar -->
					<div class="relative h-10 w-10 flex-shrink-0 overflow-hidden rounded-full bg-surface-3 border border-stroke/60 shadow-sm">
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
						<div class="truncate text-xs font-bold text-text-primary group-hover:text-pink-400 transition-colors">
							{artist.artist}
						</div>
						{#if artist.top_track}
							<div class="flex items-center gap-1.5 text-[11px] text-text-secondary">
								<span class="text-text-tertiary text-[10px]">Top :</span>
								<span class="truncate italic text-text-secondary">{artist.top_track}</span>
							</div>
						{/if}
					</div>

					<!-- Plays & Minutes -->
					<div class="text-right min-w-[70px] space-y-0.5 flex-shrink-0">
						<div class="text-xs font-bold font-mono text-text-primary">
							{artist.plays.toLocaleString()} {artist.plays > 1 ? 'écoutes' : 'écoute'}
						</div>
						<div class="text-[10px] font-mono text-text-tertiary">
							{artist.total_minutes >= 60 ? `${Math.floor(artist.total_minutes / 60)}h ${artist.total_minutes % 60}m` : `${artist.total_minutes} min`}
						</div>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
