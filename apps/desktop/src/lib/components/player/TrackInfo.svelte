<script lang="ts">
	import type { Track, PreviewInfo, StandaloneTrack } from '$shared/types'
	import type { BeatportTrack } from '$shared/types/beatport'
	import { getTrackDisplayName, getTrackDisplayArtist } from '$shared/utils'
	import { AlbumArt, AlbumArtModal, Icon, Text } from '$lib/components/common'
	import { translate } from '$shared/i18n'
	import { beatportStore } from '$shared/stores/beatport'

	type Props = {
		track: Track | null
		previewInfo?: PreviewInfo | null
		beatportTrack?: BeatportTrack | null
		standaloneTrack?: StandaloneTrack | null
		playbackSource?: 'library' | 'preview' | 'beatport' | 'standalone' | string
		onLocate?: () => void
		onLikeToggle?: () => void
	}

	let {
		track,
		previewInfo = null,
		beatportTrack = null,
		standaloneTrack = null,
		playbackSource = 'library',
		onLocate,
		onLikeToggle,
	}: Props = $props()

	let showArtworkModal = $state(false)

	const previewTrack = $derived(previewInfo ? previewInfo.release.tracks[previewInfo.trackIndex] : null)
	const previewArtworkPath = $derived(previewInfo?.release.artwork_path ?? null)
	const previewArtworkUrl = $derived(previewInfo?.release.artwork_url ?? null)

	const beatportArtistNames = $derived(
		beatportTrack?.artists?.map((a) => a.name).join(', ') || $translate('player.trackInfo.beatportArtist')
	)

	function handleArtworkClick() {
		if (
			(playbackSource === 'beatport' && beatportTrack?.artwork_url) ||
			(playbackSource === 'preview' && (previewArtworkPath || previewArtworkUrl)) ||
			(playbackSource === 'standalone' && standaloneTrack?.artwork_path) ||
			(playbackSource === 'library' && track?.artwork_path) ||
			track?.artwork_path
		) {
			showArtworkModal = true
		}
	}
</script>

