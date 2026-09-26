<script lang="ts">
	import { hotCuePads, cueMarkerLabel } from '$shared/utils/cues'
	import { onMount } from 'svelte'
	import {
		playerStore,
		isPlaying,
		playbackPosition,
		playbackDuration,
		standaloneTrack,
		currentTrack,
		playbackSource,
		recentStandaloneTracks,
		recentTracksStore,
		recentTracksLoading,
		albumsStore,
		selectedAlbum,
		libraryStore,
		appDataDir,
		activeView,
		uiStore,
		currentCues,
		currentWaveformBars,
	} from '$lib/stores'
	import type { StandaloneTrack, Cue } from '$shared/types'
	import * as libraryApi from '$shared/api/library'
	import { revealItemInDir } from '@tauri-apps/plugin-opener'
	import { toastStore } from '$shared/stores/toast'
	import {
		getArtworkUrl,
		formatDuration,
		formatBpm,
		formatKey,
		formatBitrate,
		getCamelotColor,
		getEnergyInfo,
		formatDate,
		getHarmonicKeys,
	} from '$shared/utils'
	import { Icon, Text, Tooltip } from '$lib/components/common'
	import AlbumGridView from './AlbumGridView.svelte'
	import AlbumDetailView from './AlbumDetailView.svelte'

	// =========================================================================
	// State & Derived Track
	// =========================================================================

	let isAlbumMode = $state(false)
	let isImporting = $state(false)
	let isHoveringWaveform = $state(false)
	let hoverWaveformPercent = $state(0)
	let isSeekingWaveform = $state(false)
	let waveformDragMs = $state<number | null>(null)
	let waveformContainer: HTMLDivElement | null = $state(null)
	let recentSearchQuery = $state('')

	const filteredRecentTracks = $derived.by(() => {
		const q = recentSearchQuery.trim().toLowerCase()
		if (!q) return $recentStandaloneTracks
		return $recentStandaloneTracks.filter((t) => {
			const title = (t.title || '').toLowerCase()
			const artist = (t.artist || '').toLowerCase()
			const album = (t.album || '').toLowerCase()
			const filename = t.file_path.split('/').pop()?.toLowerCase() || ''
			return title.includes(q) || artist.includes(q) || album.includes(q) || filename.includes(q)
		})
	})

	// Currently displayed track in Hero:
	// Prioritize currently active standalone track, then active library track, then latest recent track
	const activeHeroTrack = $derived.by<StandaloneTrack | null>(() => {
		// Priority 1: Current standalone playing track
		if ($playbackSource === 'standalone' && $standaloneTrack) {
			return $standaloneTrack
		}
		// Priority 2: Current library playing track converted to standalone
		if ($playbackSource === 'library' && $currentTrack) {
			return {
				id: $currentTrack.id,
				file_path: $currentTrack.file_path,
				title: $currentTrack.title,
				artist: $currentTrack.artist,
				album: $currentTrack.album,
				genre: $currentTrack.genre,
				year: $currentTrack.year,
				duration_ms: $currentTrack.duration_ms,
				bitrate: $currentTrack.bitrate,
				sample_rate: $currentTrack.sample_rate,
				format: $currentTrack.format || 'mp3',
				bpm: $currentTrack.bpm,
				key: $currentTrack.key,
				energy: $currentTrack.energy,
				artwork_path: $currentTrack.artwork_path,
				is_in_library: true,
				last_played_at: $currentTrack.last_played,
			}
		}
		// Priority 3: First track in recents list
		if ($recentStandaloneTracks.length > 0) {
			return $recentStandaloneTracks[0]
		}
		return null
	})

	const isCurrentPlayingInHero = $derived(
		$isPlaying &&
			Boolean(
				activeHeroTrack &&
				(($playbackSource === 'standalone' && $standaloneTrack?.file_path === activeHeroTrack.file_path) ||
					($playbackSource === 'library' && $currentTrack?.file_path === activeHeroTrack.file_path))
			)
	)

	const heroArtworkUrl = $derived(
		activeHeroTrack?.artwork_path ? getArtworkUrl(activeHeroTrack.artwork_path, $appDataDir) : null
	)

	const camelotInfo = $derived(activeHeroTrack?.key ? getCamelotColor(activeHeroTrack.key) : null)

	const energyInfo = $derived(activeHeroTrack?.energy ? getEnergyInfo(activeHeroTrack.energy) : null)

	const effectiveDuration = $derived($playbackDuration > 0 ? $playbackDuration : activeHeroTrack?.duration_ms || 0)

	const currentDisplayPosition = $derived(
		isSeekingWaveform && waveformDragMs !== null ? waveformDragMs : $playbackPosition
	)

	const progressPercent = $derived(
		effectiveDuration > 0 ? Math.min(100, Math.max(0, (currentDisplayPosition / effectiveDuration) * 100)) : 0
	)

	// Waveform bars (decoded audio waveform if available, or fallback to simulated 64-bar pattern)
	// Neutral flat line while the waveform loads or when the file cannot be decoded
	// (never a fake pattern that looks like real audio).
	const defaultWaveformBars = Array.from({ length: 64 }, () => 6)

	const activeWaveformBars = $derived(
		$currentWaveformBars && $currentWaveformBars.length > 0 ? $currentWaveformBars : defaultWaveformBars
	)

	// 8 Hot Cue pads (slot 0–7, label 1–8); memory cues never take a pad
	const cuePads = $derived(hotCuePads($currentCues).map((pad) => ({ slot: pad.label, cue: pad.cue })))

	async function handleCuePadClick(slot: number, cue: Cue | null) {
		if (cue) {
			await playerStore.jumpToCue(cue)
		}
	}

	async function handleHarmonicMatch() {
		if (!activeHeroTrack?.key) {
			toastStore.warning("Ce morceau n'a pas de tonalité Camelot définie")
			return
		}

		const compatibleKeys = getHarmonicKeys(activeHeroTrack.key)
		const currentBpm = activeHeroTrack.bpm

		let bpmMin: number | undefined
		let bpmMax: number | undefined

		if (currentBpm && currentBpm > 0) {
			bpmMin = Math.round(currentBpm * 0.96 * 10) / 10
			bpmMax = Math.round(currentBpm * 1.04 * 10) / 10
		}

		await libraryStore.loadTracks({
			keys: compatibleKeys,
			bpm_min: bpmMin,
			bpm_max: bpmMax,
		})

		uiStore.setActiveView('library')

		const bpmInfo = currentBpm ? ` • ${Math.round(currentBpm)} BPM (±4%)` : ''
		toastStore.success(
			`Mix Harmonique : ${activeHeroTrack.title || 'Piste'} (${formatKey(activeHeroTrack.key, 'camelot')}${bpmInfo}) — Clés : ${compatibleKeys.join(', ')}`
		)
	}

	// =========================================================================
	// Lifecycle
	// =========================================================================

	onMount(() => {
		if ($recentStandaloneTracks.length === 0) {
			recentTracksStore.load()
		}
		albumsStore.loadAlbums()
	})

	// =========================================================================
	// Handlers
	// =========================================================================

	async function handlePlayTrack(track: StandaloneTrack) {
		await playerStore.playStandalone(track, track.is_in_library)
	}

	function handleTogglePlayPause() {
		if (!activeHeroTrack) return
		if (
			($playbackSource === 'standalone' && $standaloneTrack?.file_path === activeHeroTrack.file_path) ||
			($playbackSource === 'library' && $currentTrack?.file_path === activeHeroTrack.file_path)
		) {
			playerStore.togglePlayPause()
		} else {
			playerStore.playStandalone(activeHeroTrack, activeHeroTrack.is_in_library)
		}
	}

	function handleStop() {
		playerStore.stop()
	}

	function handleSeek(ms: number) {
		playerStore.seek(ms)
	}

	function handleWaveformMouseDown(e: MouseEvent) {
		if (!effectiveDuration || !waveformContainer) return
		isSeekingWaveform = true
		const rect = waveformContainer.getBoundingClientRect()
		const x = Math.max(0, Math.min(e.clientX - rect.left, rect.width))
		waveformDragMs = Math.floor((x / rect.width) * effectiveDuration)

		const handleMouseMove = (moveEvent: MouseEvent) => {
			if (isSeekingWaveform && waveformContainer && effectiveDuration) {
				const r = waveformContainer.getBoundingClientRect()
				const mx = Math.max(0, Math.min(moveEvent.clientX - r.left, r.width))
				waveformDragMs = Math.floor((mx / r.width) * effectiveDuration)
			}
		}

		const handleMouseUp = () => {
			if (isSeekingWaveform && waveformDragMs !== null) {
				playerStore.seek(waveformDragMs)
			}
			isSeekingWaveform = false
			waveformDragMs = null
			window.removeEventListener('mousemove', handleMouseMove)
			window.removeEventListener('mouseup', handleMouseUp)
		}

		window.addEventListener('mousemove', handleMouseMove)
		window.addEventListener('mouseup', handleMouseUp)
	}

	function handleWaveformMouseMove(e: MouseEvent) {
		if (!waveformContainer) return
		const rect = waveformContainer.getBoundingClientRect()
		const x = Math.max(0, Math.min(e.clientX - rect.left, rect.width))
		hoverWaveformPercent = Math.max(0, Math.min(1, x / rect.width))
		isHoveringWaveform = true
	}

	function handleWaveformMouseLeave() {
		isHoveringWaveform = false
	}

	async function handleImportToLibrary() {
		if (!activeHeroTrack || activeHeroTrack.is_in_library || isImporting) return
		isImporting = true
		try {
			const res = await libraryApi.importTracks([activeHeroTrack.file_path])
			if (res.tracks.length > 0) {
				await libraryStore.reloadWithCurrentFilter()
				const imported = activeHeroTrack
				await recentTracksStore.removeTrack(imported.id)
				// Update the source store: the hero track is a $derived value and must not be mutated
				playerStore.markStandaloneInLibrary(imported.file_path)
				toastStore.success(`"${imported.title || 'Morceau'}" ajouté à la bibliothèque Crate`)
			} else if (res.errors.length > 0) {
				toastStore.error(`Échec de l'import: ${res.errors[0]}`)
			}
		} catch (err) {
			console.error('Import standalone track error:', err)
			toastStore.error("Erreur lors de l'ajout à la bibliothèque Crate")
		} finally {
			isImporting = false
		}
	}

	async function handleRevealInFinder(filePath: string) {
		try {
			await revealItemInDir(filePath)
		} catch (err) {
			console.error('Reveal in finder error:', err)
			toastStore.error('Impossible de révéler le fichier dans le Finder')
		}
	}

	async function handleRemoveRecent(e: MouseEvent, id: string) {
		e.stopPropagation()
		await recentTracksStore.removeTrack(id)
	}

	async function handleClearRecent() {
		await recentTracksStore.clear()
		toastStore.info('Historique des fichiers récents effacé')
	}

	function formatSampleRateDisplay(sr: number | null | undefined): string {
		if (!sr) return '-'
		const khz = (sr / 1000).toFixed(sr % 1000 === 0 ? 0 : 1)
		return `${khz} kHz`
	}
