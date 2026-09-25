<script lang="ts">
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

	const camelotInfo = $derived(
		activeHeroTrack?.key ? getCamelotColor(activeHeroTrack.key) : null
	)

	const energyInfo = $derived(
		activeHeroTrack?.energy ? getEnergyInfo(activeHeroTrack.energy) : null
	)

	const effectiveDuration = $derived(
		$playbackDuration > 0
			? $playbackDuration
			: activeHeroTrack?.duration_ms || 0
	)

	const currentDisplayPosition = $derived(
		isSeekingWaveform && waveformDragMs !== null ? waveformDragMs : $playbackPosition
	)

	const progressPercent = $derived(
		effectiveDuration > 0 ? Math.min(100, Math.max(0, (currentDisplayPosition / effectiveDuration) * 100)) : 0
	)

	// Waveform bars (decoded audio waveform if available, or fallback to simulated 64-bar pattern)
	const defaultWaveformBars = [
		20, 35, 50, 45, 60, 80, 75, 90, 85, 100, 95, 70, 60, 75, 85, 90,
		65, 80, 95, 100, 85, 70, 60, 75, 85, 90, 95, 100, 80, 65, 50, 70,
		85, 90, 100, 95, 80, 75, 90, 85, 70, 60, 75, 85, 90, 80, 65, 70,
		85, 95, 100, 90, 80, 70, 60, 50, 65, 80, 75, 60, 45, 35, 25, 15
	]

	const activeWaveformBars = $derived(
		$currentWaveformBars && $currentWaveformBars.length > 0 ? $currentWaveformBars : defaultWaveformBars
	)

	// 8 Hot Cue slots
	const cuePads = $derived.by(() => {
		return Array.from({ length: 8 }, (_, idx) => {
			const cueNum = idx + 1
			const found = $currentCues.find(
				(c: Cue) => c.hot_cue_index === cueNum || c.hot_cue_index === idx
			) || $currentCues[idx]
			return {
				slot: cueNum,
				cue: found || null,
			}
		})
	})

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
				await libraryStore.loadTracks()
				await recentTracksStore.removeTrack(activeHeroTrack.id)
				activeHeroTrack.is_in_library = true
				toastStore.success(`"${activeHeroTrack.title || 'Morceau'}" ajouté à la bibliothèque Crate`)
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

