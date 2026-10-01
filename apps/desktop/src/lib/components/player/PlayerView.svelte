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
		language,
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
		formatDate,
		formatNumber,
		getHarmonicKeys,
	} from '$shared/utils'
	import { translate } from '$shared/i18n'
	import { Button, EnergyBadge, Icon, KeyBadge, SegmentedControl, Spinner, Tooltip } from '$lib/components/common'
	import type { SegmentOption } from '$lib/components/common'
	import AlbumGridView from './AlbumGridView.svelte'
	import AlbumDetailView from './AlbumDetailView.svelte'
	import { waveformSeekTarget } from './waveformSeek'
	import { recentRowForKey } from './recentKeys'

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
	// Keyboard navigation of the recent files (see recentKeys.ts): the list is one tab stop with an active row.
	let activeRecentIndex = $state(0)
	let recentListEl: HTMLDivElement | null = $state(null)

	// Keep the active row inside the list when the filter shortens it.
	$effect(() => {
		const count = filteredRecentTracks.length
		if (activeRecentIndex > 0 && activeRecentIndex >= count) activeRecentIndex = Math.max(0, count - 1)
	})

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

	type BottomMode = 'recent' | 'albums'

	const modeOptions: SegmentOption<BottomMode>[] = $derived([
		{ value: 'recent', label: $translate('player.mode.recent'), icon: 'disc' },
		{ value: 'albums', label: $translate('player.mode.albums'), icon: 'folder-open' },
	])

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
			toastStore.warning($translate('player.toast.noCamelotKey'))
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

		const mixValues = {
			title: activeHeroTrack.title || $translate('player.trackFallback'),
			key: formatKey(activeHeroTrack.key, 'camelot'),
			keys: compatibleKeys.join(', '),
		}
		toastStore.success(
			currentBpm
				? $translate('player.toast.harmonicMixBpm', {
						values: { ...mixValues, bpm: formatNumber(Math.round(currentBpm), $language) },
					})
				: $translate('player.toast.harmonicMix', { values: mixValues })
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

	// Keyboard seeking on the focused waveform (same steps as the global shortcuts, see waveformSeek.ts). The event
	// is stopped so the global arrow shortcut does not seek a second time.
	function handleWaveformKeydown(e: KeyboardEvent) {
		const target = waveformSeekTarget(e, currentDisplayPosition, effectiveDuration)
		if (target === null) return
		e.preventDefault()
		e.stopPropagation()
		playerStore.seek(target)
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
				toastStore.success(
					$translate('player.toast.imported', {
						values: { title: imported.title || $translate('player.trackFallback') },
					})
				)
			} else if (res.errors.length > 0) {
				toastStore.error($translate('player.toast.importFailed', { values: { error: res.errors[0] } }))
			}
		} catch (err) {
			console.error('Import standalone track error:', err)
			toastStore.error($translate('player.toast.addToLibraryFailed'))
		} finally {
			isImporting = false
		}
	}

	async function handleRevealInFinder(filePath: string) {
		try {
			await revealItemInDir(filePath)
		} catch (err) {
			console.error('Reveal in finder error:', err)
			toastStore.error($translate('player.toast.revealFailed'))
		}
	}

	// A click on a row plays it, except in the row's action cell (reveal, remove), even between its two buttons.
	function handleRecentListClick(e: MouseEvent) {
		const target = e.target as HTMLElement
		if (target.closest('[data-row-actions]')) return
		const row = target.closest<HTMLElement>('[data-recent-index]')
		if (!row) return
		const index = Number(row.dataset.recentIndex)
		const track = filteredRecentTracks[index]
		if (!track) return
		activeRecentIndex = index
		handlePlayTrack(track)
	}

	// Keys on the focused list: arrows, Home and End move the active row, Enter or Space plays it. Only for keyboard
	// focus (a list focused by a mouse click leaves the arrows to the global volume and seek shortcuts, as before);
	// handled keys are stopped so the global Enter / Space / arrow shortcuts do not also run.
	function handleRecentListKeydown(e: KeyboardEvent) {
		if (e.target !== e.currentTarget || !recentListEl?.matches(':focus-visible')) return
		const count = filteredRecentTracks.length
		if (count === 0) return
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault()
			e.stopPropagation()
			const track = filteredRecentTracks[Math.min(activeRecentIndex, count - 1)]
			if (track) handlePlayTrack(track)
			return
		}
		const next = recentRowForKey(e.key, activeRecentIndex, count)
		if (next === null) return
		e.preventDefault()
		e.stopPropagation()
		activeRecentIndex = next
		document.getElementById(`recent-row-${next}`)?.scrollIntoView({ block: 'nearest' })
	}

	async function handleRemoveRecent(e: MouseEvent, id: string) {
		e.stopPropagation()
		await recentTracksStore.removeTrack(id)
	}

	async function handleClearRecent() {
		await recentTracksStore.clear()
		toastStore.info($translate('player.toast.historyCleared'))
	}

	function formatSampleRateDisplay(sr: number | null | undefined): string {
		if (!sr) return '-'
		// Same rounding as before (one decimal, none for whole kHz), with the app language's decimal separator
		const khz = formatNumber(Math.round(sr / 100) / 10, $language)
		return `${khz} kHz`
	}
