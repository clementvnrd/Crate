<script lang="ts">
	import type { BeatportTrack } from '$shared/types/beatport'
	import { beatportStore } from '$shared/stores/beatport'
	import { playerStore, isPlaying as isPlayerPlaying, beatportTrack, playbackSource } from '$lib/stores'
	import { displaySettingsStore } from '$shared/stores/displaySettings'
	import { Icon, KeyBadge } from '$lib/components/common'
	import { translate } from '$shared/i18n'
	import { BEATPORT_DATE_CELL, BEATPORT_GENRE_CELL, BEATPORT_MIX_NAME, BEATPORT_TRACK_GRID } from './trackGrid'

	interface Props {
		track: BeatportTrack
		index: number
	}

	let { track, index }: Props = $props()

	let isHovered = $state(false)

	let inCart = $derived($beatportStore.cart.some((t) => String(t.id) === String(track.id)))
	let isFavorite = $derived($beatportStore.favorites.some((t) => String(t.id) === String(track.id)))
	let isPlaying = $derived(
		$playbackSource === 'beatport' && String($beatportTrack?.id) === String(track.id) && $isPlayerPlaying
	)
	let isCurrentTrack = $derived($playbackSource === 'beatport' && String($beatportTrack?.id) === String(track.id))

	function togglePlay(e: MouseEvent) {
		e.stopPropagation()
		playerStore.playBeatport(track)
	}

	function toggleCart(e: MouseEvent) {
		e.stopPropagation()
		if (inCart) {
			beatportStore.removeFromCart(track.id)
		} else {
			beatportStore.addToCart(track)
		}
	}

	function toggleFav(e: MouseEvent) {
		e.stopPropagation()
		beatportStore.toggleFavorite(track)
	}
</script>

<!-- An item of the track list (BeatportView). The hover only swaps the index for a play button and a double-click
     plays the preview: pointer shortcuts for the title button, which plays it from the keyboard. -->
<div
	role="listitem"
	class="group grid {BEATPORT_TRACK_GRID} items-center gap-2 px-3 py-1.5 text-xs transition-colors select-none {isCurrentTrack
		? 'border-l-2 border-l-beatport bg-beatport-wash/40'
		: 'border-b border-stroke/40 hover:bg-surface-2/60'}"
	onmouseenter={() => (isHovered = true)}
	onmouseleave={() => (isHovered = false)}
	ondblclick={() => playerStore.playBeatport(track)}