<div class="relative flex flex-col h-full w-full overflow-hidden bg-surface-0/80 text-text-primary select-none">
	<!-- ===================================================================== -->
	<!-- AMBIENT GLOW FLUID BACKGROUND                                         -->
	<!-- ===================================================================== -->
	{#if heroArtworkUrl}
		<div class="pointer-events-none absolute -top-20 -left-20 -right-20 h-[450px] opacity-15 dark:opacity-30 filter blur-[100px] overflow-hidden select-none transition-all duration-700 z-0">
			<img
				src={heroArtworkUrl}
				alt=""
				class="h-full w-full object-cover scale-150 transform-gpu"
			/>
		</div>
	{/if}

	<!-- ===================================================================== -->
	<!-- 1. PLAYER HERO (STYLE APPLE MUSIC MACOS)                              -->
	<!-- ===================================================================== -->
	<div class="relative z-20 flex-shrink-0 w-full border-b border-stroke-subtle bg-surface-1/60 backdrop-blur-xl p-6 md:p-8 shadow-xl dark:shadow-2xl">
		<div class="max-w-7xl mx-auto w-full">
			{#if activeHeroTrack}
				<div class="flex items-center gap-6 md:gap-8 w-full">
					<!-- 225x225px Artwork Cover -->
					<div class="relative h-[225px] w-[225px] flex-shrink-0 overflow-hidden rounded-2xl border border-stroke-subtle bg-surface-2/60 shadow-2xl shadow-black/20 dark:shadow-black/60 group">
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
							class="absolute inset-0 flex items-center justify-center bg-black/30 backdrop-blur-xs opacity-0 transition-opacity duration-200 group-hover:opacity-100 cursor-pointer"
							onclick={handleTogglePlayPause}
							aria-label="Play / Pause"
						>
							<div class="flex h-14 w-14 items-center justify-center rounded-full bg-cyan-400 text-black shadow-lg shadow-cyan-400/50 transition-transform active:scale-95 group-hover:scale-105">
								<Icon name={isCurrentPlayingInHero ? 'pause' : 'play'} class="h-6 w-6 ml-0.5" fill />
							</div>
						</button>
					</div>

					<!-- Metadata, Waveform & Controls Column -->
					<div class="flex flex-1 flex-col justify-between min-w-0 h-[225px]">
						<!-- Top Part: Micro-pills Badges (Left) & Controls/Mode Toggle (Right) -->
						<div class="flex items-center justify-between gap-4 w-full">
							<!-- Badges micro-pills horizontaux -->
							<div class="flex flex-wrap items-center gap-2">
								<!-- Source -->
								{#if activeHeroTrack.is_in_library}
									<span class="inline-flex items-center gap-1 rounded-full bg-brand-primary/15 border border-brand-primary/30 px-2.5 py-0.5 text-[10px] font-semibold text-brand-primary">
										<Icon name="library" class="h-2.5 w-2.5" />
										Bibliothèque Crate
									</span>
								{:else}
									<span class="inline-flex items-center gap-1 rounded-full bg-amber-500/15 border border-amber-500/30 px-2.5 py-0.5 text-[10px] font-semibold text-amber-600 dark:text-amber-300">
										<Icon name="hard-drive" class="h-2.5 w-2.5" />
										Fichier externe
									</span>
								{/if}

								<!-- Format -->
								<span class="rounded-full bg-surface-2 border border-stroke-subtle px-2.5 py-0.5 text-[10px] font-mono font-medium uppercase tracking-wider text-text-secondary">
									{activeHeroTrack.format || 'AUDIO'}
								</span>

								<!-- Bitrate -->
								<span class="rounded-full bg-surface-2 border border-stroke-subtle px-2.5 py-0.5 text-[10px] font-mono font-medium text-text-tertiary">
									{formatBitrate(activeHeroTrack.bitrate, activeHeroTrack.format, activeHeroTrack.sample_rate)}
								</span>

								<!-- kHz -->
								{#if activeHeroTrack.sample_rate}
									<span class="rounded-full bg-surface-2 border border-stroke-subtle px-2.5 py-0.5 text-[10px] font-mono font-medium text-text-tertiary">
										{formatSampleRateDisplay(activeHeroTrack.sample_rate)}
									</span>
								{/if}

								<!-- BPM -->
								{#if activeHeroTrack.bpm}
									<span class="rounded-full bg-cyan-500/10 border border-cyan-500/20 px-2.5 py-0.5 text-[10px] font-mono font-bold text-cyan-600 dark:text-cyan-300">
										{formatBpm(activeHeroTrack.bpm)} BPM
									</span>
								{/if}

								<!-- Camelot Key & Mix Harmonique (1 Clic) -->
								{#if activeHeroTrack.key}
									<div class="inline-flex items-center gap-1.5">
										{#if camelotInfo}
											<span
												class="inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-[10px] font-extrabold select-none border shadow-xs"
												style="background-color: {camelotInfo.bg}; color: {camelotInfo.text}; border-color: {camelotInfo.border};"
												title="Camelot Key: {camelotInfo.name}"
											>
												{formatKey(activeHeroTrack.key, 'camelot')}
											</span>
										{:else}
											<span class="rounded-full bg-surface-2 border border-stroke-subtle px-2.5 py-0.5 text-[10px] font-mono font-bold text-text-primary">
												{activeHeroTrack.key}
											</span>
										{/if}

										<button
											type="button"
											class="inline-flex items-center gap-1 rounded-full bg-purple-500/15 hover:bg-purple-500/25 border border-purple-500/30 text-purple-600 dark:text-purple-300 px-2 py-0.5 text-[10px] font-bold shadow-xs hover:scale-105 active:scale-95 transition-all cursor-pointer"
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
										class="inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-[10px] font-bold border shadow-xs"
										style="background-color: {energyInfo.bg}; color: {energyInfo.color}; border-color: {energyInfo.border};"
										title="Énergie: {energyInfo.descriptor} ({activeHeroTrack.energy}/10)"
									>
										<span>⚡</span>
										<span>{activeHeroTrack.energy}</span>
									</span>
								{/if}
							</div>

							<!-- Action Buttons & Mode Switch -->
							<div class="flex items-center gap-3 flex-shrink-0">
								<!-- Mode Toggle: [📋 Récents | 💿 Albums] -->
								<div class="inline-flex items-center p-0.5 rounded-full bg-surface-2/90 border border-stroke-subtle shadow-inner">
									<button
										type="button"
										class="flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold transition-all cursor-pointer {!isAlbumMode ? 'bg-cyan-500 text-black shadow-xs font-bold' : 'text-text-secondary hover:text-text-primary'}"
										onclick={() => isAlbumMode = false}
									>
										<Icon name="disc" class="h-3 w-3" />
										<span>Récents</span>
									</button>
									<button
										type="button"
										class="flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold transition-all cursor-pointer {isAlbumMode ? 'bg-cyan-500 text-black shadow-xs font-bold' : 'text-text-secondary hover:text-text-primary'}"
										onclick={() => isAlbumMode = true}
									>
										<Icon name="folder-open" class="h-3 w-3" />
										<span>Mode Album</span>
									</button>
								</div>

								{#if !activeHeroTrack.is_in_library}
									<button
										type="button"
										class="flex items-center gap-1.5 rounded-full bg-brand-primary px-3 py-1.5 text-xs font-semibold text-black shadow-md shadow-brand-primary/20 hover:brightness-110 active:scale-95 transition-all cursor-pointer disabled:opacity-50"
										onclick={handleImportToLibrary}
										disabled={isImporting}
									>
										<Icon name={isImporting ? 'loader' : 'plus'} class="h-3.5 w-3.5 {isImporting ? 'animate-spin' : ''}" />
										<span>{isImporting ? 'Ajout...' : 'Ajouter à ma bibliothèque'}</span>
									</button>
								{/if}

								<Tooltip text="Révéler dans le Finder" position="bottom">
									<button
										type="button"
										class="flex h-8 w-8 items-center justify-center rounded-full border border-stroke-subtle bg-surface-2 text-text-secondary hover:bg-surface-3 hover:text-text-primary active:scale-95 transition-all cursor-pointer"
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
							<h1 class="text-2xl font-bold tracking-tight text-text-primary line-clamp-1" title={activeHeroTrack.title || activeHeroTrack.file_path}>
								{activeHeroTrack.title || activeHeroTrack.file_path.split('/').pop() || 'Morceau Inconnu'}
							</h1>
							<p class="text-base font-medium text-text-secondary mt-0.5 line-clamp-1">
								{activeHeroTrack.artist || 'Artiste inconnu'}
								{#if activeHeroTrack.album}
									<span class="text-text-tertiary font-normal"> — {activeHeroTrack.album}</span>
								{/if}
							</p>
						</div>

						<!-- Bottom Part: Waveform (48px / h-12) & Integrated Controls -->
						<div class="flex flex-col gap-3">
							<!-- Waveform Row with Time Markers -->
							<div class="flex items-center gap-3">
								<span class="text-xs font-mono font-bold text-cyan-600 dark:text-cyan-400 tabular-nums w-12 text-right">
									{formatDuration(currentDisplayPosition)}
								</span>

								<!-- Waveform Seekbar (h-12 / 48px) -->
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<div
									bind:this={waveformContainer}
									class="relative h-12 flex-1 cursor-pointer overflow-hidden rounded-xl bg-surface-2/60 px-2 py-1 transition-all border border-stroke-subtle group"
									onmousedown={handleWaveformMouseDown}
									onmousemove={handleWaveformMouseMove}
									onmouseleave={handleWaveformMouseLeave}
								>
									<!-- Waveform Bars Display -->
									<div class="flex h-full w-full items-center justify-between gap-[2px]">
										{#each activeWaveformBars as barHeight, i}
											{@const barPercent = (i / activeWaveformBars.length) * 100}
											{@const isPlayed = barPercent <= progressPercent}
											<div
												class="flex-1 rounded-full {isPlayed ? 'bg-cyan-400 shadow-[0_0_6px_rgba(6,182,212,0.7)]' : 'bg-text-tertiary/20 group-hover:bg-text-tertiary/35'}"
												style="height: {barHeight}%;"
											></div>
										{/each}
									</div>

									<!-- Hot Cue Visual Pins on Waveform -->
									{#if effectiveDuration > 0}
										{#each $currentCues as cue, idx}
											{@const cuePercent = Math.min(100, Math.max(0, (cue.position_ms / effectiveDuration) * 100))}
											{@const cueNum = cue.hot_cue_index != null ? cue.hot_cue_index : idx + 1}
											<!-- svelte-ignore a11y_click_events_have_key_events -->
											<button
												type="button"
												class="absolute top-0 bottom-0 z-10 w-4 -translate-x-1/2 flex flex-col items-center justify-between group/pin cursor-pointer transition-all hover:scale-110"
												style="left: {cuePercent}%"
												onclick={(e) => {
													e.stopPropagation()
													playerStore.jumpToCue(cue)
												}}
												title="Hot Cue {cueNum}: {cue.name || formatDuration(cue.position_ms)}"
											>
												<div class="h-3.5 w-3.5 rounded-xs bg-amber-400 text-[8px] font-black text-black flex items-center justify-center shadow-[0_0_8px_rgba(251,191,36,0.9)] border border-amber-200">
													{cueNum}
												</div>
												<div class="w-[1.5px] flex-1 bg-amber-400/80 group-hover/pin:bg-amber-300 shadow-[0_0_4px_rgba(251,191,36,0.6)]"></div>
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
											class="pointer-events-none absolute top-1 rounded-md bg-surface-1 px-2 py-0.5 text-[10px] font-mono font-bold text-cyan-400 border border-cyan-500/40 -translate-x-1/2 shadow-xl"
											style="left: {hoverWaveformPercent * 100}%"
										>
											{formatDuration(hoverWaveformPercent * effectiveDuration)}
										</div>
									{/if}
								</div>

								<span class="text-xs font-mono font-semibold text-text-tertiary tabular-nums w-12">
									{formatDuration(effectiveDuration)}
								</span>
							</div>

							<!-- 8 Hot Cue Pads Row -->
							<div class="grid grid-cols-8 gap-2 px-1 py-1 bg-surface-1/40 rounded-xl border border-stroke-subtle">
								{#each cuePads as pad}
									<button
										type="button"
										class="flex flex-col items-center justify-center py-1.5 px-1 rounded-lg border transition-all select-none {pad.cue ? 'bg-amber-500/15 border-amber-500/40 text-amber-500 dark:text-amber-400 hover:bg-amber-500/25 hover:border-amber-400 active:scale-95 shadow-xs cursor-pointer' : 'bg-surface-2/40 border-white/5 text-text-tertiary/40 hover:bg-surface-2/60 cursor-default'}"
										onclick={() => handleCuePadClick(pad.slot, pad.cue)}
										disabled={!pad.cue}
										title={pad.cue ? `Sauter au Hot Cue ${pad.slot} (${pad.cue.name || formatDuration(pad.cue.position_ms)})` : `Cue ${pad.slot} non défini`}
									>
										<span class="text-[11px] font-extrabold font-mono leading-none">CUE {pad.slot}</span>
										<span class="text-[9px] font-mono mt-0.5 tabular-nums opacity-90">
											{pad.cue ? formatDuration(pad.cue.position_ms) : '--:--'}
										</span>
									</button>
								{/each}
							</div>

							<!-- Controls: Centered Transport Controls -->
							<div class="flex items-center justify-center gap-3">
								<button
									type="button"
									class="flex h-10 w-10 items-center justify-center rounded-full bg-surface-2 border border-stroke-subtle text-text-secondary hover:bg-surface-3 hover:text-text-primary active:scale-95 transition-all cursor-pointer shadow-xs"
									onclick={() => handleSeek(0)}
									title="Revenir au début"
								>
									<Icon name="skip-back" class="h-4 w-4" fill />
								</button>

								<button
									type="button"
									class="flex h-[52px] w-[52px] items-center justify-center rounded-full bg-cyan-400 text-black shadow-lg shadow-cyan-400/40 hover:brightness-110 active:scale-95 transition-all cursor-pointer"
									onclick={handleTogglePlayPause}
									title={isCurrentPlayingInHero ? 'Pause' : 'Lecture'}
								>
									<Icon name={isCurrentPlayingInHero ? 'pause' : 'play'} class="h-7 w-7 ml-0.5" fill />
								</button>

								<button
									type="button"
									class="flex h-10 w-10 items-center justify-center rounded-full bg-surface-2 border border-stroke-subtle text-text-secondary hover:bg-surface-3 hover:text-text-primary active:scale-95 transition-all cursor-pointer shadow-xs"
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
				<div class="flex items-center justify-between py-12 px-6">
					<div class="flex items-center gap-6">
						<div class="flex h-20 w-20 items-center justify-center rounded-2xl bg-surface-2 border border-stroke-subtle text-cyan-600 dark:text-cyan-400 shadow-inner">
							<Icon name="disc" class="h-10 w-10 opacity-80" />
						</div>
						<div>
							<h2 class="text-xl font-bold text-text-primary">Lecteur Audio Crate</h2>
							<p class="text-sm text-text-secondary mt-1">
								Ouvrez n'importe quel fichier audio depuis le Finder pour l'écouter instantanément.
							</p>
						</div>
					</div>

					<!-- Mode Toggle even in empty hero -->
					<div class="inline-flex items-center p-0.5 rounded-full bg-surface-2 border border-stroke-subtle shadow-inner">
						<button
							type="button"
							class="flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold transition-all cursor-pointer {!isAlbumMode ? 'bg-cyan-500 text-black shadow-xs font-bold' : 'text-text-secondary hover:text-text-primary'}"
							onclick={() => isAlbumMode = false}
						>
							<Icon name="disc" class="h-3 w-3" />
							<span>Récents</span>
						</button>
						<button
							type="button"
							class="flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold transition-all cursor-pointer {isAlbumMode ? 'bg-cyan-500 text-black shadow-xs font-bold' : 'text-text-secondary hover:text-text-primary'}"
							onclick={() => isAlbumMode = true}
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
		<div class="w-full border-y border-stroke-subtle bg-surface-0/90 backdrop-blur-md px-6 py-2 flex-shrink-0 z-20 shadow-xs">
			<!-- Ligne 1 : Titre + Barre de recherche instantanée + Bouton Effacer l'historique -->
			<div class="flex items-center justify-between pb-2 gap-4">
				<div class="flex items-center gap-4 flex-1 min-w-0">
					<div class="flex items-center gap-2.5 flex-shrink-0">
						<Icon name="disc" class="h-4 w-4 text-cyan-600 dark:text-cyan-400" />
						<h2 class="text-sm font-bold text-text-primary">
							Fichiers Récents
						</h2>
					</div>

					<!-- Barre de recherche instantanée -->
					<div class="relative flex items-center max-w-sm w-full">
						<Icon name="search" class="absolute left-2.5 h-3.5 w-3.5 text-text-tertiary pointer-events-none" />
						<input
							type="text"
							placeholder="Filtrer les récents (titre, artiste, format, clé)..."
							bind:value={recentSearchQuery}
							class="h-7 w-full rounded-full bg-surface-2/80 border border-stroke-subtle pl-8 pr-7 text-xs text-text-primary placeholder:text-text-tertiary focus:border-cyan-500 focus:bg-surface-2 focus:outline-hidden transition-all"
						/>
						{#if recentSearchQuery}
							<button
								type="button"
								class="absolute right-2 flex h-4 w-4 items-center justify-center rounded-full text-text-tertiary hover:text-text-primary cursor-pointer"
								onclick={() => recentSearchQuery = ''}
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
						class="flex items-center gap-1.5 rounded-full border border-stroke-subtle bg-surface-2 px-2.5 py-0.5 text-[11px] font-medium text-text-tertiary hover:border-red-500/30 hover:bg-red-500/10 hover:text-red-500 dark:hover:text-red-400 active:scale-95 transition-all cursor-pointer flex-shrink-0"
						onclick={handleClearRecent}
					>
						<Icon name="trash" class="h-3 w-3" />
						<span>Effacer l'historique</span>
					</button>
				{/if}
			</div>

			<!-- Ligne 2 : En-têtes de colonnes fixes -->
			<div class="grid grid-cols-[32px_36px_minmax(180px,2fr)_minmax(120px,1.2fr)_110px_110px_64px_130px_60px] items-center gap-3 px-3 py-1 text-[10px] font-semibold uppercase tracking-wider text-text-tertiary border-t border-stroke-subtle">
				<span class="pl-1">#</span>
				<span>Cover</span>
				<span>Titre & Artiste</span>
				<span>Album</span>
				<span>Format</span>
				<span>BPM / Clé</span>
				<span class="text-right">Durée</span>
				<span>Dernière lecture</span>
				<span class="text-right pr-1">Actions</span>
			</div>
		</div>

		<!-- LISTE DÉFILANTE DES SONS UNIQUEMENT (flex-1 overflow-y-auto) -->
		<div class="flex-1 overflow-y-auto px-6 py-1 scroll-smooth">
			{#if $recentTracksLoading}
				<div class="flex h-32 items-center justify-center text-text-tertiary gap-2">
					<Icon name="loader" class="h-4 w-4 animate-spin" />
					<span class="text-xs">Chargement de l'historique...</span>
				</div>
			{:else if $recentStandaloneTracks.length === 0}
				<!-- Empty State -->
				<div class="flex flex-col items-center justify-center py-12 text-center">
					<div class="flex h-12 w-12 items-center justify-center rounded-2xl bg-surface-2 border border-stroke-subtle mb-2.5 text-cyan-600 dark:text-cyan-400/60 shadow-inner">
						<Icon name="disc" class="h-6 w-6" />
					</div>
					<h3 class="text-sm font-semibold text-text-primary">Aucun fichier audio récent</h3>
					<p class="text-xs text-text-tertiary max-w-sm mt-1">
						Double-cliquez sur un fichier audio dans le Finder ou ouvrez-le avec Crate pour l'écouter directement sans encombrer votre bibliothèque.
					</p>
				</div>
			{:else if filteredRecentTracks.length === 0}
				<!-- Empty Filter State -->
				<div class="flex flex-col items-center justify-center py-12 text-center">
					<div class="flex h-10 w-10 items-center justify-center rounded-full bg-surface-2 border border-stroke-subtle mb-2 text-text-tertiary">
						<Icon name="search" class="h-5 w-5" />
					</div>
					<h3 class="text-xs font-semibold text-text-primary">Aucun résultat pour « {recentSearchQuery} »</h3>
					<button
						type="button"
						class="mt-2 text-xs text-cyan-600 dark:text-cyan-400 hover:underline cursor-pointer"
						onclick={() => recentSearchQuery = ''}
					>
						Effacer le filtre
					</button>
				</div>
			{:else}
				<div class="flex flex-col divide-y divide-stroke-subtle/50 py-1">
					{#each filteredRecentTracks as track, index}
						{@const isPlayingThis = $isPlaying && $playbackSource === 'standalone' && $standaloneTrack?.file_path === track.file_path}
						{@const trackArtUrl = track.artwork_path ? getArtworkUrl(track.artwork_path, $appDataDir) : null}
						{@const trackCamelot = track.key ? getCamelotColor(track.key) : null}

						<!-- Track Row -->
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<div
							class="grid grid-cols-[32px_36px_minmax(180px,2fr)_minmax(120px,1.2fr)_110px_110px_64px_130px_60px] items-center gap-3 px-3 py-1.5 rounded-lg text-xs transition-colors cursor-pointer group {isPlayingThis ? 'bg-cyan-500/10 text-cyan-600 dark:text-cyan-300' : 'hover:bg-surface-1 text-text-primary'}"
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
									<span class="group-hover:hidden">{index + 1}</span>
									<Icon name="play" class="h-3.5 w-3.5 hidden group-hover:block text-cyan-600 dark:text-cyan-400" fill />
								{/if}
							</div>

							<!-- Cover Thumbnail -->
							<div class="h-8 w-8 overflow-hidden rounded-lg bg-surface-2/60 border border-stroke-subtle flex-shrink-0">
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
								<div class="font-semibold text-text-primary group-hover:text-cyan-600 dark:group-hover:text-cyan-400 transition-colors truncate">
									{track.title || track.file_path.split('/').pop() || 'Morceau inconnu'}
								</div>
								<div class="text-[11px] text-text-secondary truncate">
									{track.artist || 'Artiste inconnu'}
								</div>
							</div>

							<!-- Album -->
							<div class="text-text-secondary truncate text-[11px] pr-2">
								{track.album || '-'}
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
							<div class="font-mono text-right text-text-secondary tabular-nums text-[11px]">
								{formatDuration(track.duration_ms)}
							</div>

							<!-- Last Played -->
							<div class="text-[11px] text-text-tertiary truncate">
								{track.last_played_at ? formatDate(track.last_played_at, 'locale') : '-'}
							</div>

							<!-- Actions -->
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<!-- svelte-ignore a11y_click_events_have_key_events -->
							<div class="pr-1 text-right" onclick={(e) => e.stopPropagation()}>
								<div class="flex items-center justify-end gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
									<Tooltip text="Révéler dans le Finder" position="top">
										<button
											type="button"
											class="p-1 rounded-full text-text-tertiary hover:bg-surface-2 hover:text-text-primary cursor-pointer transition-colors"
											onclick={() => handleRevealInFinder(track.file_path)}
										>
											<Icon name="folder-open" class="h-3 w-3" />
										</button>
									</Tooltip>

									<Tooltip text="Supprimer de l'historique" position="top">
										<button
											type="button"
											class="p-1 rounded-full text-text-tertiary hover:bg-red-500/10 hover:text-red-500 dark:hover:text-red-400 cursor-pointer transition-colors"
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
