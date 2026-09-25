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

	const albumArtUrl = $derived(
		album.artwork_path ? getArtworkUrl(album.artwork_path, $appDataDir) : null
	)

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

<div class="flex flex-col h-full w-full overflow-y-auto px-6 py-4">
	<!-- Top Navigation Bar: Back button -->
	<div class="flex items-center justify-between pb-4 flex-shrink-0">
		<button
			type="button"
			class="inline-flex items-center gap-2 rounded-xl bg-surface-1 hover:bg-surface-2 border border-stroke-subtle px-3.5 py-1.5 text-xs font-semibold text-text-secondary hover:text-text-primary active:scale-95 transition-all cursor-pointer shadow-xs"
			onclick={onBack}
		>
			<Icon name="arrow-left" class="h-3.5 w-3.5 text-cyan-600 dark:text-cyan-400" />
			<span>Tous les albums</span>
		</button>

		<button
			type="button"
			class="flex items-center gap-1.5 rounded-xl border border-stroke-subtle bg-surface-1 px-3 py-1.5 text-xs font-medium text-text-tertiary hover:border-red-500/30 hover:bg-red-500/10 hover:text-red-500 dark:hover:text-red-400 active:scale-95 transition-all cursor-pointer"
			onclick={handleRemoveAlbum}
		>
			<Icon name="trash" class="h-3.5 w-3.5" />
			<span>Retirer l'album</span>
		</button>
	</div>

	<!-- Album Header (Apple Music Detail Hero) -->
	<div class="flex items-center gap-6 md:gap-8 py-4 border-b border-stroke-subtle flex-shrink-0">
		<!-- 190x190px Cover Artwork -->
		<div class="relative h-[180px] w-[180px] md:h-[190px] md:w-[190px] flex-shrink-0 overflow-hidden rounded-2xl border border-stroke-subtle bg-surface-2/80 shadow-2xl shadow-black/20 dark:shadow-black/60">
			{#if albumArtUrl}
				<img
					src={albumArtUrl}
					alt={album.title}
					class="h-full w-full object-cover"
				/>
			{:else}
				<div class="flex h-full w-full items-center justify-center text-text-tertiary">
					<Icon name="disc" class="h-16 w-16 opacity-30" />
				</div>
			{/if}
		</div>

		<!-- Metadata & Play Controls -->
		<div class="flex flex-1 flex-col justify-center min-w-0">
			<span class="text-[11px] font-bold uppercase tracking-wider text-cyan-600 dark:text-cyan-400">
				Album
			</span>
			<h1 class="text-2xl md:text-3xl font-extrabold tracking-tight text-text-primary line-clamp-1 mt-0.5" title={album.title}>
				{album.title}
			</h1>
			<p class="text-base font-semibold text-text-secondary mt-1 truncate" title={album.artist}>
				{album.artist}
			</p>

			<!-- Genre, Year, Track count & Total duration -->
			<div class="flex flex-wrap items-center gap-2 text-xs text-text-tertiary mt-2">
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
			<div class="flex items-center gap-3 mt-4">
				<button
					type="button"
					class="flex items-center gap-2 rounded-xl bg-cyan-500 hover:bg-cyan-400 text-black px-4 py-2 text-xs font-bold shadow-lg shadow-cyan-500/25 active:scale-95 transition-all cursor-pointer"
					onclick={handlePlayAll}
				>
					<Icon name="play" class="h-4 w-4" fill />
					<span>Lecture</span>
				</button>

				<button
					type="button"
					class="flex items-center gap-2 rounded-xl bg-surface-1 hover:bg-surface-2 border border-stroke-subtle text-text-primary px-3.5 py-2 text-xs font-bold active:scale-95 transition-all cursor-pointer"
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
		<div class="grid grid-cols-[36px_minmax(200px,2fr)_120px_120px_70px] items-center gap-3 px-3 py-2 text-[10px] font-semibold uppercase tracking-wider text-text-tertiary border-b border-stroke-subtle">
			<span class="pl-1">#</span>
			<span>Titre & Artiste</span>
			<span>Format / Bitrate</span>
			<span>BPM / Clé</span>
			<span class="text-right pr-2">Durée</span>
		</div>

		<!-- Tracks Rows -->
		<div class="flex flex-col divide-y divide-stroke-subtle/50 py-1">
			{#each $selectedAlbumTracks as track, index}
				{@const isPlayingThis = $isPlaying && (($playbackSource === 'standalone' && $standaloneTrack?.file_path === track.file_path) || ($playbackSource === 'library' && $currentTrack?.file_path === track.file_path))}
				{@const trackCamelot = track.key ? getCamelotColor(track.key) : null}

				<!-- Track Row -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<div
					class="grid grid-cols-[36px_minmax(200px,2fr)_120px_120px_70px] items-center gap-3 px-3 py-2.5 rounded-xl text-xs transition-colors cursor-pointer group {isPlayingThis ? 'bg-cyan-500/10 text-cyan-600 dark:text-cyan-300' : 'hover:bg-surface-1 text-text-primary'}"
					onclick={() => handlePlayTrack(track)}
				>
					<!-- Index / Play Icon / Equalizer Animation -->
					<div class="pl-1 font-mono text-text-tertiary flex items-center">
						{#if isPlayingThis}
							<div class="flex items-end gap-[2px] h-3.5 w-3.5 text-cyan-600 dark:text-cyan-400">
								<span class="w-[3px] bg-cyan-600 dark:bg-cyan-400 rounded-full animate-eq-1"></span>
								<span class="w-[3px] bg-cyan-600 dark:bg-cyan-400 rounded-full animate-eq-2"></span>
								<span class="w-[3px] bg-cyan-600 dark:bg-cyan-400 rounded-full animate-eq-3"></span>
							</div>
						{:else}
							<span class="group-hover:hidden">{track.track_number || index + 1}</span>
							<Icon name="play" class="h-3.5 w-3.5 hidden group-hover:block text-cyan-600 dark:text-cyan-400" fill />
						{/if}
					</div>

					<!-- Title & Artist -->
					<div class="min-w-0 pr-2">
						<div class="font-semibold text-text-primary group-hover:text-cyan-600 dark:group-hover:text-cyan-400 transition-colors truncate">
							{track.title}
						</div>
						<div class="text-[11px] text-text-secondary truncate mt-0.5">
							{track.artist}
						</div>
					</div>

					<!-- Format & Bitrate -->
					<div class="font-mono text-[11px] truncate">
						<span class="font-bold uppercase text-text-primary">{track.format}</span>
						{#if track.bitrate}
							<span class="text-text-tertiary ml-1">({formatBitrate(track.bitrate, track.format, track.sample_rate)})</span>
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
									class="rounded-full px-1.5 py-0.2 text-[9px] font-extrabold border shadow-xs"
									style="background-color: {trackCamelot.bg}; color: {trackCamelot.text}; border-color: {trackCamelot.border};"
								>
									{formatKey(track.key, 'camelot')}
								</span>
							{:else}
								<span class="font-mono text-text-secondary text-[10px]">{track.key}</span>
							{/if}
						{/if}
						{#if !track.bpm && !track.key}
							<span class="text-text-tertiary">-</span>
						{/if}
					</div>

					<!-- Duration -->
					<div class="font-mono text-right text-text-secondary tabular-nums text-[11px] pr-2">
						{formatDuration(track.duration_ms)}
					</div>
				</div>
			{/each}
		</div>
	</div>
</div>

<style>
	@keyframes eq-pulse-1 {
		0%, 100% { height: 4px; }
		50% { height: 14px; }
	}
	@keyframes eq-pulse-2 {
		0%, 100% { height: 12px; }
		50% { height: 5px; }
	}
	@keyframes eq-pulse-3 {
		0%, 100% { height: 6px; }
		50% { height: 14px; }
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
