import { tick } from 'svelte'
import { derived, get } from 'svelte/store'
import type {
	ActiveView,
	DiscoveryRelease,
	DiscoverySourceType,
	DuplicateTrack,
	Playlist,
	SettingsPage,
	Track,
	UsbDevice,
} from '$shared/types'
import {
	appStore,
	libraryStore,
	sortedTracks,
	displayedTracks,
	playerStore,
	currentTrack,
	shuffleEnabled,
	tagsStore,
	playlistsStore,
	uiStore,
	uiLayoutStore,
	activeView,
	selectedTrackIds,
	selectedReleaseIds,
	settingsStore,
	continuousPlayback,
	devicesStore,
	missingTracksStore,
	missingTrackIds,
	displayedReleases,
	expandedReleaseIds,
	discoveryStore,
	previewInfo,
	standaloneTrack,
	recentStandaloneTracks,
	recentTracksStore,
	playbackSource,
	beatportTrack,
	albumsStore,
} from '$lib/stores'
import { beatportStore } from '$shared/stores/beatport'
import { tagFilterMode } from '$shared/stores/ui'
import { recentlyToggledMixedTags } from '$shared/stores/ui'
import { syncStore } from '$lib/stores/sync'
import { cloudSyncStore } from '$shared/stores/cloudSync'
import { toastStore } from '$shared/stores/toast'
import { exportStore } from '$lib/stores/export'
import { startUpdaterSchedule } from './updaterSchedule'
import {
	buildDiscoveryCandidates,
	createShuffleSession,
	discoveryTrackKey,
	findPlayableTrackIndex,
	navigateQueue,
	nextPreviewTarget,
	previewTargetKey,
	previousPreviewTarget,
	refreshQueueSnapshot,
	resolveQueue,
	sequentialNextIndex,
	sequentialPreviousIndex,
	soleCurrentItem,
	type QueueContext,
	type QueueDirection,
	type QueueLocation,
} from '$shared/utils/playbackQueue'
import { reportSkippedTracks } from '$shared/stores/player'
import { dismissSplash } from '$lib/stores/splash'
import { discoveryPlaylistStore } from '$shared/stores/discoveryPlaylist'
import {
	createTagController,
	createTrackController,
	createDeviceController,
	createExportController,
	createPlaylistController,
} from '$lib/controllers'
import { useAppInitialization } from './useAppInitialization'
import { useKeyboardShortcuts } from './useKeyboardShortcuts'
import { useMenuActions } from './useMenuActions'
import { useMediaKeys } from './useMediaKeys'
import { useDragDropCoordination } from './useDragDropCoordination'
import { translate } from '$shared/i18n'
import * as playlistsApi from '$shared/api/playlists'
import { toErrorMessage } from '$shared/utils/errors'

// =============================================================================
// Types
// =============================================================================

export interface AppSetupConfig {
	getPlaylists: () => Playlist[]
	getDevices: () => UsbDevice[]
	getSelectedPlaylistId: () => string | null
	getSelectedFolderId: () => string | null
	getSelectedTagIds: () => string[]
	getModalOrchestrator: () => ModalOrchestratorRef | undefined
	handleViewChange: (view: ActiveView) => void
	setShowAddReleaseModal: () => void
	setIsDragOver: (dragOver: boolean) => void
}

interface ModalOrchestratorRef {
	isModalOpen: () => boolean
	openSettingsModal: (tab?: SettingsPage) => void
	openCreatePlaylistModal: (parentId: string | null) => void
	openCreateFolderModal: (parentId: string | null) => void
	openCreateSmartPlaylistModal: (parentId: string | null, context?: ActiveView) => void
	openEditSmartPlaylistModal: (playlist: Playlist) => void
	openRenamePlaylistModal: (playlist: Playlist) => void
	openDeletePlaylistModal: (playlist: Playlist, hasChildren: boolean) => void
	openDeletePlaylistBulkModal: (playlists: Playlist[]) => void
	openMoveConflictModal: (playlist: Playlist, conflict: Playlist, targetId: string | null) => void
	openRelocateModal: (track: Track) => void
	openRemoveFromPlaylistModal: (trackIds: string[], playlistId: string) => void
	openRemoveFromLibraryModal: (trackIds: string[]) => void
	openDeleteTrackAndFileModal: (trackIds: string[]) => void
	openRemoveDiscoveryReleasesModal: (releaseIds: string[]) => void
	openRemoveDiscoveryReleasesFromPlaylistModal: (releaseIds: string[], playlistId: string) => void
	openDuplicateTrackModal: (
		duplicates: DuplicateTrack[],
		onComplete: (updatedTracks: Track[], newTracks: Track[], replacedTrackIds: string[]) => void
	) => void
	openDeviceInfoModal: (device: UsbDevice) => void
	openReformatDeviceModal: (device: UsbDevice) => void
	openExportToDeviceModal: (device: UsbDevice) => void
	openExportPlaylistModal: (playlist: Playlist) => void
	openQuickExportModal: () => void
	openExportFailureModal: (error: string, deviceId: string, mountPoint: string, filesCopied: number) => void
}