</script>

<!-- `background`: the hero's toggle is slightly translucent, the empty hero's one is opaque (as before) -->
{#snippet modeToggle(background: string)}
	<SegmentedControl
		variant="deck"
		class={background}
		ariaLabel={$translate('player.mode.label')}
		options={modeOptions}
		value={isAlbumMode ? 'albums' : 'recent'}
		onchange={(mode) => (isAlbumMode = mode === 'albums')}
	/>
{/snippet}

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
		class="relative z-20 w-full flex-shrink-0 border-b border-stroke-subtle bg-surface-1/60 px-6 py-4 shadow-xl backdrop-blur-xl dark:shadow-2xl [@media(min-height:820px)]:p-8"
	>
		<div class="mx-auto w-full max-w-7xl">
			{#if activeHeroTrack}
				<div class="flex w-full items-center gap-6 md:gap-8">
					<!-- Artwork: scales with the window height (140–260 px) instead of a fixed 225 px -->
					<div
						class="group relative size-[clamp(140px,22vh,260px)] flex-shrink-0 overflow-hidden rounded-2xl border border-stroke-subtle bg-surface-2/60 shadow-2xl shadow-black/20 dark:shadow-black/60"
					>
						{#if heroArtworkUrl}
							<img
								src={heroArtworkUrl}
								alt={$translate('common.albumArtwork')}
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
							aria-label={$translate('player.hero.playPause')}
						>
							<div
								class="flex h-14 w-14 items-center justify-center rounded-full bg-deck-live text-black shadow-lg shadow-deck-live/50 transition-transform group-hover:scale-105 active:scale-95"
							>
								<Icon name={isCurrentPlayingInHero ? 'pause' : 'play'} class="ml-0.5 h-6 w-6" fill />
							</div>
						</button>
					</div>

					<!-- Metadata, Waveform & Controls Column -->
					<div class="flex min-h-[clamp(140px,22vh,260px)] min-w-0 flex-1 flex-col justify-between gap-3">
						<!-- Top Part: Micro-pills Badges (Left) & Controls/Mode Toggle (Right) -->
						<div class="flex w-full items-center justify-between gap-4">
							<!-- Badges micro-pills horizontaux -->
							<div class="flex flex-wrap items-center gap-2">
								<!-- Source -->
								{#if activeHeroTrack.is_in_library}
									<span
										class="inline-flex items-center gap-1 rounded-full border border-brand-primary/30 bg-brand-primary/15 px-2.5 py-0.5 text-[10px] font-semibold text-brand-primary"
									>
										<Icon name="library" class="h-2.5 w-2.5" />
										{$translate('player.hero.sourceLibrary')}
									</span>
								{:else}
									<span
										class="inline-flex items-center gap-1 rounded-full border border-deck-cue-tint/30 bg-deck-cue-tint/15 px-2.5 py-0.5 text-[10px] font-semibold text-deck-cue-text-strong"
									>
										<Icon name="hard-drive" class="h-2.5 w-2.5" />
										{$translate('player.hero.sourceExternal')}
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
										class="rounded-full border border-deck-live-tint/20 bg-deck-live-tint/10 px-2.5 py-0.5 font-mono text-[10px] font-bold text-deck-live-text-strong"
									>
										{formatBpm(activeHeroTrack.bpm)} BPM
									</span>
								{/if}

								<!-- Camelot Key & Mix Harmonique (1 Clic) -->
								{#if activeHeroTrack.key}
									<div class="inline-flex items-center gap-1.5">
										<KeyBadge
											value={activeHeroTrack.key}
											label={formatKey(activeHeroTrack.key, 'camelot')}
											variant="pill"
										/>

										<button
											type="button"
											class="inline-flex cursor-pointer items-center gap-1 rounded-full border border-purple-500/30 bg-purple-500/15 px-2 py-0.5 text-[10px] font-bold text-purple-600 shadow-xs transition-all hover:scale-105 hover:bg-purple-500/25 active:scale-95 dark:text-purple-300"
											onclick={handleHarmonicMatch}
											title={$translate('player.hero.harmonicMixHint')}
										>
											<Icon name="sparkles" class="h-3 w-3 text-purple-500 dark:text-purple-300" />
											<span>{$translate('player.hero.harmonicMix')}</span>
										</button>
									</div>
								{/if}

								<!-- Energy -->
								{#if activeHeroTrack.energy}
									<EnergyBadge energy={activeHeroTrack.energy} variant="pill" />
								{/if}
							</div>

							<!-- Action Buttons & Mode Switch -->
							<div class="flex flex-shrink-0 items-center gap-3">
								<!-- Bottom section: recent files or album mode -->
								{@render modeToggle('bg-surface-2/90')}

								{#if !activeHeroTrack.is_in_library}
									<Button
										variant="primary"
										size="bare"
										shape="full"
										weight="semibold"
										display="flex"
										glow="md/20"
										press
										class="gap-1.5 px-3 py-1.5 text-xs"
										onclick={handleImportToLibrary}
										disabled={isImporting}
									>
										{#if isImporting}
											<Spinner icon="loader" color="current" class="h-3.5 w-3.5" />
										{:else}
											<Icon name="plus" class="h-3.5 w-3.5" />
										{/if}
										<span
											>{isImporting ? $translate('player.hero.adding') : $translate('player.hero.addToLibrary')}</span
										>
									</Button>
								{/if}

								<Tooltip text={$translate('player.recent.reveal')} position="bottom">
									<button
										type="button"
										class="flex h-8 w-8 cursor-pointer items-center justify-center rounded-full border border-stroke-subtle bg-surface-2 text-text-secondary transition-all hover:bg-surface-3 hover:text-text-primary active:scale-95"
										onclick={() => handleRevealInFinder(activeHeroTrack.file_path)}
										aria-label={$translate('player.recent.reveal')}
									>
										<Icon name="folder-open" class="h-4 w-4 text-deck-live-text" />
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
								{activeHeroTrack.title ||
									activeHeroTrack.file_path.split('/').pop() ||
									$translate('player.unknownTrack')}
							</h1>
							<p class="mt-0.5 line-clamp-1 text-base font-medium text-text-secondary">
								{activeHeroTrack.artist || $translate('common.unknownArtist')}
								{#if activeHeroTrack.album}
									<span class="font-normal text-text-tertiary"> — {activeHeroTrack.album}</span>
								{/if}
							</p>
						</div>

						<!-- Bottom Part: Waveform (48px / h-12) & Integrated Controls -->
						<div class="flex flex-col gap-3">
							<!-- Waveform Row with Time Markers -->
							<div class="flex items-center gap-3">
								<span class="w-12 text-right font-mono text-xs font-bold text-deck-live-text tabular-nums">
									{formatDuration(currentDisplayPosition)}
								</span>

								<!-- Waveform Seekbar (h-12 / 48px): a slider for the keyboard (arrows, Page Up/Down, Home/End) -->
								<div
									bind:this={waveformContainer}
									class="group relative h-12 flex-1 cursor-pointer overflow-hidden rounded-xl border border-stroke-subtle bg-surface-2/60 px-2 py-1 transition-all"
									role="slider"
									tabindex="0"
									aria-label={$translate('player.waveform.label')}
									aria-valuemin={0}
									aria-valuemax={Math.round(effectiveDuration / 1000)}
									aria-valuenow={Math.round(currentDisplayPosition / 1000)}
									aria-valuetext={$translate('player.waveform.position', {
										values: {
											position: formatDuration(currentDisplayPosition),
											duration: formatDuration(effectiveDuration),
										},
									})}
									onkeydown={handleWaveformKeydown}
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
													? 'bg-deck-live shadow-[0_0_6px_rgba(6,182,212,0.7)]'
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
												title={cue.cue_type === 'hot'
													? $translate('player.cues.hotPin', {
															values: { number: cueNum, name: cue.name || formatDuration(cue.position_ms) },
														})
													: $translate('player.cues.memoryPin', {
															values: { name: cue.name || formatDuration(cue.position_ms) },
														})}
											>
												<div
													class="flex h-3.5 w-3.5 items-center justify-center rounded-xs border border-deck-cue-outline bg-deck-cue text-[8px] font-black text-black shadow-[0_0_8px_rgba(251,191,36,0.9)]"
												>
													{cueNum}
												</div>
												<div
													class="w-[1.5px] flex-1 bg-deck-cue/80 shadow-[0_0_4px_rgba(251,191,36,0.6)] group-hover/pin:bg-deck-cue-text-strong"
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
											class="pointer-events-none absolute top-1 -translate-x-1/2 rounded-md border border-deck-live-tint/40 bg-surface-1 px-2 py-0.5 font-mono text-[10px] font-bold text-deck-live-text shadow-xl"
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
											? 'cursor-pointer border-deck-cue-tint/40 bg-deck-cue-tint/15 text-deck-cue-text shadow-xs hover:border-deck-cue hover:bg-deck-cue-tint/25 active:scale-95'
											: 'cursor-default border-white/5 bg-surface-2/40 text-text-tertiary/40 hover:bg-surface-2/60'}"
										onclick={() => handleCuePadClick(pad.slot, pad.cue)}
										disabled={!pad.cue}
										title={pad.cue
											? $translate('player.cues.jumpTo', {
													values: { slot: pad.slot, name: pad.cue.name || formatDuration(pad.cue.position_ms) },
												})
											: $translate('player.cues.notSet', { values: { slot: pad.slot } })}
									>
										<span class="font-mono text-[11px] leading-none font-extrabold"
											>{$translate('player.cues.pad', { values: { slot: pad.slot } })}</span
										>
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
									class="flex h-10 w-10 cursor-pointer items-center justify-center rounded-full border border-stroke-subtle bg-surface-2 text-text-secondary shadow-xs transition-all hover:bg-surface-3 hover:text-text-primary active:scale-95"
									onclick={() => handleSeek(0)}
									title={$translate('player.hero.backToStart')}
								>
									<Icon name="skip-back" class="h-4 w-4" fill />
								</button>

								<button
									type="button"
									class="flex h-[52px] w-[52px] cursor-pointer items-center justify-center rounded-full bg-deck-live text-black shadow-lg shadow-deck-live/40 transition-all hover:brightness-110 active:scale-95"
									onclick={handleTogglePlayPause}
									title={isCurrentPlayingInHero ? $translate('player.pause') : $translate('player.play')}
								>
									<Icon name={isCurrentPlayingInHero ? 'pause' : 'play'} class="ml-0.5 h-7 w-7" fill />
								</button>

								<button
									type="button"
									class="flex h-10 w-10 cursor-pointer items-center justify-center rounded-full border border-stroke-subtle bg-surface-2 text-text-secondary shadow-xs transition-all hover:bg-surface-3 hover:text-text-primary active:scale-95"
									onclick={handleStop}
									title={$translate('player.hero.stopPlayback')}
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
							class="flex h-20 w-20 items-center justify-center rounded-2xl border border-stroke-subtle bg-surface-2 text-deck-live-text shadow-inner"
						>
							<Icon name="disc" class="h-10 w-10 opacity-80" />
						</div>
						<div>
							<h2 class="text-xl font-bold text-text-primary">{$translate('player.hero.emptyTitle')}</h2>
							<p class="mt-1 text-sm text-text-secondary">
								{$translate('player.hero.emptyHint')}
							</p>
						</div>
					</div>

					<!-- Mode toggle, also available in the empty hero -->
					{@render modeToggle('bg-surface-2')}
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
						<Icon name="disc" class="h-4 w-4 text-deck-live-text" />
						<h2 class="text-sm font-bold text-text-primary">{$translate('player.recent.list')}</h2>
					</div>

					<!-- Barre de recherche instantanée -->
					<div class="relative flex w-full max-w-sm items-center">
						<Icon name="search" class="pointer-events-none absolute left-2.5 h-3.5 w-3.5 text-text-tertiary" />
						<input
							type="text"
							placeholder={$translate('player.recent.filterPlaceholder')}
							bind:value={recentSearchQuery}
							class="h-7 w-full rounded-full border border-stroke-subtle bg-surface-2/80 pr-7 pl-8 text-xs text-text-primary transition-all placeholder:text-text-tertiary focus:border-deck-live-tint focus:bg-surface-2 focus:outline-hidden"
						/>
						{#if recentSearchQuery}
							<button
								type="button"
								class="absolute right-2 flex h-4 w-4 cursor-pointer items-center justify-center rounded-full text-text-tertiary hover:text-text-primary"
								onclick={() => (recentSearchQuery = '')}
								aria-label={$translate('player.recent.clearSearch')}
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
						<span>{$translate('player.recent.clearHistory')}</span>
					</button>
				{/if}
			</div>

			<!-- Ligne 2 : En-têtes de colonnes fixes -->
			<div
				class="grid grid-cols-[32px_36px_minmax(160px,2fr)_minmax(0,1.2fr)_110px_110px_64px_130px_60px] items-center gap-3 border-t border-stroke-subtle px-3 py-1 text-[10px] font-semibold tracking-wider text-text-tertiary uppercase"
			>
				<span class="pl-1">#</span>
				<span>{$translate('player.columns.cover')}</span>
				<span>{$translate('player.columns.titleArtist')}</span>
				<span>{$translate('library.columns.album')}</span>
				<span>{$translate('library.columns.format')}</span>
				<span>{$translate('player.columns.bpmKey')}</span>
				<span class="text-right">{$translate('player.columns.duration')}</span>
				<span>{$translate('player.columns.lastPlayed')}</span>
				<span class="pr-1 text-right">{$translate('player.columns.actions')}</span>
			</div>
		</div>

		<!-- LISTE DÉFILANTE DES SONS UNIQUEMENT (flex-1 overflow-y-auto) -->
		{#if !$recentTracksLoading && filteredRecentTracks.length > 0}
			<!-- The scroll area is a grid: one tab stop, the active row outlined only for keyboard focus (recentKeys.ts).
			     A click on a row plays it, except in its action cell, as before. -->
			<div
				bind:this={recentListEl}
				class="group/recents min-h-0 flex-1 overflow-y-auto scroll-smooth px-6 py-1 focus-visible:outline-none"
				role="grid"
				tabindex="0"
				aria-label={$translate('player.recent.list')}
				aria-activedescendant="recent-row-{Math.min(activeRecentIndex, filteredRecentTracks.length - 1)}"
				onclick={handleRecentListClick}
				onkeydown={handleRecentListKeydown}
			>
				<div class="flex flex-col divide-y divide-stroke-subtle/50 py-1">
					{#each filteredRecentTracks as track, index (track.id)}
						{@const isPlayingThis =
							$isPlaying && $playbackSource === 'standalone' && $standaloneTrack?.file_path === track.file_path}
						{@const trackArtUrl = track.artwork_path ? getArtworkUrl(track.artwork_path, $appDataDir) : null}

						<!-- Track Row: a click plays it (handled by the list); from the keyboard it is the list's active row -->
						<div
							id="recent-row-{index}"
							role="row"
							tabindex="-1"
							aria-selected={index === activeRecentIndex}
							class="group grid cursor-pointer grid-cols-[32px_36px_minmax(160px,2fr)_minmax(0,1.2fr)_110px_110px_64px_130px_60px] items-center gap-3 rounded-lg px-3 py-1.5 text-xs transition-colors {index ===
							activeRecentIndex
								? 'group-focus-visible/recents:outline-2 group-focus-visible/recents:-outline-offset-2 group-focus-visible/recents:outline-brand-primary'
								: ''} {isPlayingThis
								? 'bg-deck-live-tint/10 text-deck-live-text-strong'
								: 'text-text-primary hover:bg-surface-1'}"
							data-recent-index={index}
						>
							<!-- Index / Play Icon / Equalizer Animation -->
							<div class="flex items-center pl-1 font-mono text-text-tertiary">
								{#if isPlayingThis}
									<div class="flex h-3.5 w-3.5 items-end gap-[2px] text-deck-live-text">
										<span class="animate-eq-1 w-[3px] rounded-full bg-deck-live-text"></span>
										<span class="animate-eq-2 w-[3px] rounded-full bg-deck-live-text"></span>
										<span class="animate-eq-3 w-[3px] rounded-full bg-deck-live-text"></span>
									</div>
								{:else}
									<span class="group-hover:hidden">{index + 1}</span>
									<Icon name="play" class="hidden h-3.5 w-3.5 text-deck-live-text group-hover:block" fill />
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
								<div class="truncate font-semibold text-text-primary transition-colors group-hover:text-deck-live-text">
									{track.title || track.file_path.split('/').pop() || $translate('player.unknownTrack')}
								</div>
								<div class="truncate text-[11px] text-text-secondary">
									{track.artist || $translate('common.unknownArtist')}
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
									<span class="font-mono font-bold text-deck-live-text-strong">{formatBpm(track.bpm)}</span>
								{/if}
								{#if track.key}
									<KeyBadge value={track.key} label={formatKey(track.key, 'camelot')} variant="pill-xs" />
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
								{track.last_played_at ? formatDate(track.last_played_at, 'locale', $language) : '-'}
							</div>

							<!-- Actions (shown on hover, or while one of them has keyboard focus) -->
							<div class="pr-1 text-right" data-row-actions>
								<div
									class="flex items-center justify-end gap-1 opacity-0 transition-opacity group-hover:opacity-100 has-focus-visible:opacity-100"
								>
									<Tooltip text={$translate('player.recent.reveal')} position="top">
										<button
											type="button"
											class="cursor-pointer rounded-full p-1 text-text-tertiary transition-colors hover:bg-surface-2 hover:text-text-primary"
											aria-label={$translate('player.recent.reveal')}
											onclick={() => handleRevealInFinder(track.file_path)}
										>
											<Icon name="folder-open" class="h-3 w-3" />
										</button>
									</Tooltip>

									<Tooltip text={$translate('player.recent.remove')} position="top">
										<button
											type="button"
											class="cursor-pointer rounded-full p-1 text-text-tertiary transition-colors hover:bg-red-500/10 hover:text-red-500 dark:hover:text-red-400"
											aria-label={$translate('player.recent.remove')}
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
			</div>
		{:else}
			<div class="min-h-0 flex-1 overflow-y-auto scroll-smooth px-6 py-1">
				{#if $recentTracksLoading}
					<div class="flex h-32 items-center justify-center gap-2 text-text-tertiary">
						<Spinner icon="loader" class="h-4 w-4" color="current" />
						<span class="text-xs">{$translate('player.recent.loading')}</span>
					</div>
				{:else if $recentStandaloneTracks.length === 0}
					<!-- Empty State -->
					<div class="flex flex-col items-center justify-center py-12 text-center">
						<div
							class="mb-2.5 flex h-12 w-12 items-center justify-center rounded-2xl border border-stroke-subtle bg-surface-2 text-deck-live-text/60 shadow-inner"
						>
							<Icon name="disc" class="h-6 w-6" />
						</div>
						<h3 class="text-sm font-semibold text-text-primary">{$translate('player.recent.emptyTitle')}</h3>
						<p class="mt-1 max-w-sm text-xs text-text-tertiary">
							{$translate('player.recent.emptyHint')}
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
						<h3 class="text-xs font-semibold text-text-primary">
							{$translate('player.recent.noResults', { values: { query: recentSearchQuery } })}
						</h3>
						<button
							type="button"
							class="mt-2 cursor-pointer text-xs text-deck-live-text hover:underline"
							onclick={() => (recentSearchQuery = '')}
						>
							{$translate('player.recent.clearFilter')}
						</button>
					</div>
				{/if}
			</div>
		{/if}
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
	/* Reduced motion: the equaliser stands still at mid height instead of looping. */
	@media (prefers-reduced-motion: reduce) {
		.animate-eq-1,
		.animate-eq-2,
		.animate-eq-3 {
			animation: none;
		}
		.animate-eq-1 {
			height: 9px;
		}
		.animate-eq-2 {
			height: 12px;
		}
		.animate-eq-3 {
			height: 7px;
		}
	}
</style>