>
	<!-- # / Play button / Animated Equalizer -->
	<div class="flex items-center justify-center text-text-tertiary">
		{#if isPlaying}
			{#if isHovered}
				<button
					type="button"
					class="flex h-6 w-6 cursor-pointer items-center justify-center rounded-full bg-beatport text-black shadow-lg shadow-beatport/30 transition-transform active:scale-95"
					onclick={togglePlay}
					title={$translate('beatport.track.pausePreview')}
				>
					<Icon name="pause" class="h-3 w-3" fill />
				</button>
			{:else}
				<div
					class="flex h-5 w-5 items-end justify-center gap-[2px] pb-0.5"
					title={$translate('beatport.track.nowPlaying')}
				>
					<span class="h-3 w-[3px] animate-pulse rounded-full bg-beatport motion-reduce:animate-none"></span>
					<span
						class="h-4.5 w-[3px] animate-pulse rounded-full bg-beatport [animation-delay:150ms] motion-reduce:animate-none"
					></span>
					<span
						class="h-2.5 w-[3px] animate-pulse rounded-full bg-beatport [animation-delay:300ms] motion-reduce:animate-none"
					></span>
				</div>
			{/if}
		{:else if isHovered || isCurrentTrack}
			<button
				type="button"
				class="flex h-6 w-6 cursor-pointer items-center justify-center rounded-full bg-beatport-tint/20 text-beatport-text-strong shadow-sm transition-all hover:bg-beatport hover:text-black active:scale-95"
				onclick={togglePlay}
				title={$translate('beatport.track.playPreview')}
			>
				<Icon name="play" class="ml-0.5 h-3 w-3" fill />
			</button>
		{:else}
			<span class="font-mono text-[11px] text-text-tertiary">{index}</span>
		{/if}
	</div>

	<!-- Artwork (500x500 square release cover) -->
	<div class="flex items-center justify-center">
		{#if track.artwork_url}
			<img
				src={track.artwork_url}
				alt={track.title}
				class="h-8 w-8 rounded-md border border-stroke/40 object-cover shadow-sm"
				loading="lazy"
				onerror={(e) => {
					// Fallback if image fails to load
					const target = e.currentTarget as HTMLImageElement
					target.style.display = 'none'
				}}
			/>
		{:else}
			<div
				class="flex h-8 w-8 items-center justify-center rounded-md border border-stroke/40 bg-surface-3 text-[10px] font-bold text-beatport-text-strong"
			>
				BP
			</div>
		{/if}
	</div>

	<!-- Title & Artist -->
	<div class="min-w-0 pr-2">
		<div class="flex items-center gap-1.5 truncate font-medium text-text-primary">
			<button
				type="button"
				class="cursor-pointer truncate text-left hover:text-beatport-text-strong {isCurrentTrack
					? 'font-semibold text-beatport-text-strong'
					: ''}"
				onclick={() => playerStore.playBeatport(track)}
			>
				{track.title}
			</button>
			{#if track.mix_name}
				<span class="truncate text-[11px] text-text-tertiary {BEATPORT_MIX_NAME}">({track.mix_name})</span>
			{/if}
		</div>
		<div class="truncate text-[11px] text-text-secondary">
			{#if track.artists && track.artists.length > 0}
				{#each track.artists as artist, aIdx (aIdx)}
					<button
						type="button"
						class="cursor-pointer transition-colors hover:text-beatport-text-strong hover:underline"
						onclick={(e) => {
							e.stopPropagation()
							beatportStore.setNavArtist(artist.id, artist.name, artist.image_url ?? undefined)
						}}
					>
						{artist.name}
					</button>{#if aIdx < track.artists.length - 1},&nbsp;{/if}
				{/each}
			{:else}
				<span class="text-text-tertiary">{$translate('beatport.artist.label')}</span>
			{/if}
		</div>
	</div>

	<!-- Genre -->
	<div class="truncate text-text-secondary {BEATPORT_GENRE_CELL}">
		{track.genre}
	</div>

	<!-- Released -->
	<div class="truncate font-mono text-[11px] text-text-tertiary {BEATPORT_DATE_CELL}">
		{track.release_date}
	</div>

	<!-- Length -->
	<div class="font-mono text-text-secondary tabular-nums">
		{track.duration_formatted}
	</div>

	<!-- Key (Camelot) -->
	<div class="flex items-center">
		<KeyBadge value={track.key} variant="cell-compact" zeroPad={$displaySettingsStore.camelotZeroPadding} />
	</div>

	<!-- BPM -->
	<div class="font-mono text-text-secondary tabular-nums">
		{track.bpm ? Math.round(track.bpm) : '-'}
	</div>

	<!-- Actions: Cart 🛒, Favorite ❤️, Add ➕ -->
	<div class="flex items-center justify-end gap-1.5">
		<!-- Cart / Selection -->
		<button
			type="button"
			class="flex h-6 w-6 cursor-pointer items-center justify-center rounded transition-colors hover:bg-surface-3 {inCart
				? 'font-bold text-beatport-text'
				: 'text-text-tertiary hover:text-text-primary'}"
			onclick={toggleCart}
			title={$translate(inCart ? 'beatport.cart.remove' : 'beatport.cart.add')}
		>
			<Icon name="cart" class="h-3.5 w-3.5" />
		</button>

		<!-- Favorite -->
		<button
			type="button"
			class="flex h-6 w-6 cursor-pointer items-center justify-center rounded transition-colors hover:bg-surface-3 {isFavorite
				? 'font-bold text-red-500'
				: 'text-text-tertiary hover:text-red-400'}"
			onclick={toggleFav}
			title={$translate(isFavorite ? 'beatport.track.removeFavorite' : 'beatport.track.addFavorite')}
		>
			<Icon name="heart" class="h-3.5 w-3.5" />
		</button>

		<!-- Add to Playlist / Cart Checkmark -->
		<button
			type="button"
			class="flex h-6 w-6 cursor-pointer items-center justify-center rounded text-text-tertiary transition-colors hover:bg-surface-3 hover:text-text-primary"
			onclick={toggleCart}
			title={$translate(inCart ? 'beatport.cart.inCart' : 'beatport.cart.add')}
		>
			{#if inCart}
				<Icon name="check" class="h-3.5 w-3.5 text-beatport-text-strong" />
			{:else}
				<Icon name="plus" class="h-3.5 w-3.5" />
			{/if}
		</button>
	</div>
</div>