export interface AppSetupResult {
	tagController: ReturnType<typeof createTagController>
	trackController: ReturnType<typeof createTrackController>
	deviceController: ReturnType<typeof createDeviceController>
	exportController: ReturnType<typeof createExportController>
	playlistController: ReturnType<typeof createPlaylistController>
	playPreview: (release: DiscoveryRelease, trackIndex: number) => void
	playNextTrack: () => void
	playPreviousTrack: () => void
	onMountSetup: () => Promise<() => void>
	setupDragDrop: () => () => void
}

// =============================================================================
// Hook
// =============================================================================

export function createAppSetup(config: AppSetupConfig): AppSetupResult {
	const {
		getPlaylists,
		getDevices,
		getSelectedPlaylistId,
		getSelectedFolderId,
		getSelectedTagIds,
		getModalOrchestrator,
		handleViewChange,
		setShowAddReleaseModal,
		setIsDragOver,
	} = config

	// =========================================================================
	// Controllers
	// =========================================================================

	const tagController = createTagController({
		tagsStore,
		libraryStore,
		discoveryStore,
		uiStore,
		getSelectedTagIds,
		getSelectedPlaylistId,
		getTagFilterMode: () => get(tagFilterMode),
		getSelectedTrackIds: () => get(selectedTrackIds),
		getSelectedReleaseIds: () => get(selectedReleaseIds),
		getRecentlyToggledMixedTags: () => get(recentlyToggledMixedTags),
		getActiveView: () => get(activeView),
	})

	const rawTrackController = createTrackController(
		{
			playerStore,
			libraryStore,
			playlistsStore,
			missingTracksStore,
			uiStore,
			toastStore,
			getSelectedPlaylistId,
			getPlaylists,
			getMissingTrackIds: () => get(missingTrackIds),
		},
		{
			openRelocateModal: (track) => getModalOrchestrator()?.openRelocateModal(track),
			openRemoveFromPlaylistModal: (trackIds, playlistId) =>
				getModalOrchestrator()?.openRemoveFromPlaylistModal(trackIds, playlistId),
			openRemoveFromLibraryModal: (trackIds) => getModalOrchestrator()?.openRemoveFromLibraryModal(trackIds),
			openDeleteTrackAndFileModal: (trackIds) => getModalOrchestrator()?.openDeleteTrackAndFileModal(trackIds),
			openDuplicateTrackModal: (duplicates, onComplete) =>
				getModalOrchestrator()?.openDuplicateTrackModal(duplicates, onComplete),
		}
	)

	// Wrap trackController.play to capture the playback queue context when
	// the user initiates library playback (double-click, Enter, etc.). The shuffle
	// session follows the start by itself (see the `sync` subscriptions below).
	const trackController = {
		...rawTrackController,
		play(track: Track) {
			navigation++
			libraryQueue = { location: libraryLocation(), snapshot: get(displayedTracks) }
			rawTrackController.play(track)
		},
	}

	const deviceController = createDeviceController(
		{ devicesStore, settingsStore, toastStore },
		{
			openDeviceInfoModal: (device) => getModalOrchestrator()?.openDeviceInfoModal(device),
			openReformatDeviceModal: (device) => getModalOrchestrator()?.openReformatDeviceModal(device),
		}
	)

	const exportController = createExportController(
		{
			exportStore,
			toastStore,
			getDevices,
			getPlaylists,
		},
		{
			openExportToDeviceModal: (device) => getModalOrchestrator()?.openExportToDeviceModal(device),
			openExportPlaylistModal: (playlist) => getModalOrchestrator()?.openExportPlaylistModal(playlist),
			openQuickExportModal: () => getModalOrchestrator()?.openQuickExportModal(),
			openExportFailureModal: (error, deviceId, mountPoint, filesCopied) =>
				getModalOrchestrator()?.openExportFailureModal(error, deviceId, mountPoint, filesCopied),
		}
	)

	const playlistController = createPlaylistController(
		{
			playlistsStore,
			discoveryStore,
			libraryStore,
			uiStore,
			toastStore,
			getPlaylists,
			getSelectedPlaylistId,
			getSelectedFolderId,
			getSelectedTagIds,
			getTagFilterMode: () => get(tagFilterMode),
			getActiveView: () => get(activeView),
			onDiscoveryPlaylistSelected: async (playlistId) => {
				const playlist = getPlaylists().find((p) => p.id === playlistId)
				const releases = playlist?.is_smart
					? await playlistsApi.getSmartPlaylistReleases(playlistId)
					: await playlistsStore.getPlaylistReleases(playlistId)
				discoveryPlaylistStore.cacheAndSet(playlistId, releases)
			},
		},
		{
			openCreatePlaylistModal: (parentId) => getModalOrchestrator()?.openCreatePlaylistModal(parentId),
			openCreateFolderModal: (parentId) => getModalOrchestrator()?.openCreateFolderModal(parentId),
			openCreateSmartPlaylistModal: (parentId, context) =>
				getModalOrchestrator()?.openCreateSmartPlaylistModal(parentId, context),
			openEditSmartPlaylistModal: (playlist) => getModalOrchestrator()?.openEditSmartPlaylistModal(playlist),
			openRenamePlaylistModal: (playlist) => getModalOrchestrator()?.openRenamePlaylistModal(playlist),
			openDeletePlaylistModal: (playlist, hasChildren) =>
				getModalOrchestrator()?.openDeletePlaylistModal(playlist, hasChildren),
			openMoveConflictModal: (playlist, conflict, targetId) =>
				getModalOrchestrator()?.openMoveConflictModal(playlist, conflict, targetId),
		}
	)

	// =========================================================================
	// Playback Queue
	// =========================================================================
	// Tracks the playback context so continuous playback, next/previous use the
	// correct track list even when the user navigates to a different view.
	// While the user is still where playback started, the live list is used
	// (so a filter, a sort change, track adds/removes are reflected immediately,
	// on purpose); once they navigate away, the list as they left it. The rule
	// is `resolveQueue` in the pure `playbackQueue` module.

	// Library track queue and discovery release queue (`null` until playback starts from a list)
	let libraryQueue: QueueContext<Track> | null = null
	let discoveryQueue: QueueContext<DiscoveryRelease> | null = null

	// Bumped by every navigation and every change of what is loaded in the player: a skip chain (a next track that
	// fails to load, CRA-180) stops retrying as soon as the user has moved on or something else started.
	let navigation = 0

	function libraryLocation(): QueueLocation {
		return { view: get(activeView), playlistId: get(libraryStore).selectedPlaylistId }
	}

	function discoveryLocation(): QueueLocation {
		const ui = get(uiStore)
		return { view: ui.activeView, playlistId: ui.selectedPlaylistId ?? null }
	}

	// Shuffle playback bookkeeping (no repeat until exhausted, history for "previous"),
	// kept in the pure `playbackQueue` module. Both library and discovery shuffle
	// operate at the individual track level.
	const libraryShuffle = createShuffleSession<Track>((t) => t.id)
	const discoveryShuffle = createShuffleSession<{ release: DiscoveryRelease; trackIndex: number }>(previewTargetKey)

	// Re-anchor the shuffle session on the current track whenever shuffle is switched on.
	shuffleEnabled.subscribe((on) => {
		if (!on) return
		libraryShuffle.reset(get(currentTrack)?.id ?? null)
		const preview = get(previewInfo)
		discoveryShuffle.reset(preview ? discoveryTrackKey(preview.releaseId, preview.trackIndex) : null)
	})

	// Every playback start, whatever started it — a row, the Suggested panel, Duplicate Killer, the Upgrader, the
	// track restored at launch, or the queue itself — is reported to the shuffle session, which keeps its history
	// for its own picks and starts afresh on any other track (CRA-181). Derived on the key so the 100 ms position
	// ticks, which emit a new track object, do not reach the session.
	derived(currentTrack, (track) => track?.id ?? null).subscribe((id) => {
		navigation++
		libraryShuffle.sync(id)
	})
	derived(previewInfo, (preview) =>
		preview ? discoveryTrackKey(preview.releaseId, preview.trackIndex) : null
	).subscribe((key) => {
		navigation++
		discoveryShuffle.sync(key)
	})

	// Keep the frozen queue snapshot up-to-date while the context view is active.
	// When the user navigates away, the snapshot freezes at the last known state.
	displayedTracks.subscribe((tracks) => {
		libraryQueue = refreshQueueSnapshot(libraryQueue, libraryLocation(), tracks)
	})

	displayedReleases.subscribe((releases) => {
		discoveryQueue = refreshQueueSnapshot(discoveryQueue, discoveryLocation(), releases)
	})

	/** Get the current library track queue, using live data when the view matches. */
	function getLibraryQueue(): readonly Track[] {
		return resolveQueue(libraryQueue, libraryLocation(), get(displayedTracks))
	}

	/** Get the current discovery release queue, using live data when the view matches. */
	function getDiscoveryQueue(): readonly DiscoveryRelease[] {
		return resolveQueue(discoveryQueue, discoveryLocation(), get(displayedReleases))
	}

	/**
	 * Play a discovery preview, capturing the release queue context.
	 * Use this instead of playerStore.playPreview() for user-initiated preview playback.
	 */
	function playPreview(release: DiscoveryRelease, trackIndex: number) {
		navigation++
		const ui = get(uiStore)
		// A preview started from outside the Discovery view walks the whole Discovery list.
		const playlistId = ui.activeView === 'discovery' ? (ui.selectedPlaylistId ?? null) : null
		discoveryQueue = { location: { view: 'discovery', playlistId }, snapshot: get(displayedReleases) }
		playerStore.playPreview(release, trackIndex)
	}

	// =========================================================================
	// Track Navigation
	// =========================================================================

	const PREVIEWABLE_SOURCES: Set<DiscoverySourceType> = new Set(['bandcamp', 'soundcloud', 'youtube'])

	function trackCanPlay(release: DiscoveryRelease, trackIndex: number): boolean {
		const track = release.tracks[trackIndex]
		if (!track?.duration_ms) return false
		if (release.source_type === 'discogs') return track.video_id !== null
		return PREVIEWABLE_SOURCES.has(release.source_type) || release.tracks.some((t) => t.video_id !== null)
	}

	function findPreviewableTrackIndex(release: DiscoveryRelease, direction: 'first' | 'last'): number {
		return findPlayableTrackIndex(release, direction, trackCanPlay)
	}

	/**
	 * Next / previous in the library queue: shuffle or sequential, skipping tracks whose file cannot be loaded and
	 * telling the user once (CRA-180), restarting the current track when "previous" has nowhere to go (CRA-181).
	 */
	async function navigateLibrary(direction: QueueDirection) {
		const generation = navigation
		let tracks = getLibraryQueue()
		if (tracks.length === 0) {
			tracks = get(sortedTracks)
		}
		if (tracks.length === 0) return

		const outcome = await navigateQueue(direction, {
			items: tracks,
			currentKey: get(currentTrack)?.id ?? null,
			keyOf: (track) => track.id,
			shuffle: get(shuffleEnabled) ? libraryShuffle : null,
			excludeCurrent: true,
			start: (track) => playerStore.play(track),
			restartCurrent: () => playerStore.restartTrack(),
			isCancelled: () => generation !== navigation,
		})
		reportSkippedTracks(outcome, (track) => track.title)
	}

	function playNextTrack() {
		navigation++
		const source = get(playbackSource)

		// 1. Discovery / Preview playback
		if (source === 'preview' || get(previewInfo)) {
			const preview = get(previewInfo)
			if (preview) {
				if (get(shuffleEnabled)) {
					const candidates = buildDiscoveryCandidates(getDiscoveryQueue(), trackCanPlay)
					const currentKey = discoveryTrackKey(preview.releaseId, preview.trackIndex)
					// A one-track queue loops on its track, as it does with shuffle off (CRA-181).
					const pick =
						discoveryShuffle.next(candidates, currentKey) ?? soleCurrentItem(candidates, currentKey, previewTargetKey)
					if (pick) playerStore.playPreview(pick.release, pick.trackIndex)
					return
				}

				// Non-shuffle: next track in release, then next release
				const target = nextPreviewTarget(preview, getDiscoveryQueue(), trackCanPlay)
				if (target) playerStore.playPreview(target.release, target.trackIndex)
				return
			}
		}

		// 2. Beatport playback
		if (source === 'beatport') {
			const bpState = get(beatportStore)
			const tracks = bpState.searchQuery.trim() ? bpState.searchResults : bpState.currentSectionTracks
			if (tracks.length > 0) {
				const current = get(beatportTrack)
				const idx = tracks.findIndex((t) => String(t.id) === String(current?.id))
				const nextIdx = sequentialNextIndex(idx, tracks.length)
				playerStore.playBeatport(tracks[nextIdx])
				return
			}
		}

		// 3. Standalone files & Player Albums
		if (source === 'standalone' || (!get(currentTrack) && get(standaloneTrack))) {
			const albState = get(albumsStore)
			if (albState.isPlayingAlbum && albState.selectedAlbum && albState.selectedAlbumTracks.length > 0) {
				albumsStore.playNextAlbumTrack()
				return
			}

			const recents = get(recentStandaloneTracks)
			if (recents.length > 0) {
				const current = get(standaloneTrack)
				const idx = recents.findIndex((t) => t.file_path === current?.file_path || t.id === current?.id)
				const nextIdx = sequentialNextIndex(idx, recents.length)
				playerStore.playStandalone(recents[nextIdx], recents[nextIdx].is_in_library)
				return
			}
		}

		// 4. Library playback
		navigateLibrary('next').catch((err) => console.warn('Next track failed:', err))
	}

	function playPreviousTrack() {
		navigation++
		const source = get(playbackSource)

		// 1. Discovery / Preview playback
		if (source === 'preview' || get(previewInfo)) {
			const preview = get(previewInfo)
			if (preview) {
				if (get(shuffleEnabled)) {
					const candidates = buildDiscoveryCandidates(getDiscoveryQueue(), trackCanPlay)
					const currentKey = discoveryTrackKey(preview.releaseId, preview.trackIndex)
					const prev =
						discoveryShuffle.previous(candidates) ?? soleCurrentItem(candidates, currentKey, previewTargetKey)
					if (prev) {
						playerStore.playPreview(prev.release, prev.trackIndex)
					} else {
						// Nothing to go back to: restart the current preview (CRA-181).
						playerStore.restartTrack()
					}
					return
				}

				// Non-shuffle: previous track in release, then previous release
				const target = previousPreviewTarget(preview, getDiscoveryQueue(), trackCanPlay)
				if (target) playerStore.playPreview(target.release, target.trackIndex)
				return
			}
		}

		// 2. Beatport playback
		if (source === 'beatport') {
			const bpState = get(beatportStore)
			const tracks = bpState.searchQuery.trim() ? bpState.searchResults : bpState.currentSectionTracks
			if (tracks.length > 0) {
				const current = get(beatportTrack)
				const idx = tracks.findIndex((t) => String(t.id) === String(current?.id))
				const prevIdx = sequentialPreviousIndex(idx, tracks.length)
				playerStore.playBeatport(tracks[prevIdx])
				return
			}
		}

		// 3. Standalone files & Player Albums
		if (source === 'standalone' || (!get(currentTrack) && get(standaloneTrack))) {
			const albState = get(albumsStore)
			if (albState.isPlayingAlbum && albState.selectedAlbum && albState.selectedAlbumTracks.length > 0) {
				albumsStore.playPreviousAlbumTrack()
				return
			}

			const recents = get(recentStandaloneTracks)
			if (recents.length > 0) {
				const current = get(standaloneTrack)
				const idx = recents.findIndex((t) => t.file_path === current?.file_path || t.id === current?.id)
				const prevIdx = sequentialPreviousIndex(idx, recents.length)
				playerStore.playStandalone(recents[prevIdx], recents[prevIdx].is_in_library)
				return
			}
		}

		// 4. Library playback (shuffle walks back through the actual play history)
		navigateLibrary('previous').catch((err) => console.warn('Previous track failed:', err))
	}

	// =========================================================================
	// Shared Handlers (used by both keyboard shortcuts and menu actions)
	// =========================================================================

	const handlers = {
		playPause: () => {
			if (get(activeView) === 'player') {
				const active = get(currentTrack) || get(standaloneTrack)
				if (active) {
					playerStore.togglePlayPause()
					return
				}
				const recents = get(recentStandaloneTracks)
				if (recents.length > 0) {
					playerStore.playStandalone(recents[0], recents[0].is_in_library)
					return
				}
			}

			const state = get(currentTrack)
			const standalone = get(standaloneTrack)
			const preview = get(previewInfo)

			// If a track or preview is loaded, toggle normally
			if (state || standalone || preview) {
				playerStore.togglePlayPause()
				return
			}

			// Nothing loaded — play first item in current view
			if (get(activeView) === 'discovery') {
				const releases = get(displayedReleases)
				for (const release of releases) {
					const trackIdx = findPreviewableTrackIndex(release, 'first')
					if (trackIdx !== -1) {
						playPreview(release, trackIdx)
						return
					}
				}
			} else {
				const tracks = get(displayedTracks)
				if (tracks.length > 0) {
					trackController.play(tracks[0])
				}
			}
		},
		stop: () => playerStore.stop(),
		seekForward: () => playerStore.seekRelative(10000),
		seekBackward: () => playerStore.seekRelative(-10000),
		fineSeekForward: () => playerStore.seekRelative(1000),
		fineSeekBackward: () => playerStore.seekRelative(-1000),
		volumeUp: () => playerStore.adjustVolume(0.1),
		volumeDown: () => playerStore.adjustVolume(-0.1),
		toggleMute: () => playerStore.toggleMute(),

		selectAll: () => {
			if (get(activeView) === 'discovery') {
				uiStore.setSelectedReleases(new Set(get(displayedReleases).map((r) => r.id)))
			} else {
				uiStore.setSelectedTracks(new Set(get(sortedTracks).map((t) => t.id)))
			}
		},

		openSettings: (tab?: SettingsPage) => getModalOrchestrator()?.openSettingsModal(tab),

		quickExport: () => {
			if (getDevices().length > 0) getModalOrchestrator()?.openQuickExportModal()
		},

		jumpToPlayingTrack: () => {
			const track = get(currentTrack)
			if (!track) return
			if (getSelectedPlaylistId()) playlistController.handleLibraryClick()
			uiStore.selectTrack(track.id)
		},

		toggleView: () => {
			if (getModalOrchestrator()?.isModalOpen()) return
			const next = get(activeView) === 'library' ? 'discovery' : 'library'
			handleViewChange(next)
		},

		import: async () => {
			if (get(activeView) !== 'library') {
				handleViewChange('library')
				await tick()
			}
			trackController.handleImport()
		},

		addRelease: async () => {
			if (get(activeView) !== 'discovery') {
				handleViewChange('discovery')
				await tick()
			}
			setShowAddReleaseModal()
		},

		refreshMetadata: async () => {
			if (get(activeView) !== 'discovery') return
			const ids = [...get(selectedReleaseIds)]
			if (ids.length === 0) return
			await Promise.all(ids.map((id) => discoveryStore.refreshMetadata(id)))
		},
	}

	// =========================================================================
	// Mount Setup
	// =========================================================================

	let tornDown = false

	/**
	 * Waits at most `ms` for a setup step that returns a cleanup function, without ever losing that
	 * cleanup: if the step finishes after the timeout, its cleanup still runs at teardown (or right
	 * away if teardown already happened). Failures are logged with the step name.
	 */
	function cleanupWhenReady(promise: Promise<() => void>, ms: number, label: string): Promise<() => void> {
		let cleanupFn: (() => void) | null = null
		const cleanup = () => {
			cleanupFn?.()
			cleanupFn = null
		}
		const settled = promise
			.then((fn) => {
				if (tornDown) fn()
				else cleanupFn = fn
			})
			.catch((err) => console.warn(`${label} failed:`, err))
		return Promise.race([
			settled.then(() => cleanup),
			new Promise<() => void>((resolve) =>
				setTimeout(() => {
					if (!cleanupFn) console.warn(`${label} still initializing after ${ms} ms; continuing`)
					resolve(cleanup)
				}, ms)
			),
		])
	}

	function withTimeout<T>(promise: Promise<T>, ms: number, fallback: T): Promise<T> {
		let timer: ReturnType<typeof setTimeout>
		const timeoutPromise = new Promise<T>((resolve) => {
			timer = setTimeout(() => {
				console.warn(`Setup step still running after ${ms} ms; continuing`)
				resolve(fallback)
			}, ms)
		})
		return Promise.race([
			promise
				.then((res) => {
					clearTimeout(timer)
					return res
				})
				.catch((err) => {
					clearTimeout(timer)
					console.warn('Timed promise caught error:', err)
					return fallback
				}),
			timeoutPromise,
		])
	}

	async function onMountSetup(): Promise<() => void> {
		let cleanupApp: () => void = () => {}
		let cleanupKeyboard: () => void = () => {}
		let cleanupMenu: () => void = () => {}
		let cleanupMediaKeys: () => void = () => {}
		// First, outside the try below: the updater must know when Crate is busy (gig safety) even if a
		// later start-up step fails. The first automatic check still waits 30 seconds.
		const stopUpdaterSchedule = startUpdaterSchedule()

		try {
			await withTimeout(exportStore.startListening(), 1500, undefined)

			cleanupApp = await cleanupWhenReady(
				useAppInitialization({
					stores: {
						appStore,
						libraryStore,
						tagsStore,
						playlistsStore,
						settingsStore,
						devicesStore,
						syncStore,
						discoveryStore,
					},
					toastStore,
					onExternalFileDrop: trackController.handleExternalFileDrop,
					onDragStateChange: (dragOver) => setIsDragOver(dragOver),
				}),
				2500,
				'App initialization'
			)

			// Wire shared stores to their desktop-only collaborators
			playerStore.setTrackMissingHandler((id) => missingTracksStore.markMissing(id))
			playlistsStore.setPlaylistsChangedHandler((ids) => syncStore.notifyPlaylistChanges(ids))

			// Restore last-playing track/preview from localStorage now that stores are loaded
			try {
				playerStore.restoreTrack(get(libraryStore).tracks)
				await withTimeout(playerStore.restorePreview(), 1500, undefined)
			} catch (err) {
				console.warn('Failed to restore track/preview:', err)
			}

			// Restore persisted navigation state (playlist/folder selection)
			try {
				const restoredState = get(uiStore)
				if (restoredState.selectedPlaylistId) {
					const playlist = getPlaylists().find((p) => p.id === restoredState.selectedPlaylistId)
					if (playlist) {
						await playlistController.handlePlaylistSelect(playlist)
					} else {
						uiStore.selectPlaylist(null)
					}
				} else if (restoredState.selectedFolderId) {
					const folder = getPlaylists().find((p) => p.id === restoredState.selectedFolderId)
					if (!folder) {
						uiStore.selectFolder(null)
					}
				}
			} catch (err) {
				console.warn('Failed to restore navigation state:', err)
			}

			cleanupKeyboard = useKeyboardShortcuts({
				isModalOpen: () => getModalOrchestrator()?.isModalOpen() ?? false,
				onPlayPause: handlers.playPause,
				onFocusSearch: () => {
					const searchInput = document.querySelector('input[type="search"]') as HTMLInputElement
					searchInput?.focus()
				},
				onClearSelection: () => uiStore.clearSelection(),
				onSelectAll: handlers.selectAll,
				onOpenSettings: handlers.openSettings,
				onNewPlaylist: () => playlistController.handleCreatePlaylist(),
				onNewFolder: () => playlistController.handleCreateFolder(),
				onImport: handlers.import,
				onDeleteSelected: () => {
					const treeIds = get(uiLayoutStore).selectedTreeIds
					if (treeIds.size > 1) {
						const selected = getPlaylists().filter((p) => treeIds.has(p.id))
						if (selected.length > 0) {
							getModalOrchestrator()?.openDeletePlaylistBulkModal(selected)
						}
						return true
					}

					const playlistId = getSelectedPlaylistId()
					const playlists = getPlaylists()
					const currentPlaylist = playlistId ? playlists.find((p) => p.id === playlistId) : null

					if (get(activeView) === 'discovery') {
						const releaseIds = get(selectedReleaseIds)
						if (releaseIds.size > 0) {
							if (playlistId && !currentPlaylist?.is_smart) {
								getModalOrchestrator()?.openRemoveDiscoveryReleasesFromPlaylistModal(Array.from(releaseIds), playlistId)
							} else {
								getModalOrchestrator()?.openRemoveDiscoveryReleasesModal(Array.from(releaseIds))
							}
							return true
						}
					}

					const ids = [...get(selectedTrackIds)]
					if (ids.length > 0) {
						if (playlistId && !currentPlaylist?.is_smart) {
							getModalOrchestrator()?.openRemoveFromPlaylistModal(ids, playlistId)
						} else {
							getModalOrchestrator()?.openRemoveFromLibraryModal(ids)
						}
					} else if (playlistId) {
						if (currentPlaylist) playlistController.handlePlaylistDelete(currentPlaylist)
					} else {
						const folderId = getSelectedFolderId()
						if (folderId) {
							const folder = playlists.find((p) => p.id === folderId)
							if (folder) playlistController.handlePlaylistDelete(folder)
						}
					}
					return true
				},
				onPlaySelected: () => {
					if (get(activeView) === 'discovery') {
						const releaseIds = get(selectedReleaseIds)
						if (releaseIds.size > 0) {
							const releases = get(displayedReleases)
							expandedReleaseIds.toggleSelection(
								[...releaseIds],
								(id) => (releases.find((r) => r.id === id)?.tracks.length ?? 0) > 0
							)
						}
						return
					}
					const selectedIds = get(selectedTrackIds)
					if (selectedIds.size > 0) {
						const firstSelectedId = [...selectedIds][0]
						const track = get(displayedTracks).find((t) => t.id === firstSelectedId)
						if (track) trackController.play(track)
					}
				},
				onSeekBackward: handlers.seekBackward,
				onSeekForward: handlers.seekForward,
				onFineSeekBackward: handlers.fineSeekBackward,
				onFineSeekForward: handlers.fineSeekForward,
				onPreviousTrack: playPreviousTrack,
				onNextTrack: playNextTrack,
				onVolumeUp: handlers.volumeUp,
				onVolumeDown: handlers.volumeDown,
				onToggleMute: handlers.toggleMute,
				onSelectPreviousTrack: () => {
					if (get(activeView) === 'discovery') {
						const releases = get(displayedReleases)
						if (releases.length === 0) return
						const ids = get(selectedReleaseIds)
						if (ids.size === 0) {
							uiStore.selectRelease(releases[releases.length - 1].id)
						} else {
							const firstId = [...ids][0]
							const idx = releases.findIndex((r) => r.id === firstId)
							if (idx > 0) uiStore.selectRelease(releases[idx - 1].id)
						}
						return
					}
					const tracks = get(displayedTracks)
					if (tracks.length === 0) return
					const ids = get(selectedTrackIds)
					if (ids.size === 0) {
						uiStore.selectTrack(tracks[tracks.length - 1].id)
					} else {
						const firstId = [...ids][0]
						const idx = tracks.findIndex((t) => t.id === firstId)
						if (idx > 0) uiStore.selectTrack(tracks[idx - 1].id)
					}
				},
				onSelectNextTrack: () => {
					if (get(activeView) === 'discovery') {
						const releases = get(displayedReleases)
						if (releases.length === 0) return
						const ids = get(selectedReleaseIds)
						if (ids.size === 0) {
							uiStore.selectRelease(releases[0].id)
						} else {
							const lastId = [...ids].pop()
							const idx = releases.findIndex((r) => r.id === lastId)
							if (idx >= 0 && idx < releases.length - 1) uiStore.selectRelease(releases[idx + 1].id)
						}
						return
					}
					const tracks = get(displayedTracks)
					if (tracks.length === 0) return
					const ids = get(selectedTrackIds)
					if (ids.size === 0) {
						uiStore.selectTrack(tracks[0].id)
					} else {
						const lastId = [...ids].pop()
						const idx = tracks.findIndex((t) => t.id === lastId)
						if (idx >= 0 && idx < tracks.length - 1) uiStore.selectTrack(tracks[idx + 1].id)
					}
				},
				onQuickExport: handlers.quickExport,
				onJumpToPlayingTrack: handlers.jumpToPlayingTrack,
				onToggleView: handlers.toggleView,
				onAddRelease: handlers.addRelease,
				onRefreshMetadata: handlers.refreshMetadata,
			})

			cleanupMenu = await cleanupWhenReady(
				useMenuActions({
					onImport: handlers.import,
					onAddRelease: handlers.addRelease,
					onCreatePlaylist: playlistController.handleCreatePlaylist,
					onCreateFolder: playlistController.handleCreateFolder,
					onSelectAll: handlers.selectAll,
					onPlayPause: handlers.playPause,
					onStop: handlers.stop,
					onNextTrack: playNextTrack,
					onPreviousTrack: playPreviousTrack,
					onSeekForward: handlers.seekForward,
					onSeekBackward: handlers.seekBackward,
					onFineSeekForward: handlers.fineSeekForward,
					onFineSeekBackward: handlers.fineSeekBackward,
					onVolumeUp: handlers.volumeUp,
					onVolumeDown: handlers.volumeDown,
					onToggleMute: handlers.toggleMute,
					onOpenSettings: handlers.openSettings,
					onQuickExport: handlers.quickExport,
					onJumpToPlayingTrack: handlers.jumpToPlayingTrack,
					onToggleView: handlers.toggleView,
					onToggleEditor: () => uiLayoutStore.toggleRightSidebar(),
					onExpandAllReleases: () => {
						const releases = get(displayedReleases)
						expandedReleaseIds.expandAll(releases.filter((r) => r.tracks.length > 0).map((r) => r.id))
					},
					onCollapseAllReleases: () => expandedReleaseIds.collapseAll(),
					onRefreshMetadata: handlers.refreshMetadata,
				}),
				1500,
				'Menu actions'
			)

			cleanupMediaKeys = await cleanupWhenReady(
				useMediaKeys({
					onPlayPause: handlers.playPause,
					onNextTrack: playNextTrack,
					onPreviousTrack: playPreviousTrack,
				}),
				1500,
				'Media keys'
			)

			if ('mediaSession' in navigator) {
				navigator.mediaSession.setActionHandler('nexttrack', playNextTrack)
				navigator.mediaSession.setActionHandler('previoustrack', playPreviousTrack)
			}

			playerStore.onTrackEnd(() => {
				if (get(continuousPlayback)) {
					playNextTrack()
				}
			})

			if (get(activeView) === 'discovery') {
				discoveryStore.loadReleases().catch(() => {})
			}

			cloudSyncStore.load().catch(() => {})
			cloudSyncStore.startPolling()
			cloudSyncStore.startOverrideListener()
		} catch (err) {
			console.warn('Initialization error:', err)
		} finally {
			// Instant splash dismissal - no artificial delay
			dismissSplash()

			// Background preloading - non-blocking
			setTimeout(() => {
				recentTracksStore.load().catch(() => {})
				beatportStore.loadInitialData().catch(() => {})
			}, 100)
		}

		return () => {
			tornDown = true
			cleanupApp()
			cleanupKeyboard()
			cleanupMenu()
			cleanupMediaKeys()
			if ('mediaSession' in navigator) {
				navigator.mediaSession.setActionHandler('nexttrack', null)
				navigator.mediaSession.setActionHandler('previoustrack', null)
			}
			playerStore.onTrackEnd(null)
			exportStore.stopListening()
			cloudSyncStore.stopPolling()
			cloudSyncStore.stopOverrideListener()
			stopUpdaterSchedule()
		}
	}

	// =========================================================================
	// Drag-Drop Setup
	// =========================================================================

	function setupDragDrop(): () => void {
		return useDragDropCoordination({
			getPlaylists,
			getDevices,
			onTracksDropOnPlaylist: trackController.handleTracksDropOnPlaylist,
			onReleasesDropOnPlaylist: async (playlistId: string, releaseIds: string[]) => {
				await playlistsStore.addReleases(playlistId, releaseIds)
			},
			onPlaylistMove: playlistController.handlePlaylistDragMove,
			onBulkPlaylistMove: playlistController.handleBulkPlaylistMove,
			onPlaylistExportToDevice: exportController.handlePlaylistDropOnDevice,
			onTagDropOnTrack: async (tagId: string, trackId: string) => {
				const trackIds = get(selectedTrackIds).has(trackId) ? Array.from(get(selectedTrackIds)) : [trackId]
				await tagsStore.assignTags(trackIds, [tagId])
				const playlistId = getSelectedPlaylistId()
				if (playlistId) {
					await libraryStore.loadPlaylistTracks(playlistId)
				} else {
					await libraryStore.loadTracks()
				}
			},
			onTagDropOnRelease: async (tagId: string, releaseId: string) => {
				const releaseIds = get(selectedReleaseIds).has(releaseId) ? Array.from(get(selectedReleaseIds)) : [releaseId]
				await discoveryStore.assignTags(releaseIds, [tagId])
			},
			onTagDropOnCategory: async (tagId: string, _sourceCategoryId: string, targetCategoryId: string) => {
				try {
					await tagsStore.moveTag(tagId, targetCategoryId)
					libraryStore.updateTagCategory(tagId, targetCategoryId)
					discoveryStore.updateTagCategory(tagId, targetCategoryId)
					discoveryPlaylistStore.updateTagCategory(tagId, targetCategoryId)
				} catch (error) {
					const message = toErrorMessage(error, get(translate)('errors.tagNameConflict'))
					toastStore.error(message)
				}
			},
		})
	}

	return {
		tagController,
		trackController,
		deviceController,
		exportController,
		playlistController,
		playPreview,
		playNextTrack,
		playPreviousTrack,
		onMountSetup,
		setupDragDrop,
	}
}