</script>

<div class="relative flex h-full w-full flex-col overflow-hidden bg-surface-0/80 text-text-primary select-none">
	<!-- ===================================================================== -->
	<!-- AMBIENT GLOW FLUID BACKGROUND                                         -->
	<!-- ===================================================================== -->
	{#if heroArtworkUrl}
		<div
			class="pointer-events-none absolute -top-20 -right-20 -left-20 z-0 h-[450px] overflow-hidden opacity-15 blur-[100px] filter transition-all duration-700 select-none dark:opacity-30"
		>
			<img src={heroArtworkUrl} alt="" class="h-full w-full scale-150 transform-gpu object-cover" />
		</div>
	{/if}

	<!-- ===================================================================== -->
	<!-- 1. PLAYER HERO (STYLE APPLE MUSIC MACOS)                              -->
	<!-- ===================================================================== -->
	<div
		class="relative z-20 w-full flex-shrink-0 border-b border-stroke-subtle bg-surface-1/60 p-6 shadow-xl backdrop-blur-xl md:p-8 dark:shadow-2xl"
	>
		<div class="mx-auto w-full max-w-7xl">
			{#if activeHeroTrack}
				<div class="flex w-full items-center gap-6 md:gap-8">
					<!-- 225x225px Artwork Cover -->
					<div
						class="group relative h-[225px] w-[225px] flex-shrink-0 overflow-hidden rounded-2xl border border-stroke-subtle bg-surface-2/60 shadow-2xl shadow-black/20 dark:shadow-black/60"
					>
						{#if heroArtworkUrl}
							<img
								src={heroArtworkUrl}
								alt="Artwork"
								class="h-full w-full object-cover transition-transform duration-500 group-hover:scale-105"
							/>
						{:else}
							<div class="flex h-full w-full items-center justify-center bg-surface-2/80 text-text-tertiary">
								<Icon name="music-note" class="h-16 w-16 opacity-30" />
							</div>
						{/if}

						<!-- Circular Play/Pause overlay button on hover -->
						<button
							type="button"
							class="absolute inset-0 flex cursor-pointer items-center justify-center bg-black/30 opacity-0 backdrop-blur-xs transition-opacity duration-200 group-hover:opacity-100"
							onclick={handleTogglePlayPause}
							aria-label="Play / Pause"
						>
							<div
								class="flex h-14 w-14 items-center justify-center rounded-full bg-cyan-400 text-black shadow-lg shadow-cyan-400/50 transition-transform group-hover:scale-105 active:scale-95"
							>
								<Icon name={isCurrentPlayingInHero ? 'pause' : 'play'} class="ml-0.5 h-6 w-6" fill />
							</div>
						</button>
					</div>

					<!-- Metadata, Waveform & Controls Column -->
					<div class="flex h-[225px] min-w-0 flex-1 flex-col justify-between">
						<!-- Top Part: Micro-pills Badges (Left) & Controls/Mode Toggle (Right) -->
						<div class="flex w-full items-center justify-between gap-4">
							<!-- Badges micro-pills horizontaux -->
							<div class="flex flex-wrap items-center gap-2">
								<!-- Source -->
								{#if activeHeroTrack.is_in_library}
									<span
										class="bg-brand-primary/15 border-brand-primary/30 inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 text-[10px] font-semibold text-brand-primary"
									>
										<Icon name="library" class="h-2.5 w-2.5" />
										Bibliothèque Crate
									</span>
								{:else}
									<span
										class="inline-flex items-center gap-1 rounded-full border border-amber-500/30 bg-amber-500/15 px-2.5 py-0.5 text-[10px] font-semibold text-amber-600 dark:text-amber-300"
									>
										<Icon name="hard-drive" class="h-2.5 w-2.5" />
										Fichier externe
									</span>
								{/if}

								<!-- Format -->
								<span
									class="rounded-full border border-stroke-subtle bg-surface-2 px-2.5 py-0.5 font-mono text-[10px] font-medium tracking-wider text-text-secondary uppercase"
								>
									{activeHeroTrack.format || 'AUDIO'}
								</span>

								<!-- Bitrate -->
								<span
									class="rounded-full border border-stroke-subtle bg-surface-2 px-2.5 py-0.5 font-mono text-[10px] font-medium text-text-tertiary"
								>
									{formatBitrate(activeHeroTrack.bitrate, activeHeroTrack.format, activeHeroTrack.sample_rate)}
								</span>

								<!-- kHz -->
								{#if activeHeroTrack.sample_rate}
									<span
										class="rounded-full border border-stroke-subtle bg-surface-2 px-2.5 py-0.5 font-mono text-[10px] font-medium text-text-tertiary"
									>
										{formatSampleRateDisplay(activeHeroTrack.sample_rate)}
									</span>
								{/if}

								<!-- BPM -->
								{#if activeHeroTrack.bpm}
									<span
										class="rounded-full border border-cyan-500/20 bg-cyan-500/10 px-2.5 py-0.5 font-mono text-[10px] font-bold text-cyan-600 dark:text-cyan-300"
									>
										{formatBpm(activeHeroTrack.bpm)} BPM
									</span>
								{/if}

								<!-- Camelot Key & Mix Harmonique (1 Clic) -->
								{#if activeHeroTrack.key}
									<div class="inline-flex items-center gap-1.5">
										{#if camelotInfo}
											<span
												class="inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 text-[10px] font-extrabold shadow-xs select-none"
												style="background-color: {camelotInfo.bg}; color: {camelotInfo.text}; border-color: {camelotInfo.border};"
												title="Camelot Key: {camelotInfo.name}"
											>
												{formatKey(activeHeroTrack.key, 'camelot')}
											</span>
										{:else}
											<span
												class="rounded-full border border-stroke-subtle bg-surface-2 px-2.5 py-0.5 font-mono text-[10px] font-bold text-text-primary"
											>
												{activeHeroTrack.key}
											</span>
										{/if}

										<button
											type="button"
											class="inline-flex cursor-pointer items-center gap-1 rounded-full border border-purple-500/30 bg-purple-500/15 px-2 py-0.5 text-[10px] font-bold text-purple-600 shadow-xs transition-all hover:scale-105 hover:bg-purple-500/25 active:scale-95 dark:text-purple-300"
											onclick={handleHarmonicMatch}
											title="Trouver les morceaux compatibles (Même clé, relative, ±1h, ±4% BPM)"
										>
											<Icon name="sparkles" class="h-3 w-3 text-purple-500 dark:text-purple-300" />
											<span>Mix Harmonique</span>
										</button>
									</div>
								{/if}

								<!-- Energy ⚡ -->
								{#if energyInfo}
									<span
										class="inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 text-[10px] font-bold shadow-xs"
										style="background-color: {energyInfo.bg}; color: {energyInfo.color}; border-color: {energyInfo.border};"
										title="Énergie: {energyInfo.descriptor} ({activeHeroTrack.energy}/10)"
									>
										<span>⚡</span>
										<span>{activeHeroTrack.energy}</span>
									</span>
								{/if}
							</div>

							<!-- Action Buttons & Mode Switch -->
							<div class="flex flex-shrink-0 items-center gap-3">
								<!-- Mode Toggle: [📋 Récents | 💿 Albums] -->
								<div
									class="inline-flex items-center rounded-full border border-stroke-subtle bg-surface-2/90 p-0.5 shadow-inner"
								>
									<button
										type="button"
										class="flex cursor-pointer items-center gap-1.5 rounded-full px-3 py-1 text-xs font-semibold transition-all {!isAlbumMode
											? 'bg-cyan-500 font-bold text-black shadow-xs'
											: 'text-text-secondary hover:text-text-primary'}"
										onclick={() => (isAlbumMode = false)}
									>
										<Icon name="disc" class="h-3 w-3" />
										<span>Récents</span>
									</button>
									<button
										type="button"
										class="flex cursor-pointer items-center gap-1.5 rounded-full px-3 py-1 text-xs font-semibold transition-all {isAlbumMode
											? 'bg-cyan-500 font-bold text-black shadow-xs'
											: 'text-text-secondary hover:text-text-primary'}"
										onclick={() => (isAlbumMode = true)}
									>
										<Icon name="folder-open" class="h-3 w-3" />
										<span>Mode Album</span>
									</button>
								</div>

								{#if !activeHeroTrack.is_in_library}
									<button
										type="button"
										class="shadow-brand-primary/20 flex cursor-pointer items-center gap-1.5 rounded-full bg-brand-primary px-3 py-1.5 text-xs font-semibold text-black shadow-md transition-all hover:brightness-110 active:scale-95 disabled:opacity-50"
										onclick={handleImportToLibrary}
										disabled={isImporting}
									>
										<Icon
											name={isImporting ? 'loader' : 'plus'}
											class="h-3.5 w-3.5 {isImporting ? 'animate-spin' : ''}"
										/>
										<span>{isImporting ? 'Ajout...' : 'Ajouter à ma bibliothèque'}</span>
									</button>
								{/if}

								<Tooltip text="Révéler dans le Finder" position="bottom">
									<button
										type="button"
										class="hover:bg-surface-3 flex h-8 w-8 cursor-pointer items-center justify-center rounded-full border border-stroke-subtle bg-surface-2 text-text-secondary transition-all hover:text-text-primary active:scale-95"
										onclick={() => handleRevealInFinder(activeHeroTrack.file_path)}
										aria-label="Révéler dans le Finder"
									>
										<Icon name="folder-open" class="h-4 w-4 text-cyan-600 dark:text-cyan-400" />
									</button>
								</Tooltip>
							</div>
						</div>

						<!-- Middle Part: Title & Artist directly above waveform -->
						<div class="min-w-0">
							<h1
								class="line-clamp-1 text-2xl font-bold tracking-tight text-text-primary"
								title={activeHeroTrack.title || activeHeroTrack.file_path}
							>
								{activeHeroTrack.title || activeHeroTrack.file_path.split('/').pop() || 'Morceau Inconnu'}
							</h1>
							<p class="mt-0.5 line-clamp-1 text-base font-medium text-text-secondary">
								{activeHeroTrack.artist || 'Artiste inconnu'}
								{#if activeHeroTrack.album}
									<span class="font-normal text-text-tertiary"> — {activeHeroTrack.album}</span>
								{/if}
							</p>
						</div>

						<!-- Bottom Part: Waveform (48px / h-12) & Integrated Controls -->
						<div class="flex flex-col gap-3">
							<!-- Waveform Row with Time Markers -->
							<div class="flex items-center gap-3">
								<span class="w-12 text-right font-mono text-xs font-bold text-cyan-600 tabular-nums dark:text-cyan-400">
									{formatDuration(currentDisplayPosition)}
								</span>

								<!-- Waveform Seekbar (h-12 / 48px) -->
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<div
									bind:this={waveformContainer}
									class="group relative h-12 flex-1 cursor-pointer overflow-hidden rounded-xl border border-stroke-subtle bg-surface-2/60 px-2 py-1 transition-all"
									onmousedown={handleWaveformMouseDown}
									onmousemove={handleWaveformMouseMove}
									onmouseleave={handleWaveformMouseLeave}
								>
									<!-- Waveform Bars Display -->
									<div class="flex h-full w-full items-center justify-between gap-[2px]">
										{#each activeWaveformBars as barHeight, i (i)}
											{@const barPercent = (i / activeWaveformBars.length) * 100}
											{@const isPlayed = barPercent <= progressPercent}
											<div
												class="flex-1 rounded-full {isPlayed
													? 'bg-cyan-400 shadow-[0_0_6px_rgba(6,182,212,0.7)]'
													: 'bg-text-tertiary/20 group-hover:bg-text-tertiary/35'}"
												style="height: {barHeight}%;"
											></div>
										{/each}
									</div>

									<!-- Hot Cue Visual Pins on Waveform -->
									{#if effectiveDuration > 0}
										{#each $currentCues as cue (cue.id)}
											{@const cuePercent = Math.min(100, Math.max(0, (cue.position_ms / effectiveDuration) * 100))}
											{@const cueNum = cueMarkerLabel(cue)}
											<button
												type="button"
												class="group/pin absolute top-0 bottom-0 z-10 flex w-4 -translate-x-1/2 cursor-pointer flex-col items-center justify-between transition-all hover:scale-110"
												style="left: {cuePercent}%"
												onclick={(e) => {
													e.stopPropagation()
													playerStore.jumpToCue(cue)
												}}
												title="{cue.cue_type === 'hot' ? `Hot Cue ${cueNum}` : 'Memory Cue'} : {cue.name ||
													formatDuration(cue.position_ms)}"
											>
												<div
													class="flex h-3.5 w-3.5 items-center justify-center rounded-xs border border-amber-200 bg-amber-400 text-[8px] font-black text-black shadow-[0_0_8px_rgba(251,191,36,0.9)]"
												>
													{cueNum}
												</div>
												<div
													class="w-[1.5px] flex-1 bg-amber-400/80 shadow-[0_0_4px_rgba(251,191,36,0.6)] group-hover/pin:bg-amber-300"
												></div>
											</button>
										{/each}
									{/if}

									<!-- Neon Progress Indicator Line -->
									<div
										class="pointer-events-none absolute inset-y-0 w-0.5 bg-white shadow-[0_0_8px_white]"
										style="left: {progressPercent}%"
									></div>

									<!-- Hover Time Tooltip -->
									{#if isHoveringWaveform && effectiveDuration > 0}
										<div
											class="pointer-events-none absolute top-1 -translate-x-1/2 rounded-md border border-cyan-500/40 bg-surface-1 px-2 py-0.5 font-mono text-[10px] font-bold text-cyan-400 shadow-xl"
											style="left: {hoverWaveformPercent * 100}%"
										>
											{formatDuration(hoverWaveformPercent * effectiveDuration)}
										</div>
									{/if}
								</div>

								<span class="w-12 font-mono text-xs font-semibold text-text-tertiary tabular-nums">
									{formatDuration(effectiveDuration)}
								</span>
							</div>

							<!-- 8 Hot Cue Pads Row -->
							<div class="grid grid-cols-8 gap-2 rounded-xl border border-stroke-subtle bg-surface-1/40 px-1 py-1">
								{#each cuePads as pad (pad.slot)}
									<button
										type="button"
										class="flex flex-col items-center justify-center rounded-lg border px-1 py-1.5 transition-all select-none {pad.cue
											? 'cursor-pointer border-amber-500/40 bg-amber-500/15 text-amber-500 shadow-xs hover:border-amber-400 hover:bg-amber-500/25 active:scale-95 dark:text-amber-400'
											: 'cursor-default border-white/5 bg-surface-2/40 text-text-tertiary/40 hover:bg-surface-2/60'}"
										onclick={() => handleCuePadClick(pad.slot, pad.cue)}
										disabled={!pad.cue}
										title={pad.cue
											? `Sauter au Hot Cue ${pad.slot} (${pad.cue.name || formatDuration(pad.cue.position_ms)})`
											: `Cue ${pad.slot} non défini`}
									>
										<span class="font-mono text-[11px] leading-none font-extrabold">CUE {pad.slot}</span>
										<span class="mt-0.5 font-mono text-[9px] tabular-nums opacity-90">
											{pad.cue ? formatDuration(pad.cue.position_ms) : '--:--'}
										</span>
									</button>
								{/each}
							</div>

							<!-- Controls: Centered Transport Controls -->
							<div class="flex items-center justify-center gap-3">
								<button
									type="button"
									class="hover:bg-surface-3 flex h-10 w-10 cursor-pointer items-center justify-center rounded-full border border-stroke-subtle bg-surface-2 text-text-secondary shadow-xs transition-all hover:text-text-primary active:scale-95"
									onclick={() => handleSeek(0)}
									title="Revenir au début"
								>
									<Icon name="skip-back" class="h-4 w-4" fill />
								</button>

								<button
									type="button"
									class="flex h-[52px] w-[52px] cursor-pointer items-center justify-center rounded-full bg-cyan-400 text-black shadow-lg shadow-cyan-400/40 transition-all hover:brightness-110 active:scale-95"
									onclick={handleTogglePlayPause}
									title={isCurrentPlayingInHero ? 'Pause' : 'Lecture'}
								>
									<Icon name={isCurrentPlayingInHero ? 'pause' : 'play'} class="ml-0.5 h-7 w-7" fill />
								</button>

								<button
									type="button"
									class="hover:bg-surface-3 flex h-10 w-10 cursor-pointer items-center justify-center rounded-full border border-stroke-subtle bg-surface-2 text-text-secondary shadow-xs transition-all hover:text-text-primary active:scale-95"
									onclick={handleStop}
									title="Arrêter la lecture"
								>
									<Icon name="stop" class="h-4 w-4" fill />
								</button>
							</div>
						</div>
					</div>
				</div>
			{:else}
				<!-- Empty Hero State -->
				<div class="flex items-center justify-between px-6 py-12">
					<div class="flex items-center gap-6">
						<div
							class="flex h-20 w-20 items-center justify-center rounded-2xl border border-stroke-subtle bg-surface-2 text-cyan-600 shadow-inner dark:text-cyan-400"
						>
							<Icon name="disc" class="h-10 w-10 opacity-80" />
						</div>
						<div>
							<h2 class="text-xl font-bold text-text-primary">Lecteur Audio Crate</h2>
							<p class="mt-1 text-sm text-text-secondary">
								Ouvrez n'importe quel fichier audio depuis le Finder pour l'écouter instantanément.
							</p>
						</div>
					</div>

					<!-- Mode Toggle even in empty hero -->
					<div
						class="inline-flex items-center rounded-full border border-stroke-subtle bg-surface-2 p-0.5 shadow-inner"
					>
						<button
							type="button"
							class="flex cursor-pointer items-center gap-1.5 rounded-full px-3 py-1 text-xs font-semibold transition-all {!isAlbumMode
								? 'bg-cyan-500 font-bold text-black shadow-xs'
								: 'text-text-secondary hover:text-text-primary'}"
							onclick={() => (isAlbumMode = false)}
						>
							<Icon name="disc" class="h-3 w-3" />
							<span>Récents</span>
						</button>
						<button
							type="button"
							class="flex cursor-pointer items-center gap-1.5 rounded-full px-3 py-1 text-xs font-semibold transition-all {isAlbumMode
								? 'bg-cyan-500 font-bold text-black shadow-xs'
								: 'text-text-secondary hover:text-text-primary'}"
							onclick={() => (isAlbumMode = true)}
						>
							<Icon name="folder-open" class="h-3 w-3" />
							<span>Mode Album</span>
						</button>
					</div>
				</div>
			{/if}
		</div>
	</div>

	<!-- ===================================================================== -->
	<!-- 2. BOTTOM SECTION: [FICHIERS RÉCENTS] OR [MODE ALBUM]                 -->
	<!-- ===================================================================== -->
	{#if !isAlbumMode}
		<!-- EN-TÊTE DES RÉCENTS FIXE PLEINE LARGEUR (NE DOIT PAS SCROLLER) -->
		<div
			class="z-20 w-full flex-shrink-0 border-y border-stroke-subtle bg-surface-0/90 px-6 py-2 shadow-xs backdrop-blur-md"
		>
			<!-- Ligne 1 : Titre + Barre de recherche instantanée + Bouton Effacer l'historique -->
			<div class="flex items-center justify-between gap-4 pb-2">
				<div class="flex min-w-0 flex-1 items-center gap-4">
					<div class="flex flex-shrink-0 items-center gap-2.5">
						<Icon name="disc" class="h-4 w-4 text-cyan-600 dark:text-cyan-400" />
						<h2 class="text-sm font-bold text-text-primary">Fichiers Récents</h2>
					</div>

					<!-- Barre de recherche instantanée -->
					<div class="relative flex w-full max-w-sm items-center">
						<Icon name="search" class="pointer-events-none absolute left-2.5 h-3.5 w-3.5 text-text-tertiary" />
						<input
							type="text"
							placeholder="Filtrer les récents (titre, artiste, format, clé)..."
							bind:value={recentSearchQuery}
							class="h-7 w-full rounded-full border border-stroke-subtle bg-surface-2/80 pr-7 pl-8 text-xs text-text-primary transition-all placeholder:text-text-tertiary focus:border-cyan-500 focus:bg-surface-2 focus:outline-hidden"
						/>
						{#if recentSearchQuery}
							<button
								type="button"
								class="absolute right-2 flex h-4 w-4 cursor-pointer items-center justify-center rounded-full text-text-tertiary hover:text-text-primary"
								onclick={() => (recentSearchQuery = '')}
								aria-label="Effacer la recherche"
							>
								<Icon name="x" class="h-3 w-3" />
							</button>
						{/if}
					</div>
				</div>

				{#if $recentStandaloneTracks.length > 0}
					<button
						type="button"
						class="flex flex-shrink-0 cursor-pointer items-center gap-1.5 rounded-full border border-stroke-subtle bg-surface-2 px-2.5 py-0.5 text-[11px] font-medium text-text-tertiary transition-all hover:border-red-500/30 hover:bg-red-500/10 hover:text-red-500 active:scale-95 dark:hover:text-red-400"
						onclick={handleClearRecent}
					>
						<Icon name="trash" class="h-3 w-3" />
						<span>Effacer l'historique</span>
					</button>
				{/if}
			</div>

			<!-- Ligne 2 : En-têtes de colonnes fixes -->
			<div
				class="grid grid-cols-[32px_36px_minmax(180px,2fr)_minmax(120px,1.2fr)_110px_110px_64px_130px_60px] items-center gap-3 border-t border-stroke-subtle px-3 py-1 text-[10px] font-semibold tracking-wider text-text-tertiary uppercase"
			>
				<span class="pl-1">#</span>
				<span>Cover</span>
				<span>Titre & Artiste</span>
				<span>Album</span>
				<span>Format</span>
				<span>BPM / Clé</span>
				<span class="text-right">Durée</span>
				<span>Dernière lecture</span>
				<span class="pr-1 text-right">Actions</span>
			</div>
		</div>

		<!-- LISTE DÉFILANTE DES SONS UNIQUEMENT (flex-1 overflow-y-auto) -->
		<div class="flex-1 overflow-y-auto scroll-smooth px-6 py-1">
			{#if $recentTracksLoading}
				<div class="flex h-32 items-center justify-center gap-2 text-text-tertiary">
					<Icon name="loader" class="h-4 w-4 animate-spin" />
					<span class="text-xs">Chargement de l'historique...</span>
				</div>
			{:else if $recentStandaloneTracks.length === 0}
				<!-- Empty State -->
				<div class="flex flex-col items-center justify-center py-12 text-center">
					<div
						class="mb-2.5 flex h-12 w-12 items-center justify-center rounded-2xl border border-stroke-subtle bg-surface-2 text-cyan-600 shadow-inner dark:text-cyan-400/60"
					>
						<Icon name="disc" class="h-6 w-6" />
					</div>
					<h3 class="text-sm font-semibold text-text-primary">Aucun fichier audio récent</h3>
					<p class="mt-1 max-w-sm text-xs text-text-tertiary">
						Double-cliquez sur un fichier audio dans le Finder ou ouvrez-le avec Crate pour l'écouter directement sans
						encombrer votre bibliothèque.
					</p>
				</div>
			{:else if filteredRecentTracks.length === 0}
				<!-- Empty Filter State -->
				<div class="flex flex-col items-center justify-center py-12 text-center">
					<div
						class="mb-2 flex h-10 w-10 items-center justify-center rounded-full border border-stroke-subtle bg-surface-2 text-text-tertiary"
					>
						<Icon name="search" class="h-5 w-5" />
					</div>
					<h3 class="text-xs font-semibold text-text-primary">Aucun résultat pour « {recentSearchQuery} »</h3>
					<button
						type="button"
						class="mt-2 cursor-pointer text-xs text-cyan-600 hover:underline dark:text-cyan-400"
						onclick={() => (recentSearchQuery = '')}
					>
						Effacer le filtre
					</button>
				</div>
			{:else}
				<div class="flex flex-col divide-y divide-stroke-subtle/50 py-1">
					{#each filteredRecentTracks as track, index (track.id)}
						{@const isPlayingThis =
							$isPlaying && $playbackSource === 'standalone' && $standaloneTrack?.file_path === track.file_path}
						{@const trackArtUrl = track.artwork_path ? getArtworkUrl(track.artwork_path, $appDataDir) : null}
						{@const trackCamelot = track.key ? getCamelotColor(track.key) : null}

						<!-- Track Row -->
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<div
							class="group grid cursor-pointer grid-cols-[32px_36px_minmax(180px,2fr)_minmax(120px,1.2fr)_110px_110px_64px_130px_60px] items-center gap-3 rounded-lg px-3 py-1.5 text-xs transition-colors {isPlayingThis
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
									<span class="group-hover:hidden">{index + 1}</span>
									<Icon
										name="play"
										class="hidden h-3.5 w-3.5 text-cyan-600 group-hover:block dark:text-cyan-400"
										fill
									/>
								{/if}
							</div>

							<!-- Cover Thumbnail -->
							<div class="h-8 w-8 flex-shrink-0 overflow-hidden rounded-lg border border-stroke-subtle bg-surface-2/60">
								{#if trackArtUrl}
									<img src={trackArtUrl} alt="" class="h-full w-full object-cover" />
								{:else}
									<div class="flex h-full w-full items-center justify-center text-text-tertiary opacity-40">
										<Icon name="music-note" class="h-3.5 w-3.5" />
									</div>
								{/if}
							</div>

							<!-- Title & Artist -->
							<div class="min-w-0 pr-2">
								<div
									class="truncate font-semibold text-text-primary transition-colors group-hover:text-cyan-600 dark:group-hover:text-cyan-400"
								>
									{track.title || track.file_path.split('/').pop() || 'Morceau inconnu'}
								</div>
								<div class="truncate text-[11px] text-text-secondary">
									{track.artist || 'Artiste inconnu'}
								</div>
							</div>

							<!-- Album -->
							<div class="truncate pr-2 text-[11px] text-text-secondary">
								{track.album || '-'}
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
							<div class="text-right font-mono text-[11px] text-text-secondary tabular-nums">
								{formatDuration(track.duration_ms)}
							</div>

							<!-- Last Played -->
							<div class="truncate text-[11px] text-text-tertiary">
								{track.last_played_at ? formatDate(track.last_played_at, 'locale') : '-'}
							</div>

							<!-- Actions -->
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<!-- svelte-ignore a11y_click_events_have_key_events -->
							<div class="pr-1 text-right" onclick={(e) => e.stopPropagation()}>
								<div class="flex items-center justify-end gap-1 opacity-0 transition-opacity group-hover:opacity-100">
									<Tooltip text="Révéler dans le Finder" position="top">
										<button
											type="button"
											class="cursor-pointer rounded-full p-1 text-text-tertiary transition-colors hover:bg-surface-2 hover:text-text-primary"
											onclick={() => handleRevealInFinder(track.file_path)}
										>
											<Icon name="folder-open" class="h-3 w-3" />
										</button>
									</Tooltip>

									<Tooltip text="Supprimer de l'historique" position="top">
										<button
											type="button"
											class="cursor-pointer rounded-full p-1 text-text-tertiary transition-colors hover:bg-red-500/10 hover:text-red-500 dark:hover:text-red-400"
											onclick={(e) => handleRemoveRecent(e, track.id)}
										>
											<Icon name="x" class="h-3 w-3" />
										</button>
									</Tooltip>
								</div>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{:else if !$selectedAlbum}
		<!-- ALBUM GRID VIEW -->
		<AlbumGridView onSelectAlbum={(album) => albumsStore.selectAlbum(album)} />
	{:else}
		<!-- ALBUM DETAIL VIEW -->
		<AlbumDetailView album={$selectedAlbum} onBack={() => albumsStore.selectAlbum(null)} />
	{/if}
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
