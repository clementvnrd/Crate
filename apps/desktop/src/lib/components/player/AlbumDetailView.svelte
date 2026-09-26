<script lang="ts">
	import {
		albumsStore,
		selectedAlbum,
		selectedAlbumTracks,
		isPlaying,
		standaloneTrack,
		currentTrack,
		playbackSource,
		appDataDir,
	} from '$lib/stores'
	import type { PlayerAlbum, PlayerAlbumTrack } from '$shared/types'
	import {
		getArtworkUrl,
		formatDuration,
		formatBpm,
		formatKey,
		formatBitrate,
		getCamelotColor,
		getEnergyInfo,
	} from '$shared/utils'
	import { Icon, Tooltip } from '$lib/components/common'

	interface Props {
		album: PlayerAlbum
		onBack: () => void
	}

	let { album, onBack }: Props = $props()

	const albumArtUrl = $derived(album.artwork_path ? getArtworkUrl(album.artwork_path, $appDataDir) : null)

	function handlePlayAll() {
		albumsStore.playAlbum(album, false)
	}

	function handlePlayShuffle() {
		albumsStore.playAlbum(album, true)
	}

	function handlePlayTrack(track: PlayerAlbumTrack) {
		albumsStore.playAlbumTrack(track, $selectedAlbumTracks, album)
	}

	function handleRemoveAlbum() {
		if (confirm('Voulez-vous retirer cet album du lecteur Crate ?')) {
			albumsStore.removeAlbum(album.id)
			onBack()
		}
	}
</script>

