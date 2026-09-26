<script lang="ts">
	import { onMount } from 'svelte'
	import { beatportStore, beatportCartCount, beatportCartDuration } from '$shared/stores/beatport'
	import { formatDurationCompact } from '$shared/utils/format'
	import BeatportSidebar from './BeatportSidebar.svelte'
	import BeatportTrackRow from './BeatportTrackRow.svelte'
	import BeatportCartDrawer from './BeatportCartDrawer.svelte'
	import BeatportLoginModal from './BeatportLoginModal.svelte'
	import { Text, Icon, Spinner } from '$lib/components/common'

	let searchInput = $state('')
	let selectedSearchType = $state<'tracks' | 'releases' | 'artists' | 'playlists'>('tracks')
	let isCartOpen = $state(false)

	onMount(() => {
		beatportStore.loadInitialData()
	})

	function handleSearchSubmit(e: SubmitEvent) {
		e.preventDefault()
		if (searchInput.trim()) {
			beatportStore.search(searchInput.trim(), selectedSearchType)
		}
	}

	function handleClearSearch() {
		searchInput = ''
		beatportStore.clearSearch()
	}

	function handleAddAllVisibleToCart() {
		const tracksToAdd = displayTracks
		beatportStore.addBulkToCart(tracksToAdd)
	}

	let displayTracks = $derived.by(() => {
		if ($beatportStore.searchQuery.trim()) {
			return $beatportStore.searchResults
		}
		return $beatportStore.currentSectionTracks
	})

	let sectionTitle = $derived.by(() => {
		if ($beatportStore.searchQuery.trim()) {
			return `Résultats pour "${$beatportStore.searchQuery}" (${displayTracks.length} titres)`
		}
		if ($beatportStore.navSection === 'artist') {
			return `Artiste : ${$beatportStore.selectedArtistName || 'Artiste Beatport'}`
		}
		if ($beatportStore.navSection === 'chart') {
			return `Chart : ${$beatportStore.selectedChartTitle || 'Curated Chart'}`
		}
		if ($beatportStore.navSection === 'playlist') {
			return $beatportStore.selectedPlaylistName || 'Playlist Beatport'
		}
		if ($beatportStore.navSection === 'favorites') {
			return 'Mes Favoris Beatport'
		}
		if ($beatportStore.navSection === 'purchased') {
			return 'Titres achetés (Purchases)'
		}
		if ($beatportStore.navSection === 'offline') {
			return 'Bibliothèque Hors-Ligne (Offline)'
		}
		if ($beatportStore.selectedGenreName) {
			return `Top Tracks - ${$beatportStore.selectedGenreName}`
		}
		return 'Beatport Catalog Tracks'
	})
</script>