<div class="flex min-w-0 items-center gap-3">
	<!-- Album art -->
	{#if playbackSource === 'beatport' && beatportTrack}
		<AlbumArt
			artworkPath={null}
			artworkUrl={beatportTrack.artwork_url ?? null}
			size="md"
			onclick={handleArtworkClick}
			class={beatportTrack.artwork_url ? 'cursor-zoom-in' : ''}
		/>
	{:else if playbackSource === 'preview' && previewInfo}
		<AlbumArt
			artworkPath={previewArtworkPath}
			artworkUrl={previewArtworkUrl}
			size="md"
			onclick={handleArtworkClick}
			class={previewArtworkPath || previewArtworkUrl ? 'cursor-zoom-in' : ''}
		/>
	{:else if playbackSource === 'standalone' && standaloneTrack}
		<AlbumArt
			artworkPath={standaloneTrack.artwork_path ?? null}
			size="md"
			onclick={handleArtworkClick}
			class={standaloneTrack.artwork_path ? 'cursor-zoom-in' : ''}
		/>
	{:else if track}
		<AlbumArt
			artworkPath={track.artwork_path ?? null}
			size="md"
			onclick={handleArtworkClick}
			class={track.artwork_path ? 'cursor-zoom-in' : ''}
		/>
	{:else}
		<AlbumArt artworkPath={null} size="md" />
	{/if}

	<!-- Track info -->
	<div class="min-w-0 flex-1">
		{#if playbackSource === 'beatport' && beatportTrack}
			<div class="flex items-center gap-1.5 truncate">
				<button
					type="button"
					class="cursor-pointer truncate text-left text-sm font-medium text-text-primary hover:underline"
					onclick={() => onLocate?.()}
				>
					{beatportTrack.title}
				</button>
				{#if beatportTrack.mix_name}
					<span class="truncate text-xs text-text-tertiary">({beatportTrack.mix_name})</span>
				{/if}
			</div>
			<div class="flex items-center gap-1 truncate text-xs text-text-secondary">
				<span
					class="py-0.2 inline-flex items-center rounded bg-beatport-tint/20 px-1 text-[9px] font-bold text-beatport-text-strong"
				>
					BP
				</span>
				<span class="truncate">{beatportArtistNames}</span>
			</div>
		{:else if playbackSource === 'preview' && previewInfo && previewTrack}
			<button
				type="button"
				class="block max-w-full cursor-pointer truncate text-left text-sm font-medium text-text-primary hover:underline"
				onclick={() => onLocate?.()}
			>
				{previewTrack.name}
			</button>
			<Text variant="caption" as="p" color="secondary" truncate>
				{previewInfo.release.artist || previewInfo.release.title || ''}
			</Text>
		{:else if playbackSource === 'standalone' && standaloneTrack}
			<button
				type="button"
				class="block max-w-full cursor-pointer truncate text-left text-sm font-medium text-text-primary hover:underline"
				onclick={() => onLocate?.()}
			>
				{standaloneTrack.title || standaloneTrack.file_path.split('/').pop() || $translate('player.unknownTrack')}
			</button>
			<Text variant="caption" as="p" color="secondary" truncate>
				{standaloneTrack.artist || $translate('player.trackInfo.externalFile')}
			</Text>
		{:else if track}
			<button
				class="block max-w-full cursor-pointer truncate text-left text-sm font-medium text-text-primary hover:underline"
				onclick={() => onLocate?.()}
			>
				{getTrackDisplayName(track)}
			</button>
			<Text variant="caption" as="p" color="secondary" truncate>
				{getTrackDisplayArtist(track)}
			</Text>
		{:else}
			<Text color="tertiary">{$translate('player.noTrackSelected')}</Text>
		{/if}
	</div>

	<!-- Like button (preview only) / Favorite button (Beatport) -->
	{#if playbackSource === 'beatport' && beatportTrack}
		<button
			type="button"
			class="flex-shrink-0 cursor-pointer text-text-tertiary transition-colors hover:text-danger-text"
			onclick={() => beatportStore.toggleFavorite(beatportTrack)}
			title={$translate('player.trackInfo.beatportFavorite')}
		>
			<Icon name="heart" class="h-3.5 w-3.5" />
		</button>
	{:else if playbackSource === 'preview' && previewInfo && previewTrack}
		<button
			class="flex-shrink-0 cursor-pointer transition-colors {previewTrack.is_liked
				? 'text-brand-primary'
				: 'text-secondary hover:text-primary'}"
			onclick={(e) => {
				e.currentTarget.animate([{ transform: 'scale(1)' }, { transform: 'scale(1.35)' }, { transform: 'scale(1)' }], {
					duration: 300,
					easing: 'ease-out',
				})
				onLikeToggle?.()
			}}
		>
			<Icon name="heart" class="h-3.5 w-3.5" fill={previewTrack.is_liked} />
		</button>
	{/if}
</div>

{#if showArtworkModal && ((playbackSource === 'beatport' && beatportTrack) || (playbackSource === 'preview' && previewInfo) || (playbackSource === 'standalone' && standaloneTrack) || track)}
	<AlbumArtModal
		open={showArtworkModal}
		artworkPath={playbackSource === 'preview'
			? previewArtworkPath
			: playbackSource === 'standalone'
				? (standaloneTrack?.artwork_path ?? null)
				: (track?.artwork_path ?? null)}
		artworkUrl={playbackSource === 'beatport'
			? (beatportTrack?.artwork_url ?? null)
			: playbackSource === 'preview'
				? previewArtworkUrl
				: null}
		trackTitle={playbackSource === 'beatport' && beatportTrack
			? beatportTrack.title
			: playbackSource === 'preview' && previewInfo && previewTrack
				? previewTrack.name
				: playbackSource === 'standalone' && standaloneTrack
					? standaloneTrack.title || standaloneTrack.file_path.split('/').pop() || ''
					: track
						? getTrackDisplayName(track)
						: ''}
		onClose={() => (showArtworkModal = false)}
	/>
{/if}
