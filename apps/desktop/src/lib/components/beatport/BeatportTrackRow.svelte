<script lang="ts">
	import type { BeatportTrack } from '$shared/types/beatport'
	import { beatportStore } from '$shared/stores/beatport'
	import { playerStore, isPlaying as isPlayerPlaying, beatportTrack, playbackSource } from '$lib/stores'
	import { getCamelotColor, formatCamelotKey } from '$shared/utils/camelot'
	import { displaySettingsStore } from '$shared/stores/displaySettings'
	import { Icon } from '$lib/components/common'

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

	let camelotInfo = $derived(track.key ? getCamelotColor(track.key) : null)
	let formattedKey = $derived(track.key ? formatCamelotKey(track.key, $displaySettingsStore.camelotZeroPadding) : '-')

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

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="group grid grid-cols-[36px_40px_1fr_120px_100px_60px_64px_50px_100px] items-center gap-2 px-3 py-1.5 text-xs transition-colors select-none {isCurrentTrack
		? 'border-l-2 border-l-[#00FF96] bg-emerald-950/40'
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
					class="flex h-6 w-6 cursor-pointer items-center justify-center rounded-full bg-[#00FF96] text-black shadow-lg shadow-[#00FF96]/30 transition-transform active:scale-95"
					onclick={togglePlay}
					title="Pause preview"
				>
					<Icon name="pause" class="h-3 w-3" fill />
				</button>
			{:else}
				<div class="flex h-5 w-5 items-end justify-center gap-[2px] pb-0.5" title="En cours de lecture">
					<span class="h-3 w-[3px] animate-pulse rounded-full bg-[#00FF96]"></span>
					<span class="h-4.5 w-[3px] animate-pulse rounded-full bg-[#00FF96] [animation-delay:150ms]"></span>
					<span class="h-2.5 w-[3px] animate-pulse rounded-full bg-[#00FF96] [animation-delay:300ms]"></span>
				</div>
			{/if}
		{:else if isHovered || isCurrentTrack}
			<button
				type="button"
				class="flex h-6 w-6 cursor-pointer items-center justify-center rounded-full bg-emerald-500/20 text-[#00FF96] shadow-sm transition-all hover:bg-[#00FF96] hover:text-black active:scale-95"
				onclick={togglePlay}
				title="Play preview"
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
				class="bg-surface-3 flex h-8 w-8 items-center justify-center rounded-md border border-stroke/40 text-[10px] font-bold text-[#00FF96]"
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
				class="cursor-pointer truncate text-left hover:text-[#00FF96] {isCurrentTrack
					? 'font-semibold text-[#00FF96]'
					: ''}"
				onclick={() => playerStore.playBeatport(track)}
			>
				{track.title}
			</button>
			{#if track.mix_name}
				<span class="truncate text-[11px] text-text-tertiary">({track.mix_name})</span>
			{/if}
		</div>
		<div class="truncate text-[11px] text-text-secondary">
			{#if track.artists && track.artists.length > 0}
				{#each track.artists as artist, aIdx (aIdx)}
					<button
						type="button"
						class="cursor-pointer transition-colors hover:text-[#00FF96] hover:underline"
						onclick={(e) => {
							e.stopPropagation()
							beatportStore.setNavArtist(artist.id, artist.name, artist.image_url ?? undefined)
						}}
					>
						{artist.name}
					</button>{#if aIdx < track.artists.length - 1},&nbsp;{/if}
				{/each}
			{:else}
				<span class="text-text-tertiary">Beatport Artist</span>
			{/if}
		</div>
	</div>

	<!-- Genre -->
	<div class="truncate text-text-secondary">
		{track.genre}
	</div>

	<!-- Released -->
	<div class="truncate font-mono text-[11px] text-text-tertiary">
		{track.release_date}
	</div>

	<!-- Length -->
	<div class="font-mono text-text-secondary tabular-nums">
		{track.duration_formatted}
	</div>

	<!-- Key (Camelot Colored Pill) -->
	<div class="flex items-center">
		{#if camelotInfo}
			<span
				class="relative inline-flex h-[20px] w-10 items-center justify-center rounded font-mono text-[11px] font-bold tracking-tight shadow-sm select-none"
				style="background-color: {camelotInfo.bg}; color: {camelotInfo.text};"
				title="{formattedKey} ({camelotInfo.name}) • Beatport / Camelot"
			>
				{formattedKey}
			</span>
		{:else}
			<span class="font-mono text-[11px] text-text-tertiary">{formattedKey}</span>
		{/if}
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
			class="hover:bg-surface-3 flex h-6 w-6 cursor-pointer items-center justify-center rounded transition-colors {inCart
				? 'font-bold text-emerald-400'
				: 'text-text-tertiary hover:text-text-primary'}"
			onclick={toggleCart}
			title={inCart ? 'Retirer du panier' : 'Ajouter au panier'}
		>
			<Icon name="cart" class="h-3.5 w-3.5" />
		</button>

		<!-- Favorite -->
		<button
			type="button"
			class="hover:bg-surface-3 flex h-6 w-6 cursor-pointer items-center justify-center rounded transition-colors {isFavorite
				? 'font-bold text-red-500'
				: 'text-text-tertiary hover:text-red-400'}"
			onclick={toggleFav}
			title={isFavorite ? 'Retirer des favoris' : 'Ajouter aux favoris Beatport'}
		>
			<Icon name="heart" class="h-3.5 w-3.5" />
		</button>

		<!-- Add to Playlist / Cart Checkmark -->
		<button
			type="button"
			class="hover:bg-surface-3 flex h-6 w-6 cursor-pointer items-center justify-center rounded text-text-tertiary transition-colors hover:text-text-primary"
			onclick={toggleCart}
			title={inCart ? 'Dans le panier (cliquer pour retirer)' : 'Ajouter au panier'}
		>
			{#if inCart}
				<Icon name="check" class="h-3.5 w-3.5 text-[#00FF96]" />
			{:else}
				<Icon name="plus" class="h-3.5 w-3.5" />
			{/if}
		</button>
	</div>
</div>