<div class="flex h-full w-full overflow-hidden bg-surface-0">
	<!-- Left Beatport Sidebar -->
	<div class="w-60 flex-shrink-0">
		<BeatportSidebar />
	</div>

	<!-- Main Beatport Content Area -->
	<div class="flex flex-1 flex-col overflow-y-auto pb-28">
		<!-- Top Bar -->
		<div
			class="sticky top-0 z-20 flex items-center justify-between border-b border-stroke/80 bg-surface-1/90 px-6 py-3 backdrop-blur-md"
		>
			<div class="flex items-center gap-2.5">
				<Icon name="beatport" class="h-5 w-5 text-text-primary" />
				<h1 class="text-base font-bold text-text-primary">Beatport Streaming</h1>
			</div>

			{#if $beatportStore.auth.is_authenticated}
				<!-- Search Bar -->
				<form onsubmit={handleSearchSubmit} class="flex items-center gap-2">
					<div class="relative flex h-8 items-center">
						<Icon name="search" class="absolute left-2.5 h-3.5 w-3.5 text-text-tertiary" />
						<input
							type="text"
							placeholder="Rechercher..."
							bind:value={searchInput}
							class="h-8 w-64 rounded-l-lg border border-stroke bg-surface-2 pr-3 pl-8 text-xs text-text-primary placeholder:text-text-tertiary focus:border-emerald-500 focus:outline-none"
						/>
						<select
							bind:value={selectedSearchType}
							class="h-8 cursor-pointer rounded-r-lg border-y border-r border-stroke bg-surface-3 px-2.5 text-xs text-text-secondary focus:border-emerald-500 focus:outline-none"
						>
							<option value="tracks">Catalogue Global</option>
							<option value="artists">Artistes</option>
							<option value="releases">Releases</option>
						</select>
					</div>

					<button
						type="submit"
						class="h-8 cursor-pointer rounded-lg bg-[#00FF96] px-3.5 text-xs font-bold text-black shadow-md shadow-[#00FF96]/10 transition-all hover:bg-[#00e687] active:scale-95"
					>
						Rechercher
					</button>

					{#if $beatportStore.searchQuery.trim()}
						<button
							type="button"
							class="rounded-lg border border-stroke bg-surface-2 px-2.5 py-1.5 text-xs text-text-secondary hover:text-text-primary"
							onclick={handleClearSearch}
						>
							✕
						</button>
					{/if}

					{#if $beatportCartCount > 0}
						<button
							type="button"
							class="ml-2 flex flex-shrink-0 cursor-pointer items-center gap-2 rounded-lg border border-emerald-500/40 bg-emerald-500/10 px-3.5 py-1.5 text-xs font-bold text-emerald-400 shadow-md shadow-emerald-500/10 transition-all hover:bg-emerald-500/20 active:scale-95"
							onclick={() => (isCartOpen = true)}
							title="Ouvrir le panier"
						>
							<Icon name="cart" class="h-4 w-4 text-emerald-400" />
							<span>Panier ({$beatportCartCount})</span>
							<span class="text-[11px] text-text-tertiary">· {formatDurationCompact($beatportCartDuration)}</span>
						</button>
					{/if}
				</form>
			{/if}
		</div>

		<div class="p-6">
			<!-- Unauthenticated State: Connection Gateway -->
			{#if !$beatportStore.auth.is_authenticated}
				<div class="mx-auto max-w-3xl space-y-8 py-8">
					<div
						class="relative space-y-6 overflow-hidden rounded-3xl border border-emerald-500/30 bg-gradient-to-b from-[#0e1713] to-[#121418] p-8 text-center shadow-2xl shadow-emerald-950/20"
					>
						<div
							class="mx-auto flex h-16 w-16 items-center justify-center rounded-2xl border border-emerald-500/20 bg-emerald-500/10 shadow-inner"
						>
							<Icon name="beatport" class="h-9 w-9 text-[#00FF96]" />
						</div>

						<div class="mx-auto max-w-xl space-y-2">
							<h2 class="text-2xl font-extrabold tracking-tight text-white">Intégration Beatport Streaming</h2>
							<p class="text-xs leading-relaxed text-neutral-300">
								Connectez votre abonnement Beatport pour accéder à votre bibliothèque en streaming, synchroniser vos
								playlists personnelles et télécharger vos morceaux via BeatportDL avec synchronisation Mixed In Key &
								Crate.
							</p>
						</div>

						<div class="mx-auto flex max-w-sm flex-col gap-3 pt-2 sm:flex-row">
							<button
								type="button"
								class="flex flex-1 cursor-pointer items-center justify-center gap-2 rounded-xl bg-[#00FF96] px-4 py-3 text-xs font-bold text-black shadow-lg shadow-[#00FF96]/20 transition-all hover:scale-105 hover:bg-[#00e687]"
								onclick={() => beatportStore.openLoginModal()}
							>
								<Icon name="link" class="h-4 w-4" />
								<span>Se connecter avec Beatport</span>
							</button>
						</div>
					</div>
				</div>
			{:else}
				<!-- Authenticated Content -->
				<div class="space-y-6">
					<!-- Error / Expired Session Notice -->
					{#if $beatportStore.error}
						<div
							class="flex flex-col items-center justify-between gap-3 rounded-xl border border-amber-500/40 bg-amber-950/40 p-4 text-xs text-amber-200 shadow-lg sm:flex-row"
						>
							<div class="flex items-center gap-3">
								<Icon name="alert-triangle" class="h-5 w-5 flex-shrink-0 text-amber-400" />
								<div>
									<div class="font-bold text-amber-300">Session ou connexion Beatport à renouveler</div>
									<div class="text-[11px] text-neutral-300">{$beatportStore.error}</div>
								</div>
							</div>
							<button
								type="button"
								class="flex flex-shrink-0 cursor-pointer items-center gap-1.5 rounded-lg bg-[#00FF96] px-4 py-2 text-xs font-bold text-black shadow-md shadow-[#00FF96]/20 transition-all hover:bg-[#00e687]"
								onclick={() => beatportStore.openLoginModal()}
							>
								<Icon name="refresh-cw" class="h-3.5 w-3.5" />
								<span>Renouveler ma session Beatport</span>
							</button>
						</div>
					{/if}

					<!-- Artist Page Header (if viewing artist) -->
					{#if $beatportStore.navSection === 'artist'}
						<div
							class="relative overflow-hidden rounded-2xl border border-emerald-500/30 bg-gradient-to-r from-emerald-950/40 via-surface-1 to-surface-2 p-6 shadow-lg"
						>
							<div class="flex flex-col items-center gap-6 sm:flex-row">
								<div
									class="relative h-28 w-28 flex-shrink-0 overflow-hidden rounded-full border-2 border-[#00FF96] bg-surface-3 shadow-xl shadow-[#00FF96]/20"
								>
									{#if $beatportStore.selectedArtistImage}
										<img
											src={$beatportStore.selectedArtistImage}
											alt={$beatportStore.selectedArtistName || 'Artist'}
											class="h-full w-full object-cover"
										/>
									{:else}
										<div class="flex h-full w-full items-center justify-center text-[#00FF96]">
											<Icon name="user" class="h-12 w-12" />
										</div>
									{/if}
								</div>

								<div class="flex-1 space-y-2 text-center sm:text-left">
									<div class="flex flex-wrap items-center justify-center gap-3 sm:justify-start">
										<h2 class="text-2xl font-extrabold text-white">
											{$beatportStore.selectedArtistName}
										</h2>
										<span
											class="rounded-full border border-[#00FF96]/40 bg-[#00FF96]/20 px-2.5 py-0.5 text-[10px] font-bold tracking-wider text-[#00FF96] uppercase"
										>
											Artiste Beatport
										</span>
									</div>

									{#if $beatportStore.selectedArtistDetail?.genres && $beatportStore.selectedArtistDetail.genres.length > 0}
										<div class="flex flex-wrap items-center justify-center gap-1.5 pt-1 sm:justify-start">
											{#each $beatportStore.selectedArtistDetail.genres as g, gIdx (gIdx)}
												<span
													class="rounded border border-stroke bg-surface-3 px-2 py-0.5 text-[10px] text-text-secondary"
												>
													{g}
												</span>
											{/each}
										</div>
									{/if}

									<div class="flex flex-wrap items-center justify-center gap-3 pt-2 sm:justify-start">
										<button
											type="button"
											class="flex items-center gap-1.5 rounded-lg border border-stroke bg-surface-3 px-3 py-1 text-xs text-text-secondary transition-colors hover:text-white"
											onclick={() => beatportStore.setNavSection('home')}
										>
											← Retour au Catalogue
										</button>

										{#if displayTracks.length > 0}
											<button
												type="button"
												class="flex items-center gap-1.5 rounded-lg bg-[#00FF96] px-3.5 py-1 text-xs font-bold text-black shadow-md shadow-[#00FF96]/10 transition-all hover:bg-[#00e687]"
												onclick={handleAddAllVisibleToCart}
											>
												<Icon name="plus" class="h-3.5 w-3.5" />
												<span>Ajouter toute la discographie au panier ({displayTracks.length})</span>
											</button>
										{/if}
									</div>
								</div>
							</div>
						</div>
					{/if}

					<!-- Search Results: Matching Artists Horizontal Row -->
					{#if $beatportStore.searchQuery.trim() && $beatportStore.searchArtists.length > 0}
						<div class="space-y-3">
							<h3 class="text-xs font-bold tracking-wider text-text-tertiary uppercase">
								Artistes correspondants ({$beatportStore.searchArtists.length})
							</h3>
							<div class="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5">
								{#each $beatportStore.searchArtists as artist (artist.id)}
									<button
										type="button"
										class="group flex cursor-pointer flex-col items-center space-y-2 rounded-xl border border-stroke/60 bg-surface-1 p-3 text-center transition-all hover:border-[#00FF96]/60 hover:bg-surface-2"
										onclick={() => beatportStore.setNavArtist(artist.id, artist.name, artist.image_url ?? undefined)}
									>
										<div
											class="relative h-16 w-16 overflow-hidden rounded-full border border-stroke bg-surface-3 shadow-md transition-colors group-hover:border-[#00FF96]"
										>
											{#if artist.image_url}
												<img
													src={artist.image_url}
													alt={artist.name}
													class="h-full w-full object-cover transition-transform group-hover:scale-105"
												/>
											{:else}
												<div class="flex h-full w-full items-center justify-center text-text-tertiary">
													<Icon name="user" class="h-7 w-7" />
												</div>
											{/if}
										</div>
										<div class="w-full min-w-0">
											<div class="truncate text-xs font-bold text-text-primary group-hover:text-[#00FF96]">
												{artist.name}
											</div>
											<div class="text-[10px] text-text-tertiary">Voir discographie →</div>
										</div>
									</button>
								{/each}
							</div>
						</div>
					{/if}

					{#if $beatportStore.loading}
						<div class="flex h-40 items-center justify-center gap-2 text-text-secondary">
							<Spinner class="h-5 w-5 text-emerald-400" />
							<span class="text-xs">Chargement de Beatport...</span>
						</div>
					{:else}
						<!-- Featured Curated Charts (if home and not searching) -->
						{#if $beatportStore.navSection === 'home' && !$beatportStore.searchQuery.trim() && $beatportStore.charts.length > 0}
							<div>
								<h2 class="mb-3 text-sm font-bold text-text-primary">Charts & Curations</h2>
								<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
									{#each $beatportStore.charts as chart (chart.id)}
										<button
											type="button"
											class="group relative flex cursor-pointer items-center gap-4 rounded-xl border border-stroke/60 bg-surface-1 p-3.5 text-left transition-all hover:scale-[1.02] hover:border-emerald-500/50 hover:bg-surface-2/80"
											onclick={() => beatportStore.setNavChart(String(chart.id), chart.title)}
										>
											<div class="relative h-16 w-16 flex-shrink-0 overflow-hidden rounded-lg bg-surface-3 shadow-md">
												{#if chart.image_url}
													<img
														src={chart.image_url}
														alt={chart.title}
														class="h-full w-full object-cover transition-transform group-hover:scale-105"
													/>
												{:else}
													<div class="flex h-full w-full items-center justify-center text-text-tertiary">
														<Icon name="music" class="h-6 w-6" />
													</div>
												{/if}
											</div>
											<div class="min-w-0 flex-1">
												<h3 class="truncate text-xs font-bold text-text-primary group-hover:text-emerald-400">
													{chart.title}
												</h3>
												{#if chart.description}
													<p class="mt-0.5 line-clamp-2 text-[11px] text-text-secondary">{chart.description}</p>
												{/if}
												{#if chart.tracks_count}
													<span class="mt-1 inline-block font-mono text-[10px] text-emerald-400">
														{chart.tracks_count} titres
													</span>
												{/if}
											</div>
										</button>
									{/each}
								</div>
							</div>
						{/if}

						<!-- Tracks Section -->
						<div>
							<div class="mb-3 flex items-center justify-between">
								<div class="flex items-center gap-2">
									<h2 class="text-sm font-bold text-text-primary">
										{sectionTitle}
									</h2>
									{#if $beatportStore.selectedGenreName || $beatportStore.navSection === 'chart' || $beatportStore.navSection === 'artist'}
										<button
											type="button"
											class="cursor-pointer rounded border border-stroke bg-surface-2 px-2 py-0.5 text-[10px] text-text-secondary hover:text-text-primary"
											onclick={() => beatportStore.setNavSection('home')}
										>
											Voir tout le catalogue ✕
										</button>
									{/if}
								</div>

								<div class="flex items-center gap-3">
									{#if displayTracks.length > 0}
										<button
											type="button"
											class="flex cursor-pointer items-center gap-1.5 rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-2.5 py-1 text-xs font-medium text-emerald-400 transition-colors hover:bg-emerald-500/20"
											onclick={handleAddAllVisibleToCart}
										>
											<Icon name="plus" class="h-3.5 w-3.5" />
											<span>Ajouter tout au panier ({displayTracks.length})</span>
										</button>
									{/if}
								</div>
							</div>

							<!-- Tracks Table -->
							<div class="overflow-hidden rounded-xl border border-stroke/60 bg-surface-1 shadow-sm">
								<!-- Table Header -->
								<div
									class="grid grid-cols-[36px_40px_1fr_120px_100px_60px_64px_50px_100px] items-center gap-2 border-b border-stroke/80 bg-surface-2/60 px-3 py-2 text-[11px] font-bold tracking-wider text-text-tertiary uppercase select-none"
								>
									<div class="text-center">#</div>
									<div></div>
									<div>Titre & Artiste</div>
									<div>Genre</div>
									<div>Date</div>
									<div>Durée</div>
									<div>Tonalité</div>
									<div>BPM</div>
									<div class="pr-2 text-right">Actions</div>
								</div>

								<!-- Table Rows -->
								<div class="divide-y divide-stroke/30">
									{#if displayTracks.length === 0}
										<div class="p-8 text-center text-xs text-text-tertiary">
											Aucun morceau trouvé dans cette section.
										</div>
									{:else}
										{#each displayTracks as track, idx (track.id)}
											<BeatportTrackRow {track} index={idx + 1} />
										{/each}
									{/if}
								</div>
							</div>
						</div>

						<!-- Genres Grid Section (if home and not searching) -->
						{#if $beatportStore.navSection === 'home' && !$beatportStore.searchQuery.trim() && $beatportStore.genres.length > 0}
							<div>
								<h2 class="mb-3 text-sm font-bold text-text-primary">Genres Électroniques</h2>
								<div class="grid grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-4">
									{#each $beatportStore.genres as genre (genre.id)}
										<button
											type="button"
											class="flex items-center justify-between rounded-lg border border-stroke/60 bg-surface-1 px-3.5 py-2.5 text-left text-xs font-medium text-text-secondary transition-all hover:border-emerald-500/40 hover:bg-surface-2 hover:text-text-primary {$beatportStore.selectedGenreId ===
											genre.id
												? 'border-emerald-500 bg-emerald-500/10 font-semibold text-emerald-400'
												: ''}"
											onclick={() => beatportStore.selectGenre(genre.id, genre.slug, genre.name)}
										>
											<span class="truncate">{genre.name}</span>
											<Icon name="chevron-right" class="h-3.5 w-3.5 flex-shrink-0 text-text-tertiary" />
										</button>
									{/each}
								</div>
							</div>
						{/if}
					{/if}
				</div>
			{/if}
		</div>
	</div>

	<!-- Cart / Selection Slide-over Drawer -->
	{#if $beatportStore.auth.is_authenticated}
		<BeatportCartDrawer bind:isOpen={isCartOpen} onClose={() => (isCartOpen = false)} />
	{/if}

	<!-- Login Modal -->
	{#if $beatportStore.showLoginModal}
		<BeatportLoginModal />
	{/if}
</div>
