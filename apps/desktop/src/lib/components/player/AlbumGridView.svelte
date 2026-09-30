<script lang="ts">
	import { albumsStore, playerAlbums, albumsLoading, albumsAdding, appDataDir } from '$lib/stores'
	import type { PlayerAlbum } from '$shared/types'
	import { getArtworkUrl } from '$shared/utils'
	import { translate } from '$shared/i18n'
	import { Button, ConfirmModal, Icon, Spinner } from '$lib/components/common'

	interface Props {
		onSelectAlbum: (album: PlayerAlbum) => void
	}

	let { onSelectAlbum }: Props = $props()

	function handleAddAlbum() {
		albumsStore.addAlbumFromDialog()
	}

	function handlePlayAlbumDirect(e: MouseEvent, album: PlayerAlbum) {
		e.stopPropagation()
		albumsStore.playAlbum(album, false)
	}

	let albumToRemove = $state<string | null>(null)

	function handleRemoveAlbum(e: MouseEvent, albumId: string) {
		e.stopPropagation()
		albumToRemove = albumId
	}

	function confirmRemoveAlbum() {
		if (albumToRemove) albumsStore.removeAlbum(albumToRemove)
		albumToRemove = null
	}
</script>

<div class="flex h-full w-full flex-col overflow-y-auto px-6 py-4">
	<!-- Top Bar: Header & Add Album Button -->
	<div class="flex flex-shrink-0 items-center justify-between border-b border-stroke-subtle pb-5">
		<div class="flex items-center gap-3">
			<div
				class="flex h-9 w-9 items-center justify-center rounded-xl border border-cyan-500/20 bg-cyan-500/10 text-cyan-600 shadow-xs dark:text-cyan-400"
			>
				<Icon name="disc" class="h-5 w-5" />
			</div>
			<div>
				<h2 class="flex items-center gap-2 text-base font-bold text-text-primary">
					<span>Mes Albums</span>
					<span
						class="rounded-full border border-stroke-subtle bg-surface-2 px-2 py-0.5 font-mono text-xs font-medium text-text-secondary"
					>
						{$playerAlbums.length}
					</span>
				</h2>
				<p class="text-xs text-text-tertiary">
					Explorez vos dossiers musicaux organisés par albums avec leurs pochettes et métadonnées
				</p>
			</div>
		</div>

		<Button
			variant="primary"
			tone="deck"
			size="bare"
			glow="lg/20"
			press
			class="gap-2 px-3.5 py-2 text-xs"
			onclick={handleAddAlbum}
			disabled={$albumsAdding}
		>
			{#if $albumsAdding}
				<Spinner icon="loader" class="h-4 w-4" color="current" />
			{:else}
				<Icon name="folder-plus" class="h-4 w-4" />
			{/if}
			<span>{$albumsAdding ? 'Importation...' : 'Ajouter un dossier / album'}</span>
		</Button>
	</div>

	<!-- Main Grid Content -->
	<div class="flex-1 py-6">
		{#if $albumsLoading}
			<div class="flex h-48 items-center justify-center gap-2.5 text-text-tertiary">
				<Spinner icon="loader" class="h-5 w-5 text-cyan-500" color="current" />
				<span class="text-sm">Chargement des albums...</span>
			</div>
		{:else if $playerAlbums.length === 0}
			<!-- Empty State -->
			<div class="mx-auto flex max-w-md flex-col items-center justify-center py-16 text-center">
				<div
					class="mb-4 flex h-20 w-20 items-center justify-center rounded-3xl border border-stroke-subtle bg-surface-1 text-cyan-600 shadow-inner dark:text-cyan-400"
				>
					<Icon name="disc" class="h-10 w-10 opacity-70" />
				</div>
				<h3 class="text-base font-bold text-text-primary">Aucun album importé</h3>
				<p class="mt-1.5 text-xs leading-relaxed text-text-secondary">
					Ajoutez un dossier contenant vos fichiers audio pour parcourir votre collection sous forme d'albums avec leurs
					pochettes intégrées.
				</p>
				<Button
					variant="primary"
					tone="deck"
					size="bare"
					glow="md/20"
					press
					class="mt-5 gap-2 px-4 py-2.5 text-xs"
					onclick={handleAddAlbum}
				>
					<Icon name="folder-plus" class="h-4 w-4" />
					<span>Ajouter un dossier d'album</span>
				</Button>
			</div>
		{:else}
			<!-- Apple Music Style Album Grid -->
			<div class="grid grid-cols-2 gap-6 pb-12 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
				{#each $playerAlbums as album (album.id)}
					{@const artUrl = album.artwork_path ? getArtworkUrl(album.artwork_path, $appDataDir) : null}

					<!-- Album Card -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<div class="group flex cursor-pointer flex-col" onclick={() => onSelectAlbum(album)}>
						<!-- Square Artwork Container -->
						<div
							class="relative aspect-square w-full overflow-hidden rounded-2xl border border-stroke-subtle bg-surface-2/70 shadow-md transition-all duration-300 group-hover:border-cyan-500/30 group-hover:shadow-2xl"
						>
							{#if artUrl}
								<img
									src={artUrl}
									alt={album.title}
									class="h-full w-full object-cover transition-transform duration-500 group-hover:scale-105"
								/>
							{:else}
								<div class="flex h-full w-full items-center justify-center bg-surface-2 text-text-tertiary">
									<Icon name="disc" class="h-12 w-12 opacity-30" />
								</div>
							{/if}

							<!-- Hover Overlay Gradient -->
							<div
								class="pointer-events-none absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-black/20 opacity-0 transition-opacity duration-200 group-hover:opacity-100"
							></div>

							<!-- Floating Play Button on Hover -->
							<button
								type="button"
								class="absolute right-3 bottom-3 z-10 flex h-10 w-10 translate-y-2 cursor-pointer items-center justify-center rounded-full bg-cyan-400 text-black opacity-0 shadow-lg shadow-cyan-400/50 transition-all duration-200 group-hover:translate-y-0 group-hover:opacity-100 hover:scale-110 active:scale-95"
								onclick={(e) => handlePlayAlbumDirect(e, album)}
								aria-label="Lire l'album"
							>
								<Icon name="play" class="ml-0.5 h-4 w-4" fill />
							</button>

							<!-- Delete/Remove Icon Button on Top Right -->
							<button
								type="button"
								class="absolute top-2.5 right-2.5 z-10 flex h-7 w-7 cursor-pointer items-center justify-center rounded-full bg-black/60 text-text-tertiary opacity-0 backdrop-blur-md transition-all duration-200 group-hover:opacity-100 hover:bg-black/80 hover:text-red-400 active:scale-95"
								onclick={(e) => handleRemoveAlbum(e, album.id)}
								title="Retirer de la liste"
							>
								<Icon name="trash" class="h-3.5 w-3.5" />
							</button>

							<!-- Track Count Badge on Bottom Left -->
							<div
								class="pointer-events-none absolute bottom-2.5 left-2.5 rounded-md bg-black/60 px-2 py-0.5 font-mono text-[10px] font-medium text-white/90 opacity-0 backdrop-blur-md transition-opacity duration-200 group-hover:opacity-100"
							>
								{album.track_count}
								{album.track_count > 1 ? 'pistes' : 'piste'}
							</div>
						</div>

						<!-- Album Title & Artist -->
						<div class="mt-2.5 min-w-0">
							<h3
								class="truncate text-sm font-bold text-text-primary transition-colors group-hover:text-cyan-600 dark:group-hover:text-cyan-400"
								title={album.title}
							>
								{album.title}
							</h3>
							<p class="mt-0.5 truncate text-xs font-medium text-text-secondary" title={album.artist}>
								{album.artist}
							</p>
							<div class="mt-1 flex items-center gap-1.5 text-[10px] text-text-tertiary">
								{#if album.year}
									<span>{album.year}</span>
									{#if album.genre}
										<span>•</span>
									{/if}
								{/if}
								{#if album.genre}
									<span class="truncate">{album.genre}</span>
								{/if}
							</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>

<ConfirmModal
	open={albumToRemove !== null}
	title={$translate('player.albums.removeTitle')}
	message={$translate('player.albums.removeMessage')}
	confirmLabel={$translate('player.albums.removeConfirm')}
	destructive
	onConfirm={confirmRemoveAlbum}
	onCancel={() => (albumToRemove = null)}
/>
