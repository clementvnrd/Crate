<script lang="ts">
	import {
		albumsStore,
		playerAlbums,
		albumsLoading,
		albumsAdding,
		appDataDir,
	} from '$lib/stores'
	import type { PlayerAlbum } from '$shared/types'
	import { getArtworkUrl } from '$shared/utils'
	import { Icon, Tooltip } from '$lib/components/common'

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

	function handleRemoveAlbum(e: MouseEvent, albumId: string) {
		e.stopPropagation()
		if (confirm('Voulez-vous retirer cet album du lecteur Crate ? (Les fichiers sur votre disque ne seront pas supprimés)')) {
			albumsStore.removeAlbum(albumId)
		}
	}
</script>

<div class="flex flex-col h-full w-full overflow-y-auto px-6 py-4">
	<!-- Top Bar: Header & Add Album Button -->
	<div class="flex items-center justify-between pb-5 border-b border-stroke-subtle flex-shrink-0">
		<div class="flex items-center gap-3">
			<div class="flex h-9 w-9 items-center justify-center rounded-xl bg-cyan-500/10 border border-cyan-500/20 text-cyan-600 dark:text-cyan-400 shadow-xs">
				<Icon name="disc" class="h-5 w-5" />
			</div>
			<div>
				<h2 class="text-base font-bold text-text-primary flex items-center gap-2">
					<span>Mes Albums</span>
					<span class="text-xs px-2 py-0.5 rounded-full bg-surface-2 border border-stroke-subtle font-mono text-text-secondary font-medium">
						{$playerAlbums.length}
					</span>
				</h2>
				<p class="text-xs text-text-tertiary">
					Explorez vos dossiers musicaux organisés par albums avec leurs pochettes et métadonnées
				</p>
			</div>
		</div>

		<button
			type="button"
			class="flex items-center gap-2 rounded-xl bg-cyan-500 hover:bg-cyan-400 text-black px-3.5 py-2 text-xs font-bold shadow-lg shadow-cyan-500/20 active:scale-95 transition-all cursor-pointer disabled:opacity-50"
			onclick={handleAddAlbum}
			disabled={$albumsAdding}
		>
			<Icon name={$albumsAdding ? 'loader' : 'folder-plus'} class="h-4 w-4 {$albumsAdding ? 'animate-spin' : ''}" />
			<span>{$albumsAdding ? 'Importation...' : 'Ajouter un dossier / album'}</span>
		</button>
	</div>

	<!-- Main Grid Content -->
	<div class="flex-1 py-6">
		{#if $albumsLoading}
			<div class="flex h-48 items-center justify-center text-text-tertiary gap-2.5">
				<Icon name="loader" class="h-5 w-5 animate-spin text-cyan-500" />
				<span class="text-sm">Chargement des albums...</span>
			</div>
		{:else if $playerAlbums.length === 0}
			<!-- Empty State -->
			<div class="flex flex-col items-center justify-center py-16 text-center max-w-md mx-auto">
				<div class="flex h-20 w-20 items-center justify-center rounded-3xl bg-surface-1 border border-stroke-subtle mb-4 text-cyan-600 dark:text-cyan-400 shadow-inner">
					<Icon name="disc" class="h-10 w-10 opacity-70" />
				</div>
				<h3 class="text-base font-bold text-text-primary">Aucun album importé</h3>
				<p class="text-xs text-text-secondary mt-1.5 leading-relaxed">
					Ajoutez un dossier contenant vos fichiers audio pour parcourir votre collection sous forme d'albums avec leurs pochettes intégrées.
				</p>
				<button
					type="button"
					class="mt-5 flex items-center gap-2 rounded-xl bg-cyan-500 hover:bg-cyan-400 text-black px-4 py-2.5 text-xs font-bold shadow-md shadow-cyan-500/20 active:scale-95 transition-all cursor-pointer"
					onclick={handleAddAlbum}
				>
					<Icon name="folder-plus" class="h-4 w-4" />
					<span>Ajouter un dossier d'album</span>
				</button>
			</div>
		{:else}
			<!-- Apple Music Style Album Grid -->
			<div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-6 pb-12">
				{#each $playerAlbums as album (album.id)}
					{@const artUrl = album.artwork_path ? getArtworkUrl(album.artwork_path, $appDataDir) : null}

					<!-- Album Card -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<div
						class="group flex flex-col cursor-pointer"
						onclick={() => onSelectAlbum(album)}
					>
						<!-- Square Artwork Container -->
						<div class="relative aspect-square w-full rounded-2xl overflow-hidden bg-surface-2/70 border border-stroke-subtle shadow-md group-hover:shadow-2xl group-hover:border-cyan-500/30 transition-all duration-300">
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
							<div class="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-black/20 opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none"></div>

							<!-- Floating Play Button on Hover -->
							<button
								type="button"
								class="absolute bottom-3 right-3 flex h-10 w-10 items-center justify-center rounded-full bg-cyan-400 text-black shadow-lg shadow-cyan-400/50 opacity-0 translate-y-2 group-hover:opacity-100 group-hover:translate-y-0 transition-all duration-200 hover:scale-110 active:scale-95 cursor-pointer z-10"
								onclick={(e) => handlePlayAlbumDirect(e, album)}
								aria-label="Lire l'album"
							>
								<Icon name="play" class="h-4 w-4 ml-0.5" fill />
							</button>

							<!-- Delete/Remove Icon Button on Top Right -->
							<button
								type="button"
								class="absolute top-2.5 right-2.5 flex h-7 w-7 items-center justify-center rounded-full bg-black/60 backdrop-blur-md text-text-tertiary hover:text-red-400 hover:bg-black/80 opacity-0 group-hover:opacity-100 transition-all duration-200 active:scale-95 cursor-pointer z-10"
								onclick={(e) => handleRemoveAlbum(e, album.id)}
								title="Retirer de la liste"
							>
								<Icon name="trash" class="h-3.5 w-3.5" />
							</button>

							<!-- Track Count Badge on Bottom Left -->
							<div class="absolute bottom-2.5 left-2.5 px-2 py-0.5 rounded-md bg-black/60 backdrop-blur-md text-[10px] font-mono font-medium text-white/90 opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none">
								{album.track_count} {album.track_count > 1 ? 'pistes' : 'piste'}
							</div>
						</div>

						<!-- Album Title & Artist -->
						<div class="mt-2.5 min-w-0">
							<h3 class="text-sm font-bold text-text-primary group-hover:text-cyan-600 dark:group-hover:text-cyan-400 transition-colors truncate" title={album.title}>
								{album.title}
							</h3>
							<p class="text-xs font-medium text-text-secondary truncate mt-0.5" title={album.artist}>
								{album.artist}
							</p>
							<div class="flex items-center gap-1.5 text-[10px] text-text-tertiary mt-1">
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
