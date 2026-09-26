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

<div class="flex h-full flex-col rounded-2xl border border-stroke/60 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl">
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
		<span
			class="rounded-full border border-stroke bg-surface-2 px-2 py-0.5 font-mono text-[10px] font-medium text-text-secondary"
		>
			{artists.length} artistes
		</span>
	</div>

	{#if isLoading && artists.length === 0}
		<div class="space-y-3 py-4">
			{#each Array(5) as _, i (i)}
				<div class="flex animate-pulse items-center gap-3">
					<div class="bg-surface-3 h-10 w-10 rounded-full"></div>
					<div class="flex-1 space-y-1.5">
						<div class="bg-surface-3 h-3.5 w-3/4 rounded"></div>
						<div class="bg-surface-3 h-2.5 w-1/2 rounded"></div>
					</div>
				</div>
			{/each}
		</div>
	{:else if artists.length === 0}
		<div class="flex flex-1 items-center justify-center py-12 text-center text-xs text-text-tertiary">
			Aucun artiste enregistré pour cette période.
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
								class="flex h-5 w-5 items-center justify-center rounded-full border border-amber-400/40 bg-amber-400/20 text-[11px] text-amber-300"
								>1</span
							>
						{:else if rank === 2}
							<span
								class="flex h-5 w-5 items-center justify-center rounded-full border border-slate-400/40 bg-slate-400/20 text-[11px] text-slate-300"
								>2</span
							>
						{:else if rank === 3}
							<span
								class="flex h-5 w-5 items-center justify-center rounded-full border border-amber-600/40 bg-amber-700/20 text-[11px] text-amber-600"
								>3</span
							>
						{:else}
							<span class="text-[11px] text-text-tertiary">{rank}</span>
						{/if}
					</div>

					<!-- Artist Avatar -->
					<div
						class="bg-surface-3 relative h-10 w-10 flex-shrink-0 overflow-hidden rounded-full border border-stroke/60 shadow-sm"
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
						<div class="truncate text-xs font-bold text-text-primary transition-colors group-hover:text-pink-400">
							{artist.artist}
						</div>
						{#if artist.top_track}
							<div class="flex items-center gap-1.5 text-[11px] text-text-secondary">
								<span class="text-[10px] text-text-tertiary">Top :</span>
								<span class="truncate text-text-secondary italic">{artist.top_track}</span>
							</div>
						{/if}
					</div>

					<!-- Plays & Minutes -->
					<div class="min-w-[70px] flex-shrink-0 space-y-0.5 text-right">
						<div class="font-mono text-xs font-bold text-text-primary">
							{artist.plays.toLocaleString()}
							{artist.plays > 1 ? 'écoutes' : 'écoute'}
						</div>
						<div class="font-mono text-[10px] text-text-tertiary">
							{artist.total_minutes >= 60
								? `${Math.floor(artist.total_minutes / 60)}h ${artist.total_minutes % 60}m`
								: `${artist.total_minutes} min`}
						</div>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
