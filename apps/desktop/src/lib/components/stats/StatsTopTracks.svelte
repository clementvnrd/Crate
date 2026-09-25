<script lang="ts">
	import type { TopTrackItem } from '$shared/types'
	import { getCamelotColor, formatCamelotKey } from '$shared/utils/camelot'
	import { getArtworkUrl } from '$shared/utils/artwork'
	import { appDataDir } from '$lib/stores'
	import Icon from '$lib/components/common/Icon.svelte'

	type Props = {
		tracks: TopTrackItem[]
		isLoading: boolean
	}

	let { tracks, isLoading }: Props = $props()

	function formatSourceLabel(source: string): { label: string; class: string } {
		switch (source) {
			case 'spotify':
				return { label: 'Spotify', class: 'bg-[#1DB954]/15 text-[#1DB954] border-[#1DB954]/30' }
			case 'crate_local':
				return { label: 'Crate', class: 'bg-violet-500/15 text-violet-400 border-violet-500/30' }
			case 'crate_beatport':
				return { label: 'Beatport', class: 'bg-[#00FF96]/15 text-[#00FF96] border-[#00FF96]/30' }
			case 'rekordbox':
				return { label: 'Rekordbox', class: 'bg-red-500/15 text-red-400 border-red-500/30' }
			case 'mixed_in_key':
				return { label: 'MIK', class: 'bg-[#00D2FF]/15 text-[#00D2FF] border-[#00D2FF]/30' }
			default:
				return { label: source, class: 'bg-surface-3 text-text-secondary border-stroke' }
		}
	}
</script>

<div class="flex flex-col h-full rounded-2xl border border-stroke/60 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl">
	<div class="mb-4 flex items-center justify-between">
		<div class="flex items-center gap-2">
			<div class="flex h-7 w-7 items-center justify-center rounded-lg bg-brand-primary/15 text-brand-primary">
				<Icon name="music-note" class="h-4 w-4" />
			</div>
			<div>
				<h3 class="text-sm font-bold text-text-primary">Top Morceaux</h3>
				<p class="text-[11px] text-text-tertiary">Les titres les plus écoutés</p>
			</div>
		</div>
		<span class="rounded-full bg-surface-2 border border-stroke px-2 py-0.5 text-[10px] font-mono font-medium text-text-secondary">
			{tracks.length} titres
		</span>
	</div>

	{#if isLoading && tracks.length === 0}
		<div class="space-y-3 py-4">
			{#each Array(5) as _}
				<div class="flex items-center gap-3 animate-pulse">
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
			Aucune écoute enregistrée pour cette période.
		</div>
	{:else}
		<div class="flex-1 overflow-y-auto max-h-[440px] pr-1 space-y-1.5 divide-y divide-stroke/20">
			{#each tracks as track, index (track.title + track.artist + index)}
				{@const rank = index + 1}
				{@const camelotColor = getCamelotColor(track.key)}
				{@const primarySource = track.sources?.[0] ?? 'crate_local'}
				{@const sourceInfo = formatSourceLabel(primarySource)}
				{@const artUrl = getArtworkUrl(track.artwork_url, $appDataDir)}
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

					<!-- Artwork Thumbnail -->
					<div class="relative h-10 w-10 flex-shrink-0 overflow-hidden rounded-lg bg-surface-3 border border-stroke/60 shadow-sm">
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
						<div class="truncate text-xs font-bold text-text-primary group-hover:text-brand-primary transition-colors">
							{track.title}
						</div>
						<div class="flex items-center gap-2 text-[11px] text-text-secondary">
							<span class="truncate">{track.artist}</span>
							{#if track.album}
								<span class="text-text-tertiary/60">·</span>
								<span class="truncate text-text-tertiary text-[10px] hidden sm:inline">{track.album}</span>
							{/if}
						</div>
					</div>

					<!-- Tags & Metadata (BPM, Key, Source) -->
					<div class="flex items-center gap-2 flex-shrink-0">
						{#if track.bpm}
							<span class="rounded bg-surface-3 border border-stroke px-1.5 py-0.5 text-[10px] font-mono text-text-secondary">
								{Math.round(track.bpm)} BPM
							</span>
						{/if}

						{#if track.key}
							<span
								class="rounded px-1.5 py-0.5 text-[10px] font-mono font-bold shadow-xs border"
								style={camelotColor
									? `background-color: ${camelotColor.bg}; color: ${camelotColor.text}; border-color: ${camelotColor.border};`
									: ''}
							>
								{formatCamelotKey(track.key)}
							</span>
						{/if}

						<span class="hidden sm:inline-block rounded-full border px-2 py-0.5 text-[9px] font-bold uppercase tracking-wider {sourceInfo.class}">
							{sourceInfo.label}
						</span>

						<!-- Plays & Duration -->
						<div class="text-right min-w-[56px] space-y-0.5">
							<div class="text-xs font-bold font-mono text-text-primary">
								{track.plays} {track.plays > 1 ? 'plays' : 'play'}
							</div>
							<div class="text-[10px] font-mono text-text-tertiary">
								{track.total_minutes}m
							</div>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