<div class="flex h-full w-full flex-col overflow-y-auto px-6 py-4">
	<!-- Top Navigation Bar: Back button -->
	<div class="flex flex-shrink-0 items-center justify-between pb-4">
		<button
			type="button"
			class="inline-flex cursor-pointer items-center gap-2 rounded-xl border border-stroke-subtle bg-surface-1 px-3.5 py-1.5 text-xs font-semibold text-text-secondary shadow-xs transition-all hover:bg-surface-2 hover:text-text-primary active:scale-95"
			onclick={onBack}
		>
			<Icon name="arrow-left" class="h-3.5 w-3.5 text-cyan-600 dark:text-cyan-400" />
			<span>Tous les albums</span>
		</button>

		<button
			type="button"
			class="flex cursor-pointer items-center gap-1.5 rounded-xl border border-stroke-subtle bg-surface-1 px-3 py-1.5 text-xs font-medium text-text-tertiary transition-all hover:border-red-500/30 hover:bg-red-500/10 hover:text-red-500 active:scale-95 dark:hover:text-red-400"
			onclick={handleRemoveAlbum}
		>
			<Icon name="trash" class="h-3.5 w-3.5" />
			<span>Retirer l'album</span>
		</button>
	</div>

	<!-- Album Header (Apple Music Detail Hero) -->
	<div class="flex flex-shrink-0 items-center gap-6 border-b border-stroke-subtle py-4 md:gap-8">
		<!-- 190x190px Cover Artwork -->
		<div
			class="relative h-[180px] w-[180px] flex-shrink-0 overflow-hidden rounded-2xl border border-stroke-subtle bg-surface-2/80 shadow-2xl shadow-black/20 md:h-[190px] md:w-[190px] dark:shadow-black/60"
		>
			{#if albumArtUrl}
				<img src={albumArtUrl} alt={album.title} class="h-full w-full object-cover" />
			{:else}
				<div class="flex h-full w-full items-center justify-center text-text-tertiary">
					<Icon name="disc" class="h-16 w-16 opacity-30" />
				</div>
			{/if}
		</div>

		<!-- Metadata & Play Controls -->
		<div class="flex min-w-0 flex-1 flex-col justify-center">
			<span class="text-[11px] font-bold tracking-wider text-cyan-600 uppercase dark:text-cyan-400"> Album </span>
			<h1
				class="mt-0.5 line-clamp-1 text-2xl font-extrabold tracking-tight text-text-primary md:text-3xl"
				title={album.title}
			>
				{album.title}
			</h1>
			<p class="mt-1 truncate text-base font-semibold text-text-secondary" title={album.artist}>
				{album.artist}
			</p>

			<!-- Genre, Year, Track count & Total duration -->
			<div class="mt-2 flex flex-wrap items-center gap-2 text-xs text-text-tertiary">
				{#if album.genre}
					<span>{album.genre}</span>
					<span>•</span>
				{/if}
				{#if album.year}
					<span>{album.year}</span>
					<span>•</span>
				{/if}
				<span>{album.track_count} {album.track_count > 1 ? 'morceaux' : 'morceau'}</span>
				<span>•</span>
				<span class="font-mono">{formatDuration(album.total_duration_ms)}</span>
			</div>

			<!-- Action Buttons: [▶ Tout lire] and [🔀 Aléatoire] -->
			<div class="mt-4 flex items-center gap-3">
				<button
					type="button"
					class="flex cursor-pointer items-center gap-2 rounded-xl bg-cyan-500 px-4 py-2 text-xs font-bold text-black shadow-lg shadow-cyan-500/25 transition-all hover:bg-cyan-400 active:scale-95"
					onclick={handlePlayAll}
				>
					<Icon name="play" class="h-4 w-4" fill />
					<span>Lecture</span>
				</button>

				<button
					type="button"
					class="flex cursor-pointer items-center gap-2 rounded-xl border border-stroke-subtle bg-surface-1 px-3.5 py-2 text-xs font-bold text-text-primary transition-all hover:bg-surface-2 active:scale-95"
					onclick={handlePlayShuffle}
				>
					<Icon name="shuffle" class="h-4 w-4 text-cyan-600 dark:text-cyan-400" />
					<span>Aléatoire</span>
				</button>
			</div>
		</div>
	</div>

	<!-- Tracklist Table -->
	<div class="flex-1 py-3">
		<!-- Table Header -->
		<div
			class="grid grid-cols-[36px_minmax(200px,2fr)_120px_120px_70px] items-center gap-3 border-b border-stroke-subtle px-3 py-2 text-[10px] font-semibold tracking-wider text-text-tertiary uppercase"
		>
			<span class="pl-1">#</span>
			<span>Titre & Artiste</span>
			<span>Format / Bitrate</span>
			<span>BPM / Clé</span>
			<span class="pr-2 text-right">Durée</span>
		</div>

		<!-- Tracks Rows -->
		<div class="flex flex-col divide-y divide-stroke-subtle/50 py-1">
			{#each $selectedAlbumTracks as track, index (track.id)}
				{@const isPlayingThis =
					$isPlaying &&
					(($playbackSource === 'standalone' && $standaloneTrack?.file_path === track.file_path) ||
						($playbackSource === 'library' && $currentTrack?.file_path === track.file_path))}
				{@const trackCamelot = track.key ? getCamelotColor(track.key) : null}

				<!-- Track Row -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<div
					class="group grid cursor-pointer grid-cols-[36px_minmax(200px,2fr)_120px_120px_70px] items-center gap-3 rounded-xl px-3 py-2.5 text-xs transition-colors {isPlayingThis
						? 'bg-cyan-500/10 text-cyan-600 dark:text-cyan-300'
						: 'text-text-primary hover:bg-surface-1'}"
					onclick={() => handlePlayTrack(track)}
				>
					<!-- Index / Play Icon / Equalizer Animation -->
					<div class="flex items-center pl-1 font-mono text-text-tertiary">
						{#if isPlayingThis}
							<div class="flex h-3.5 w-3.5 items-end gap-[2px] text-cyan-600 dark:text-cyan-400">
								<span class="animate-eq-1 w-[3px] rounded-full bg-cyan-600 dark:bg-cyan-400"></span>
								<span class="animate-eq-2 w-[3px] rounded-full bg-cyan-600 dark:bg-cyan-400"></span>
								<span class="animate-eq-3 w-[3px] rounded-full bg-cyan-600 dark:bg-cyan-400"></span>
							</div>
						{:else}
							<span class="group-hover:hidden">{track.track_number || index + 1}</span>
							<Icon name="play" class="hidden h-3.5 w-3.5 text-cyan-600 group-hover:block dark:text-cyan-400" fill />
						{/if}
					</div>

					<!-- Title & Artist -->
					<div class="min-w-0 pr-2">
						<div
							class="truncate font-semibold text-text-primary transition-colors group-hover:text-cyan-600 dark:group-hover:text-cyan-400"
						>
							{track.title}
						</div>
						<div class="mt-0.5 truncate text-[11px] text-text-secondary">
							{track.artist}
						</div>
					</div>

					<!-- Format & Bitrate -->
					<div class="truncate font-mono text-[11px]">
						<span class="font-bold text-text-primary uppercase">{track.format}</span>
						{#if track.bitrate}
							<span class="ml-1 text-text-tertiary"
								>({formatBitrate(track.bitrate, track.format, track.sample_rate)})</span
							>
						{/if}
					</div>

					<!-- BPM / Key -->
					<div class="flex items-center gap-1.5">
						{#if track.bpm}
							<span class="font-mono font-bold text-cyan-600 dark:text-cyan-300">{formatBpm(track.bpm)}</span>
						{/if}
						{#if track.key}
							{#if trackCamelot}
								<span
									class="py-0.2 rounded-full border px-1.5 text-[9px] font-extrabold shadow-xs"
									style="background-color: {trackCamelot.bg}; color: {trackCamelot.text}; border-color: {trackCamelot.border};"
								>
									{formatKey(track.key, 'camelot')}
								</span>
							{:else}
								<span class="font-mono text-[10px] text-text-secondary">{track.key}</span>
							{/if}
						{/if}
						{#if !track.bpm && !track.key}
							<span class="text-text-tertiary">-</span>
						{/if}
					</div>

					<!-- Duration -->
					<div class="pr-2 text-right font-mono text-[11px] text-text-secondary tabular-nums">
						{formatDuration(track.duration_ms)}
					</div>
				</div>
			{/each}
		</div>
	</div>
</div>

<style>
	@keyframes eq-pulse-1 {
		0%,
		100% {
			height: 4px;
		}
		50% {
			height: 14px;
		}
	}
	@keyframes eq-pulse-2 {
		0%,
		100% {
			height: 12px;
		}
		50% {
			height: 5px;
		}
	}
	@keyframes eq-pulse-3 {
		0%,
		100% {
			height: 6px;
		}
		50% {
			height: 14px;
		}
	}
	.animate-eq-1 {
		animation: eq-pulse-1 0.7s ease-in-out infinite;
	}
	.animate-eq-2 {
		animation: eq-pulse-2 0.5s ease-in-out infinite 0.15s;
	}
	.animate-eq-3 {
		animation: eq-pulse-3 0.8s ease-in-out infinite 0.3s;
	}
</style>
