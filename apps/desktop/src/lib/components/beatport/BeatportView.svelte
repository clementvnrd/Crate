<script lang="ts">
	import { onMount } from 'svelte'
	import { beatportStore, beatportCartCount, beatportCartDuration } from '$shared/stores/beatport'
	import { formatDurationCompact } from '$shared/utils/format'
	import BeatportSidebar from './BeatportSidebar.svelte'
	import BeatportTrackRow from './BeatportTrackRow.svelte'
	import BeatportCartDrawer from './BeatportCartDrawer.svelte'
	import BeatportLoginModal from './BeatportLoginModal.svelte'
	import { BEATPORT_DATE_CELL, BEATPORT_GENRE_CELL, BEATPORT_TRACK_GRID } from './trackGrid'
	import { translate } from '$shared/i18n'
	import { Text, Icon, Spinner, Button } from '$lib/components/common'

	let searchInput = $state('')
	type SearchType = 'tracks' | 'releases' | 'artists' | 'playlists'
	let selectedSearchType = $state<SearchType>('tracks')

	const searchTypeOptions = $derived(
		(['tracks', 'artists', 'releases'] as const).map((type) => ({
			value: type,
			label: $translate(`beatport.search.types.${type}`),
		}))
	)
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
			return $translate('beatport.section.results', {
				values: { query: $beatportStore.searchQuery, count: displayTracks.length },
			})
		}
		if ($beatportStore.navSection === 'artist') {
			return $translate('beatport.section.artist', {
				values: { name: $beatportStore.selectedArtistName || $translate('beatport.artist.label') },
			})
		}
		if ($beatportStore.navSection === 'chart') {
			return $translate('beatport.section.chart', {
				values: { name: $beatportStore.selectedChartTitle || $translate('beatport.section.chartFallback') },
			})
		}
		if ($beatportStore.navSection === 'playlist') {
			return $beatportStore.selectedPlaylistName || $translate('beatport.section.playlistFallback')
		}
		if ($beatportStore.navSection === 'favorites') {
			return $translate('beatport.section.favorites')
		}
		if ($beatportStore.navSection === 'purchased') {
			return $translate('beatport.nav.purchased')
		}
		if ($beatportStore.navSection === 'offline') {
			return $translate('beatport.nav.offline')
		}
		if ($beatportStore.selectedGenreName) {
			return $translate('beatport.section.genreTop', { values: { genre: $beatportStore.selectedGenreName } })
		}
		return $translate('beatport.section.catalog')
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
				<h1 class="text-base font-bold text-text-primary">{$translate('beatport.title')}</h1>
			</div>

			{#if $beatportStore.auth.is_authenticated}
				<!-- Search Bar -->
				<form onsubmit={handleSearchSubmit} class="flex items-center gap-2">
					<div class="relative flex h-8 items-center">
						<Icon name="search" class="absolute left-2.5 h-3.5 w-3.5 text-text-tertiary" />
						<input
							type="text"
							placeholder={$translate('beatport.search.placeholder')}
							bind:value={searchInput}
							class="h-8 w-64 rounded-l-lg border border-stroke bg-surface-2 pr-3 pl-8 text-xs text-text-primary placeholder:text-text-tertiary focus:border-beatport-tint focus:outline-none"
						/>
						<!-- Native select joined to the field: its look is part of the frozen Beatport view (CRA-141) -->
						<select
							bind:value={selectedSearchType}
							aria-label={$translate('beatport.search.type')}
							class="h-8 cursor-pointer rounded-r-lg border-y border-r border-stroke bg-surface-3 px-2.5 text-xs text-text-secondary focus:border-beatport-tint focus:outline-none"
						>
							{#each searchTypeOptions as option (option.value)}
								<option value={option.value}>{option.label}</option>
							{/each}
						</select>
					</div>

					<Button
						type="submit"
						variant="primary"
						tone="beatport"
						size="bare"
						shape="lg"
						glow="md/10"
						press
						class="h-8 px-3.5 text-xs"
					>
						{$translate('beatport.search.submit')}
					</Button>

					{#if $beatportStore.searchQuery.trim()}
						<button
							type="button"
							class="rounded-lg border border-stroke bg-surface-2 px-2.5 py-1.5 text-xs text-text-secondary hover:text-text-primary"
							aria-label={$translate('beatport.search.clear')}
							onclick={handleClearSearch}
						>
							✕
						</button>
					{/if}

					{#if $beatportCartCount > 0}
						<button
							type="button"
							class="ml-2 flex flex-shrink-0 cursor-pointer items-center gap-2 rounded-lg border border-beatport-tint/40 bg-beatport-tint/10 px-3.5 py-1.5 text-xs font-bold text-beatport-text shadow-md shadow-beatport-tint/10 transition-all hover:bg-beatport-tint/20 active:scale-95"
							onclick={() => (isCartOpen = true)}
							title={$translate('beatport.cart.open')}
						>
							<Icon name="cart" class="h-4 w-4 text-beatport-text" />
							<span>{$translate('beatport.cart.button', { values: { count: $beatportCartCount } })}</span>
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
						data-surface="beatport"
						class="relative space-y-6 overflow-hidden rounded-3xl border border-beatport-tint/30 bg-gradient-to-b from-beatport-gateway to-surface-1 p-8 text-center shadow-2xl shadow-beatport-wash/20"
					>
						<div
							class="mx-auto flex h-16 w-16 items-center justify-center rounded-2xl border border-beatport-tint/20 bg-beatport-tint/10 shadow-inner"
						>
							<Icon name="beatport" class="h-9 w-9 text-beatport-text-strong" />
						</div>

						<div class="mx-auto max-w-xl space-y-2">
							<h2 class="text-2xl font-extrabold tracking-tight text-text-primary">
								{$translate('beatport.gateway.title')}
							</h2>
							<p class="text-xs leading-relaxed text-beatport-body-text">
								{$translate('beatport.gateway.description')}
							</p>
						</div>

						<div class="mx-auto flex max-w-sm flex-col gap-3 pt-2 sm:flex-row">
							<Button
								variant="primary"
								tone="beatport"
								size="bare"
								glow="lg/20"
								lift
								class="flex-1 gap-2 px-4 py-3 text-xs"
								onclick={() => beatportStore.openLoginModal()}
							>
								<Icon name="link" class="h-4 w-4" />
								<span>{$translate('beatport.gateway.signIn')}</span>
							</Button>
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
									<div class="font-bold text-amber-300">{$translate('beatport.session.title')}</div>
									<div class="text-[11px] text-neutral-300">{$beatportStore.error}</div>
								</div>
							</div>
							<Button
								variant="primary"
								tone="beatport"
								size="bare"
								shape="lg"
								glow="md/20"
								class="flex-shrink-0 gap-1.5 px-4 py-2 text-xs"
								onclick={() => beatportStore.openLoginModal()}
							>
								<Icon name="refresh-cw" class="h-3.5 w-3.5" />
								<span>{$translate('beatport.session.renew')}</span>
							</Button>
						</div>
					{/if}

					<!-- Artist Page Header (if viewing artist) -->
					{#if $beatportStore.navSection === 'artist'}
						<div
							class="relative overflow-hidden rounded-2xl border border-beatport-tint/30 bg-gradient-to-r from-beatport-wash/40 via-surface-1 to-surface-2 p-6 shadow-lg"
						>
							<div class="flex flex-col items-center gap-6 sm:flex-row">
								<div
									class="relative h-28 w-28 flex-shrink-0 overflow-hidden rounded-full border-2 border-beatport bg-surface-3 shadow-xl shadow-beatport/20"
								>
									{#if $beatportStore.selectedArtistImage}
										<img
											src={$beatportStore.selectedArtistImage}
											alt={$beatportStore.selectedArtistName || $translate('beatport.artist.label')}
											class="h-full w-full object-cover"
										/>
									{:else}
										<div class="flex h-full w-full items-center justify-center text-beatport-text-strong">
											<Icon name="user" class="h-12 w-12" />
										</div>
									{/if}
								</div>

								<div class="flex-1 space-y-2 text-center sm:text-left">
									<div class="flex flex-wrap items-center justify-center gap-3 sm:justify-start">
										<h2 class="text-2xl font-extrabold text-text-primary">
											{$beatportStore.selectedArtistName}
										</h2>
										<span
											class="rounded-full border border-beatport/40 bg-beatport/20 px-2.5 py-0.5 text-[10px] font-bold tracking-wider text-beatport-text-strong uppercase"
										>
											{$translate('beatport.artist.label')}
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
											class="flex items-center gap-1.5 rounded-lg border border-stroke bg-surface-3 px-3 py-1 text-xs text-text-secondary transition-colors hover:text-text-primary"
											onclick={() => beatportStore.setNavSection('home')}
										>
											{$translate('beatport.artist.back')}
										</button>

										{#if displayTracks.length > 0}
											<Button
												variant="primary"
												tone="beatport"
												size="bare"
												shape="lg"
												glow="md/10"
												class="gap-1.5 px-3.5 py-1 text-xs"
												onclick={handleAddAllVisibleToCart}
											>
												<Icon name="plus" class="h-3.5 w-3.5" />
												<span
													>{$translate('beatport.artist.addDiscography', {
														values: { count: displayTracks.length },
													})}</span
												>
											</Button>
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
								{$translate('beatport.artist.matching', { values: { count: $beatportStore.searchArtists.length } })}
							</h3>
							<div class="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5">
								{#each $beatportStore.searchArtists as artist (artist.id)}
									<button
										type="button"
										class="group flex cursor-pointer flex-col items-center space-y-2 rounded-xl border border-stroke/60 bg-surface-1 p-3 text-center transition-all hover:border-beatport/60 hover:bg-surface-2"
										onclick={() => beatportStore.setNavArtist(artist.id, artist.name, artist.image_url ?? undefined)}
									>
										<div
											class="relative h-16 w-16 overflow-hidden rounded-full border border-stroke bg-surface-3 shadow-md transition-colors group-hover:border-beatport"
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
											<div class="truncate text-xs font-bold text-text-primary group-hover:text-beatport-text-strong">
												{artist.name}
											</div>
											<div class="text-[10px] text-text-tertiary">{$translate('beatport.artist.viewDiscography')}</div>
										</div>
									</button>
								{/each}
							</div>
						</div>
					{/if}

					{#if $beatportStore.loading}
						<div class="flex h-40 items-center justify-center gap-2 text-text-secondary">
							<Spinner class="h-5 w-5 text-beatport-text" />
							<span class="text-xs">{$translate('beatport.loading')}</span>
						</div>
					{:else}
						<!-- Featured Curated Charts (if home and not searching) -->
						{#if $beatportStore.navSection === 'home' && !$beatportStore.searchQuery.trim() && $beatportStore.charts.length > 0}
							<div>
								<h2 class="mb-3 text-sm font-bold text-text-primary">{$translate('beatport.charts.title')}</h2>
								<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
									{#each $beatportStore.charts as chart (chart.id)}
										<button
											type="button"
											class="group relative flex cursor-pointer items-center gap-4 rounded-xl border border-stroke/60 bg-surface-1 p-3.5 text-left transition-all hover:scale-[1.02] hover:border-beatport-tint/50 hover:bg-surface-2/80"
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
												<h3 class="truncate text-xs font-bold text-text-primary group-hover:text-beatport-text">
													{chart.title}
												</h3>
												{#if chart.description}
													<p class="mt-0.5 line-clamp-2 text-[11px] text-text-secondary">{chart.description}</p>
												{/if}
												{#if chart.tracks_count}
													<span class="mt-1 inline-block font-mono text-[10px] text-beatport-text">
														{$translate('beatport.charts.trackCount', { values: { count: chart.tracks_count } })}
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
											{$translate('beatport.section.showAll')}
										</button>
									{/if}
								</div>

								<div class="flex items-center gap-3">
									{#if displayTracks.length > 0}
										<button
											type="button"
											class="flex cursor-pointer items-center gap-1.5 rounded-lg border border-beatport-tint/30 bg-beatport-tint/10 px-2.5 py-1 text-xs font-medium text-beatport-text transition-colors hover:bg-beatport-tint/20"
											onclick={handleAddAllVisibleToCart}
										>
											<Icon name="plus" class="h-3.5 w-3.5" />
											<span>{$translate('beatport.cart.addAll', { values: { count: displayTracks.length } })}</span>
										</button>
									{/if}
								</div>
							</div>

							<!-- Tracks Table -->
							<!-- A size container: narrow tables drop the date, then the genre, before the title shrinks (trackGrid.ts) -->
							<div class="@container overflow-hidden rounded-xl border border-stroke/60 bg-surface-1 shadow-sm">
								<!-- Table Header -->
								<div
									class="grid {BEATPORT_TRACK_GRID} items-center gap-2 border-b border-stroke/80 bg-surface-2/60 px-3 py-2 text-[11px] font-bold tracking-wider text-text-tertiary uppercase select-none"
								>
									<div class="text-center">#</div>
									<div></div>
									<div>{$translate('beatport.table.title')}</div>
									<div class={BEATPORT_GENRE_CELL}>{$translate('beatport.table.genre')}</div>
									<div class={BEATPORT_DATE_CELL}>{$translate('beatport.table.date')}</div>
									<div>{$translate('beatport.table.length')}</div>
									<div>{$translate('beatport.table.key')}</div>
									<div>{$translate('beatport.table.bpm')}</div>
									<div class="pr-2 text-right">{$translate('beatport.table.actions')}</div>
								</div>

								<!-- Table Rows -->
								<div
									class="divide-y divide-stroke/30"
									role={displayTracks.length > 0 ? 'list' : undefined}
									aria-label={displayTracks.length > 0 ? sectionTitle : undefined}
								>
									{#if displayTracks.length === 0}
										<div class="p-8 text-center text-xs text-text-tertiary">
											{$translate('beatport.table.empty')}
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
								<h2 class="mb-3 text-sm font-bold text-text-primary">{$translate('beatport.genres.title')}</h2>
								<div class="grid grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-4">
									{#each $beatportStore.genres as genre (genre.id)}
										<button
											type="button"
											class="flex items-center justify-between rounded-lg border border-stroke/60 bg-surface-1 px-3.5 py-2.5 text-left text-xs font-medium text-text-secondary transition-all hover:border-beatport-tint/40 hover:bg-surface-2 hover:text-text-primary {$beatportStore.selectedGenreId ===
											genre.id
												? 'border-beatport-tint bg-beatport-tint/10 font-semibold text-beatport-text'
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
