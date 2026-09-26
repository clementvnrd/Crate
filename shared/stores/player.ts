import { writable, derived, get } from 'svelte/store'
import type { Track, PlaybackState, PreviewInfo, DiscoveryRelease, StandaloneTrack, Cue } from '../types'
import type { BeatportTrack } from '../types/beatport'
import * as playerApi from '../api/player'
import * as discoveryApi from '../api/discovery'
import * as standaloneApi from '../api/standalone'
import * as libraryApi from '../api/library'
import * as previewPlayer from '../services/previewPlayer'
import { toastStore } from './toast'
import { hotCueForSlot } from '../utils/cues'
import { translate } from '../i18n'
import {
	getStoredNumber,
	setStoredNumber,
	getStoredString,
	setStoredString,
	getStoredBoolean,
	setStoredBoolean,
} from '../utils/storage'
import { toErrorMessage } from '../utils/errors'

// =============================================================================
// State
// =============================================================================

type PlaybackSource = 'library' | 'preview' | 'beatport' | 'standalone'

interface PlayerState {
	currentTrack: Track | null
	playbackState: PlaybackState
	error: string | null
	isMuted: boolean
	volumeBeforeMute: number
	shuffleEnabled: boolean
	playbackSource: PlaybackSource
	previewInfo: PreviewInfo | null
	previewTrackIndex: number
	previewLoadingReleaseId: string | null
	beatportTrack: BeatportTrack | null
	standaloneTrack: StandaloneTrack | null
	currentCues: Cue[]
	waveformBars: number[]
}

const initialPlaybackState: PlaybackState = {
	is_playing: false,
	position_ms: getStoredNumber('player.positionMs', 0),
	duration_ms: getStoredNumber('player.durationMs', 0),
	volume: getStoredNumber('player.volume', 1.0),
	speed: getStoredNumber('player.speed', 1.0),
	current_track_id: null,
	current_track_path: null,
}

const restoredTrackId = getStoredString('player.trackId', '')
const restoredPlaybackSource = getStoredString<PlaybackSource>('player.playbackSource', 'library', [
	'library',
	'preview',
	'beatport',
	'standalone',
])
const restoredPreviewReleaseId = getStoredString('player.previewReleaseId', '')
const restoredPreviewTrackIndex = getStoredNumber('player.previewTrackIndex', 0)

const initialState: PlayerState = {
	currentTrack: null,
	playbackState: initialPlaybackState,
	error: null,
	isMuted: getStoredBoolean('player.isMuted', false),
	volumeBeforeMute: getStoredNumber('player.volumeBeforeMute', 1.0),
	shuffleEnabled: getStoredBoolean('player.shuffleEnabled', false),
	playbackSource: 'library',
	previewInfo: null,
	previewTrackIndex: 0,
	previewLoadingReleaseId: null,
	beatportTrack: null,
	standaloneTrack: null,
	currentCues: [],
	waveformBars: [],
}

// =============================================================================
// Store
// =============================================================================

function createPlayerStore() {
	const { subscribe, set, update } = writable<PlayerState>(initialState)

	let positionInterval: ReturnType<typeof setInterval> | null = null
	let onTrackEndCallback: (() => void) | null = null
	// Desktop registers this to flag a track as missing when playback fails with a
	// file-not-found error. Injected so this shared store needs no dependency on the
	// desktop-only missingTracks store.
	let onTrackMissing: ((trackId: string) => void) | null = null
	let previewRetryAttempted = false
	let previewRetrying = false
	let previewSpeedCommitTimeout: ReturnType<typeof setTimeout> | null = null
	let isRestoredFromStorage = false
	let lastPositionWriteTime = 0

	function persistPosition(positionMs: number) {
		const now = Date.now()
		if (now - lastPositionWriteTime >= 1000) {
			setStoredNumber('player.positionMs', positionMs)
			lastPositionWriteTime = now
		}
	}

	function persistPositionImmediate(positionMs: number) {
		setStoredNumber('player.positionMs', positionMs)
		lastPositionWriteTime = Date.now()
	}

	function getState(): PlayerState {
		let state: PlayerState = initialState
		const unsub = subscribe((s) => (state = s))
		unsub()
		return state
	}

	let trackingTicks = 0
	function startPositionTracking() {
		stopPositionTracking()
		trackingTicks = 0
		positionInterval = setInterval(async () => {
			trackingTicks++
			// Every ~1 second, sync with true backend audio hardware position
			if (trackingTicks % 10 === 0) {
				const s = getState()
				if (s.playbackState.is_playing && (s.playbackSource === 'library' || s.playbackSource === 'standalone')) {
					try {
						const backendState = await playerApi.getPlaybackState()
						if (backendState && typeof backendState.position_ms === 'number') {
							update((prev) => ({
								...prev,
								playbackState: {
									...prev.playbackState,
									position_ms: backendState.position_ms,
									is_playing: backendState.is_playing,
								},
							}))
							persistPosition(backendState.position_ms)
							return
						}
					} catch {
						// Fallback to client interpolation
					}
				}
			}

			update((state) => {
				if (state.playbackState.is_playing) {
					const speed = state.playbackState.speed ?? 1.0
					const newPosition = Math.min(
						state.playbackState.position_ms + Math.round(100 * speed),
						state.playbackState.duration_ms
					)
					if (newPosition >= state.playbackState.duration_ms && state.playbackState.duration_ms > 0) {
						// Track ended — defer callback to avoid store update conflicts
						setTimeout(() => {
							stopPositionTracking()
							onTrackEndCallback?.()
						}, 0)
						return {
							...state,
							playbackState: {
								...state.playbackState,
								position_ms: state.playbackState.duration_ms,
								is_playing: false,
							},
						}
					}
					if (state.playbackSource === 'library') {
						persistPosition(newPosition)
					}
					return {
						...state,
						playbackState: { ...state.playbackState, position_ms: newPosition },
					}
				}
				return state
			})
		}, 100)
	}

	async function loadTrackCuesAndWaveform(trackId: string) {
		try {
			const [cues, waveform] = await Promise.all([
				libraryApi.getTrackCues(trackId).catch(() => [] as Cue[]),
				libraryApi.getTrackWaveform(trackId).catch(() => null),
			])

			let computedBars: number[] = []
			if (waveform && waveform.length > 0) {
				const numBars = 64
				const step = Math.max(1, Math.floor(waveform.length / numBars))
				for (let i = 0; i < numBars; i++) {
					let maxVal = 0
					const startIdx = i * step
					const endIdx = Math.min(startIdx + step, waveform.length)
					for (let j = startIdx; j < endIdx; j++) {
						if (waveform[j] > maxVal) maxVal = waveform[j]
					}
					// Backend peaks are already 0–100; keep a thin minimum so silence stays visible
					computedBars.push(Math.max(4, Math.min(100, maxVal)))
				}
			}

			update((s) => {
				const matches =
					s.currentTrack?.id === trackId ||
					s.standaloneTrack?.id === trackId ||
					s.currentTrack?.file_path === trackId ||
					s.standaloneTrack?.file_path === trackId
				if (!matches) return s
				return {
					...s,
					currentCues: cues || [],
					// No waveform (undecodable file): clear the previous track's bars instead of keeping them
					waveformBars: computedBars,
					currentTrack: s.currentTrack ? { ...s.currentTrack, waveform_data: waveform } : null,
				}
			})
		} catch (e) {
			console.warn('Could not load track cues or waveform:', e)
		}
	}

	function stopPositionTracking() {
		if (positionInterval) {
			clearInterval(positionInterval)
			positionInterval = null
		}
	}

	function stopPreviewInternal() {
		previewPlayer.stop()
		stopPositionTracking()
	}

	/**
	 * Stops the Rust audio engine before a preview starts. Library tracks *and* standalone files
	 * both play through it, so checking only the 'library' source let a standalone file keep
	 * playing under the HTML preview.
	 */
	async function stopNativeEngine(state: PlayerState) {
		if (state.playbackSource === 'library' || state.playbackSource === 'standalone') {
			try {
				await playerApi.stop()
			} catch {
				// Best effort
			}
		}
	}

	function wirePreviewEvents() {
		previewPlayer.setOnTimeUpdate((positionMs: number) => {
			update((state) => {
				const { duration_ms } = state.playbackState
				// When the Audio element reports a position past the metadata duration,
				// the stream container is longer than the actual audio (e.g. proxied
				// YouTube/Discogs ~2x duration). Stop playback and trigger track end.
				if (duration_ms > 0 && positionMs >= duration_ms) {
					setTimeout(() => {
						stopPreviewInternal()
						onTrackEndCallback?.()
					}, 0)
					return {
						...state,
						playbackState: { ...state.playbackState, position_ms: duration_ms, is_playing: false },
					}
				}
				return {
					...state,
					playbackState: { ...state.playbackState, position_ms: positionMs },
				}
			})
		})
		previewPlayer.setOnDurationChange((durationMs: number) => {
			update((state) => {
				// For Beatport streams, ALWAYS accept the exact audio preview duration (e.g. 30s, 1m30, 2min)
				if (state.playbackSource === 'beatport') {
					return {
						...state,
						playbackState: { ...state.playbackState, duration_ms: durationMs },
					}
				}

				const metadataDuration = state.playbackState.duration_ms
				// When we have a metadata duration from the API, only accept the Audio
				// element's duration if it's within 10% of the known value. Proxied
				// YouTube/Discogs streams can report ~2x the real duration due to
				// container quirks; rejecting those prevents overwriting the correct value.
				if (metadataDuration > 0) {
					const ratio = durationMs / metadataDuration
					if (ratio < 0.9 || ratio > 1.1) {
						return state
					}
				}
				return {
					...state,
					playbackState: { ...state.playbackState, duration_ms: durationMs },
				}
			})
		})
		previewPlayer.setOnEnded(() => {
			update((state) => ({
				...state,
				playbackState: { ...state.playbackState, is_playing: false },
			}))
			onTrackEndCallback?.()
		})
		previewPlayer.setOnWaiting(() => {
			const state = getState()
			if (state.playbackSource === 'preview' && state.previewInfo) {
				update((s) => ({ ...s, previewLoadingReleaseId: state.previewInfo!.releaseId }))
			}
		})
		previewPlayer.setOnPlaying(() => {
			if (previewSpeedCommitTimeout) {
				clearTimeout(previewSpeedCommitTimeout)
				previewSpeedCommitTimeout = null
			}
			update((s) => ({ ...s, previewLoadingReleaseId: null }))
		})
		previewPlayer.setOnError(async (msg: string) => {
			// Ignore duplicate error callbacks fired while a retry is in-flight
			// (HTML5 Audio fires both an 'error' event and a play().catch() for one failure)
			if (previewRetrying) return

			const state = getState()
			if (state.playbackSource === 'preview' && state.previewInfo && !previewRetryAttempted) {
				previewRetryAttempted = true
				previewRetrying = true
				const { release, trackIndex } = state.previewInfo
				const track = release.tracks[trackIndex]
				if (track) {
					console.warn(`Preview stream error, retrying: ${msg}`)
					try {
						await discoveryApi.invalidatePreviewStreamCache(release.id)
						const streamUrl = await discoveryApi.fetchPreviewStream(release.id, track.position)
						previewPlayer.play(streamUrl)
						update((s) => ({
							...s,
							error: null,
							playbackState: { ...s.playbackState, is_playing: true, position_ms: 0 },
						}))
						return
					} catch {
						// Retry failed, fall through to show error
					} finally {
						previewRetrying = false
					}
				} else {
					previewRetrying = false
				}
			}
			clearPreviewEvents()
			stopPreviewInternal()
			update((s) => ({
				...s,
				error: msg,
				playbackState: { ...s.playbackState, is_playing: false },
			}))
			toastStore.error(get(translate)('errors.previewStreamFailed'))
		})
	}

	function clearPreviewEvents() {
		previewPlayer.setOnTimeUpdate(null)
		previewPlayer.setOnDurationChange(null)
		previewPlayer.setOnEnded(null)
		previewPlayer.setOnError(null)
		previewPlayer.setOnWaiting(null)
		previewPlayer.setOnPlaying(null)
	}

	return {
		subscribe,

		/**
		 * Play a library track. If preview or beatport is active, stop it first.
		 */
		async play(track: Track) {
			const state = getState()

			// Stop preview / beatport if active
			if (state.playbackSource === 'preview' || state.playbackSource === 'beatport') {
				stopPreviewInternal()
				clearPreviewEvents()
				// Sync speed to backend since preview speed changes are frontend-only
				try {
					await playerApi.setSpeed(state.playbackState.speed)
				} catch {
					// Best effort
				}
			}

			try {
				const playbackState = await playerApi.playTrack(track.id)
				isRestoredFromStorage = false
				setStoredString('player.playbackSource', 'library')
				setStoredString('player.trackId', track.id)
				setStoredString('player.previewReleaseId', '')
				setStoredNumber('player.durationMs', playbackState.duration_ms)
				persistPositionImmediate(0)
				update((s) => ({
					...s,
					currentTrack: track,
					beatportTrack: null,
					standaloneTrack: null,
					playbackState,
					error: null,
					playbackSource: 'library',
					previewInfo: null,
					previewTrackIndex: 0,
					currentCues: [],
				}))
				startPositionTracking()
				loadTrackCuesAndWaveform(track.id)
			} catch (error) {
				const errorMsg = toErrorMessage(error, 'Failed to play track')
				if (errorMsg.toLowerCase().includes('file not found') || errorMsg.toLowerCase().includes('filenotfound')) {
					onTrackMissing?.(track.id)
				}
				update((s) => ({ ...s, error: errorMsg }))
			}
		},

		/**
		 * Play a standalone track (or library track routed through standalone player).
		 */
		async playStandalone(track: StandaloneTrack, isLibraryTrack?: boolean) {
			const state = getState()

			// Stop preview / beatport if active
			if (state.playbackSource === 'preview' || state.playbackSource === 'beatport') {
				stopPreviewInternal()
				clearPreviewEvents()
				try {
					await playerApi.setSpeed(state.playbackState.speed)
				} catch {
					// Best effort
				}
			}

			try {
				const playbackState = await standaloneApi.playStandaloneTrack(track.file_path, track.id, track.duration_ms)
				isRestoredFromStorage = false
				setStoredString('player.playbackSource', 'standalone')
				setStoredString('player.trackId', track.id)
				setStoredString('player.previewReleaseId', '')
				setStoredNumber('player.durationMs', playbackState.duration_ms)
				persistPositionImmediate(0)

				update((s) => ({
					...s,
					currentTrack: isLibraryTrack || track.is_in_library ? (s.currentTrack?.id === track.id ? s.currentTrack : null) : null,
					standaloneTrack: track,
					beatportTrack: null,
					playbackState,
					error: null,
					playbackSource: 'standalone',
					previewInfo: null,
					previewTrackIndex: 0,
					currentCues: [],
				}))
				startPositionTracking()
				// A file outside the library has no track id in the database: look it up by path
				loadTrackCuesAndWaveform(isLibraryTrack || track.is_in_library ? track.id : track.file_path)

				// If not in library, save to recent standalone history
				if (!track.is_in_library && !isLibraryTrack) {
					try {
						await standaloneApi.addRecentStandaloneTrack({
							...track,
							last_played_at: new Date().toISOString(),
						})
					} catch (e) {
						console.error('Failed to add to recent standalone history', e)
					}
				}
			} catch (error) {
				const errorMsg = toErrorMessage(error, 'Failed to play track')
				update((s) => ({ ...s, error: errorMsg }))
			}
		},

		/**
		 * Play a preview of a discovery release track.
		 */
		async playPreview(release: DiscoveryRelease, trackIndex: number = 0) {
			previewRetryAttempted = false
			const state = getState()
			const track = release.tracks[trackIndex]
			if (!track) return

			// Clear stale preview events before the async gap
			clearPreviewEvents()

			// Never two sounds at once: stop the native engine (library or standalone)
			await stopNativeEngine(state)

			stopPositionTracking()
			update((s) => ({ ...s, previewLoadingReleaseId: release.id }))

			try {
				const streamUrl = await discoveryApi.fetchPreviewStream(release.id, track.position)

				wirePreviewEvents()

				// Sync volume and speed for preview player
				previewPlayer.play(streamUrl)
				const currentVolume = state.isMuted ? 0 : state.playbackState.volume
				previewPlayer.setVolume(currentVolume)
				previewPlayer.setPlaybackRate(state.playbackState.speed)

				isRestoredFromStorage = false
				setStoredString('player.playbackSource', 'preview')
				setStoredString('player.previewReleaseId', release.id)
				setStoredNumber('player.previewTrackIndex', trackIndex)
				setStoredNumber('player.durationMs', track.duration_ms || 0)
				persistPositionImmediate(0)
				update((s) => ({
					...s,
					currentTrack: null,
					beatportTrack: null,
					standaloneTrack: null,
					playbackState: {
						...s.playbackState,
						is_playing: true,
						position_ms: 0,
						duration_ms: track.duration_ms || 0,
						current_track_id: null,
						current_track_path: null,
					},
					error: null,
					playbackSource: 'preview',
					previewInfo: { releaseId: release.id, release, trackIndex },
					previewTrackIndex: trackIndex,
					previewLoadingReleaseId: null,
				}))
			} catch (error) {
				const errorMsg = toErrorMessage(error, 'Failed to fetch preview stream')
				update((s) => ({ ...s, error: errorMsg, previewLoadingReleaseId: null }))
				toastStore.error(get(translate)('errors.previewStreamFailed'))
			}
		},

		/**
		 * Play a Beatport streaming preview track.
		 */
		async playBeatport(track: BeatportTrack) {
			const state = getState()

			// If already playing this track, toggle play/pause
			if (state.playbackSource === 'beatport' && String(state.beatportTrack?.id) === String(track.id)) {
				if (state.playbackState.is_playing) {
					await this.pause()
				} else {
					await this.resume()
				}
				return
			}

			// Clear stale preview events
			clearPreviewEvents()

			// Never two sounds at once: stop the native engine (library or standalone)
			await stopNativeEngine(state)

			stopPositionTracking()

			if (!track.preview_url) {
				toastStore.warning('Aucun flux audio de préécoute disponible pour ce titre.')
				return
			}

			try {
				wirePreviewEvents()
				previewPlayer.play(track.preview_url)
				const currentVolume = state.isMuted ? 0 : state.playbackState.volume
				previewPlayer.setVolume(currentVolume)
				previewPlayer.setPlaybackRate(state.playbackState.speed)

				isRestoredFromStorage = false
				setStoredString('player.playbackSource', 'beatport')
				setStoredString('player.trackId', String(track.id))
				setStoredString('player.previewReleaseId', '')
				setStoredNumber('player.durationMs', 120_000)
				persistPositionImmediate(0)

				update((s) => ({
					...s,
					currentTrack: null,
					previewInfo: null,
					beatportTrack: track,
					standaloneTrack: null,
					playbackSource: 'beatport',
					error: null,
					playbackState: {
						...s.playbackState,
						is_playing: true,
						position_ms: 0,
						duration_ms: 120_000,
						current_track_id: String(track.id),
						current_track_path: null,
					},
				}))
			} catch (error) {
				const errorMsg = toErrorMessage(error, 'Erreur lecture Beatport')
				update((s) => ({ ...s, error: errorMsg }))
			}
		},

		/**
		 * Pause playback (source-aware)
		 */
		async pause() {
			const state = getState()

			if (state.playbackSource === 'preview' || state.playbackSource === 'beatport') {
				previewPlayer.pause()
				persistPositionImmediate(state.playbackState.position_ms)
				update((s) => ({
					...s,
					playbackState: { ...s.playbackState, is_playing: false },
					error: null,
				}))
				return
			}

			try {
				const playbackState = await playerApi.pause()
				persistPositionImmediate(playbackState.position_ms)
				update((s) => ({ ...s, playbackState, error: null }))
				stopPositionTracking()
			} catch (error) {
				update((s) => ({
					...s,
					error: toErrorMessage(error, 'Failed to pause'),
				}))
			}
		},

		/**
		 * Resume playback (source-aware)
		 */
		async resume() {
			const state = getState()

			if (state.playbackSource === 'beatport') {
				previewPlayer.setPlaybackRate(state.playbackState.speed)
				previewPlayer.resume()
				update((s) => ({
					...s,
					playbackState: { ...s.playbackState, is_playing: true },
					error: null,
				}))
				return
			}

			if (state.playbackSource === 'preview') {
				// If restored from storage, the audio element has no source — load the stream
				if (isRestoredFromStorage && state.previewInfo) {
					isRestoredFromStorage = false
					const { release, trackIndex } = state.previewInfo
					const track = release.tracks[trackIndex]
					if (!track) return
					const restoredPosition = state.playbackState.position_ms
					try {
						const streamUrl = await discoveryApi.fetchPreviewStream(release.id, track.position)
						wirePreviewEvents()
						previewPlayer.play(streamUrl)
						const currentVolume = state.isMuted ? 0 : state.playbackState.volume
						previewPlayer.setVolume(currentVolume)
						previewPlayer.setPlaybackRate(state.playbackState.speed)
						if (restoredPosition > 0) {
							previewPlayer.seek(restoredPosition)
						}
						update((s) => ({
							...s,
							playbackState: { ...s.playbackState, is_playing: true },
							error: null,
						}))
					} catch {
						update((s) => ({
							...s,
							error: 'Failed to load preview stream',
							playbackState: { ...s.playbackState, is_playing: false },
						}))
					}
					return
				}
				// Sync playback rate in case speed was changed while paused
				previewPlayer.setPlaybackRate(state.playbackState.speed)
				previewPlayer.resume()
				update((s) => ({
					...s,
					playbackState: { ...s.playbackState, is_playing: true },
					error: null,
				}))
				return
			}

			// If restored from storage, the backend has no player loaded — load the track fully
			if (isRestoredFromStorage && state.currentTrack) {
				isRestoredFromStorage = false
				const restoredPosition = state.playbackState.position_ms
				try {
					// Sync volume and speed to backend before playing so create_player uses them
					await playerApi.setVolume(state.isMuted ? 0 : state.playbackState.volume)
					await playerApi.setSpeed(state.playbackState.speed)
					const playbackState = await playerApi.playTrack(state.currentTrack.id)
					// Seek to restored position
					if (restoredPosition > 0) {
						const seekedState = await playerApi.seek(restoredPosition)
						update((s) => ({ ...s, playbackState: seekedState, error: null }))
					} else {
						update((s) => ({ ...s, playbackState, error: null }))
					}
					startPositionTracking()
				} catch (error) {
					const errorMsg = toErrorMessage(error, 'Failed to play track')
					if (errorMsg.toLowerCase().includes('file not found') || errorMsg.toLowerCase().includes('filenotfound')) {
						onTrackMissing?.(state.currentTrack.id)
					}
					update((s) => ({ ...s, error: errorMsg }))
				}
				return
			}

			try {
				const playbackState = await playerApi.resume()
				update((s) => ({ ...s, playbackState, error: null }))
				startPositionTracking()
			} catch (error) {
				update((s) => ({
					...s,
					error: toErrorMessage(error, 'Failed to resume'),
				}))
			}
		},

		/**
		 * Stop playback (source-aware). Preview or Beatport mode resets to library source.
		 */
		async stop() {
			const state = getState()
			previewRetryAttempted = false

			if (state.playbackSource === 'beatport') {
				stopPreviewInternal()
				clearPreviewEvents()
				isRestoredFromStorage = false
				setStoredString('player.playbackSource', 'library')
				setStoredString('player.trackId', '')
				setStoredNumber('player.positionMs', 0)
				setStoredNumber('player.durationMs', 0)
				update((s) => ({
					...s,
					currentTrack: null,
					beatportTrack: null,
					standaloneTrack: null,
					playbackState: { ...initialPlaybackState, volume: s.playbackState.volume, speed: s.playbackState.speed },
					error: null,
					playbackSource: 'library',
				}))
				return
			}

			if (state.playbackSource === 'preview') {
				stopPreviewInternal()
				clearPreviewEvents()
				isRestoredFromStorage = false
				setStoredString('player.playbackSource', 'library')
				setStoredString('player.previewReleaseId', '')
				setStoredNumber('player.positionMs', 0)
				setStoredNumber('player.durationMs', 0)
				update((s) => ({
					...s,
					currentTrack: null,
					beatportTrack: null,
					standaloneTrack: null,
					playbackState: { ...initialPlaybackState, volume: s.playbackState.volume, speed: s.playbackState.speed },
					error: null,
					playbackSource: 'library',
					previewInfo: null,
					previewTrackIndex: 0,
				}))
				return
			}

			try {
				const playbackState = await playerApi.stop()
				isRestoredFromStorage = false
				setStoredString('player.playbackSource', 'library')
				setStoredString('player.trackId', '')
				setStoredNumber('player.positionMs', 0)
				setStoredNumber('player.durationMs', 0)
				update((s) => ({
					...s,
					currentTrack: null,
					beatportTrack: null,
					standaloneTrack: null,
					playbackState,
					error: null,
				}))
				stopPositionTracking()
			} catch (error) {
				update((s) => ({
					...s,
					error: toErrorMessage(error, 'Failed to stop'),
				}))
			}
		},

		/**
		 * Seek to position (source-aware)
		 */
		async seek(positionMs: number) {
			const state = getState()

			if (state.playbackSource === 'preview' || state.playbackSource === 'beatport') {
				previewPlayer.seek(positionMs)
				update((s) => ({
					...s,
					playbackState: { ...s.playbackState, position_ms: positionMs },
				}))
				return
			}

			// Optimistic position update to prevent playhead jump-back
			persistPositionImmediate(positionMs)
			update((s) => ({
				...s,
				playbackState: { ...s.playbackState, position_ms: positionMs },
			}))
			try {
				const playbackState = await playerApi.seek(positionMs)
				update((s) => ({ ...s, playbackState, error: null }))
			} catch (error) {
				update((s) => ({
					...s,
					error: toErrorMessage(error, 'Failed to seek'),
				}))
			}
		},

		/**
		 * Set volume (source-aware)
		 */
		async setVolume(volume: number) {
			const state = getState()
			setStoredNumber('player.volume', volume)

			if (state.playbackSource === 'preview' || state.playbackSource === 'beatport') {
				previewPlayer.setVolume(volume)
				update((s) => ({
					...s,
					playbackState: { ...s.playbackState, volume },
					error: null,
				}))
				return
			}

			try {
				const playbackState = await playerApi.setVolume(volume)
				update((s) => ({ ...s, playbackState, error: null }))
			} catch (error) {
				update((s) => ({
					...s,
					error: toErrorMessage(error, 'Failed to set volume'),
				}))
			}
		},

		/**
		 * Set playback speed (source-aware)
		 */
		async setSpeed(speed: number) {
			const state = getState()
			setStoredNumber('player.speed', speed)

			if (state.playbackSource === 'preview') {
				update((s) => ({
					...s,
					playbackState: { ...s.playbackState, speed },
					error: null,
				}))
				return
			}

			// Optimistic update for immediate UI response
			update((s) => ({
				...s,
				playbackState: { ...s.playbackState, speed },
			}))
			try {
				const playbackState = await playerApi.setSpeed(speed)
				update((s) => ({ ...s, playbackState, error: null }))
			} catch (error) {
				update((s) => ({
					...s,
					error: toErrorMessage(error, 'Failed to set speed'),
				}))
			}
		},

		/**
		 * Commit a preview speed change: apply the rate to the audio element,
		 * show the loading spinner, and force a pause/resume to guarantee the
		 * 'playing' event fires when audio actually resumes.
		 */
		commitPreviewSpeed() {
			const state = getState()
			if (state.playbackSource !== 'preview' || !state.previewInfo) return
			if (!state.playbackState.is_playing) return

			// Clear any pending safety timeout from a previous commit
			if (previewSpeedCommitTimeout) {
				clearTimeout(previewSpeedCommitTimeout)
				previewSpeedCommitTimeout = null
			}

			// Apply rate change and show spinner
			previewPlayer.setPlaybackRate(state.playbackState.speed)
			update((s) => ({ ...s, previewLoadingReleaseId: s.previewInfo?.releaseId ?? null }))

			// Force pause+resume so the 'playing' event fires when audio resumes
			previewPlayer.pause()
			previewPlayer.resume()

			// Safety timeout: clear spinner if 'playing' never fires
			previewSpeedCommitTimeout = setTimeout(() => {
				previewSpeedCommitTimeout = null
				update((s) => ({ ...s, previewLoadingReleaseId: null }))
			}, 5000)
		},

		/**
		 * Toggle play/pause (source-aware)
		 */
		async togglePlayPause() {
			const state = getState()

			if (state.playbackState.is_playing) {
				await this.pause()
			} else {
				await this.resume()
			}
		},

		/**
		 * Seek relative to current position
		 */
		async seekRelative(offsetMs: number) {
			const state = getState()
			const { position_ms, duration_ms } = state.playbackState
			const newPosition = Math.max(0, Math.min(duration_ms, position_ms + offsetMs))
			await this.seek(newPosition)
		},

		/**
		 * Adjust volume by a relative amount
		 */
		async adjustVolume(delta: number) {
			const state = getState()

			// If muted and trying to increase volume, unmute first
			if (state.isMuted && delta > 0) {
				setStoredBoolean('player.isMuted', false)
				update((s) => ({ ...s, isMuted: false }))
			}

			// Clamp to valid range (0.0 - 1.0)
			const newVolume = Math.max(0, Math.min(1, state.playbackState.volume + delta))
			await this.setVolume(newVolume)
		},

		/**
		 * Toggle mute/unmute
		 */
		async toggleMute() {
			const state = getState()

			if (state.isMuted) {
				setStoredBoolean('player.isMuted', false)
				update((s) => ({ ...s, isMuted: false }))
				await this.setVolume(state.volumeBeforeMute)
			} else {
				setStoredBoolean('player.isMuted', true)
				setStoredNumber('player.volumeBeforeMute', state.playbackState.volume)
				update((s) => ({ ...s, isMuted: true, volumeBeforeMute: state.playbackState.volume }))
				await this.setVolume(0)
			}
		},

		/**
		 * Toggle shuffle mode. Persisted device-locally; affects playback order only.
		 */
		toggleShuffle() {
			update((s) => {
				const next = !s.shuffleEnabled
				setStoredBoolean('player.shuffleEnabled', next)
				return { ...s, shuffleEnabled: next }
			})
		},

		/**
		 * Register a callback for when a track finishes playing
		 */
		onTrackEnd(callback: (() => void) | null) {
			onTrackEndCallback = callback
		},

		setOnTrackEnd(callback: (() => void) | null) {
			onTrackEndCallback = callback
		},

		/**
		 * Register a handler called when playback fails because the track file is missing.
		 * Desktop wires this to the missingTracks store; mobile can leave it unset.
		 */
		setTrackMissingHandler(handler: ((trackId: string) => void) | null) {
			onTrackMissing = handler
		},

		/**
		 * Update is_liked for a preview track (keeps player store in sync with discovery store)
		 */
		setPreviewTrackLiked(trackId: string, isLiked: boolean) {
			update((s) => {
				if (!s.previewInfo) return s
				return {
					...s,
					previewInfo: {
						...s.previewInfo,
						release: {
							...s.previewInfo.release,
							tracks: s.previewInfo.release.tracks.map((t) => (t.id === trackId ? { ...t, is_liked: isLiked } : t)),
						},
					},
				}
			})
		},

		/**
		 * Restore the last-playing library track from localStorage after app init.
		 * Sets the track in the UI at the stored position without loading audio in the backend.
		 */
		restoreTrack(tracks: Track[]) {
			if (restoredPlaybackSource !== 'library' || !restoredTrackId) return
			const track = tracks.find((t) => t.id === restoredTrackId)
			if (!track) return
			isRestoredFromStorage = true
			update((s) => ({
				...s,
				currentTrack: track,
				standaloneTrack: null,
				playbackState: {
					...s.playbackState,
					duration_ms: track.duration_ms || s.playbackState.duration_ms,
				},
			}))
			loadTrackCuesAndWaveform(track.id)
		},

		/**
		 * Restore a preview track from localStorage after app init.
		 * Fetches the release by ID and sets the UI state without loading audio.
		 */
		async restorePreview() {
			if (restoredPlaybackSource !== 'preview' || !restoredPreviewReleaseId) return
			try {
				const release = await discoveryApi.getRelease(restoredPreviewReleaseId)
				const track = release.tracks[restoredPreviewTrackIndex]
				if (!track) return
				isRestoredFromStorage = true
				update((s) => ({
					...s,
					currentTrack: null,
					standaloneTrack: null,
					playbackState: {
						...s.playbackState,
						duration_ms: track.duration_ms || s.playbackState.duration_ms,
					},
					playbackSource: 'preview',
					previewInfo: { releaseId: release.id, release, trackIndex: restoredPreviewTrackIndex },
					previewTrackIndex: restoredPreviewTrackIndex,
				}))
			} catch {
				// Release no longer exists — clear stale persistence silently
				setStoredString('player.playbackSource', 'library')
				setStoredString('player.previewReleaseId', '')
			}
		},

		/**
		 * Reset store to initial state
		 */
		reset() {
			stopPreviewInternal()
			clearPreviewEvents()
			stopPositionTracking()
			onTrackEndCallback = null
			isRestoredFromStorage = false
			setStoredString('player.playbackSource', 'library')
			setStoredString('player.trackId', '')
			setStoredString('player.previewReleaseId', '')
			setStoredNumber('player.positionMs', 0)
			setStoredNumber('player.durationMs', 0)
			set(initialState)
		},

		/**
		 * Marks the standalone file as imported into the library (updates the store, never a copy).
		 */
		markStandaloneInLibrary(filePath: string) {
			update((s) =>
				s.standaloneTrack?.file_path === filePath
					? { ...s, standaloneTrack: { ...s.standaloneTrack, is_in_library: true } }
					: s
			)
		},

		/**
		 * Jump directly to a Hot Cue or Memory Cue
		 */
		async jumpToCue(cue: Cue) {
			if (cue && typeof cue.position_ms === 'number') {
				await this.seek(cue.position_ms)
			}
		},

		/**
		 * Jump to the hot cue of pad 1–8 (key 1 → slot 0 … key 8 → slot 7). An empty pad does nothing.
		 */
		async jumpToCueIndex(padNumber: number) {
			const cue = hotCueForSlot(getState().currentCues, padNumber - 1)
			if (cue && typeof cue.position_ms === 'number') {
				await this.seek(cue.position_ms)
			}
		},
	}
}

export const playerStore = createPlayerStore()

// =============================================================================
// Derived Stores
// =============================================================================

export const isPlaying = derived(playerStore, ($player) => $player.playbackState.is_playing)

export const currentTrack = derived(playerStore, ($player) => $player.currentTrack)

export const playbackPosition = derived(playerStore, ($player) => $player.playbackState.position_ms)

export const playbackDuration = derived(playerStore, ($player) => $player.playbackState.duration_ms)

export const volume = derived(playerStore, ($player) => $player.playbackState.volume)

export const playbackProgress = derived(playerStore, ($player) => {
	const { position_ms, duration_ms } = $player.playbackState
	if (duration_ms === 0) return 0
	return (position_ms / duration_ms) * 100
})

export const isMuted = derived(playerStore, ($player) => $player.isMuted)

export const shuffleEnabled = derived(playerStore, ($player) => $player.shuffleEnabled)

export const playbackSource = derived(playerStore, ($player) => $player.playbackSource)

export const previewInfo = derived(playerStore, ($player) => $player.previewInfo)

export const previewTrackIndex = derived(playerStore, ($player) => $player.previewTrackIndex)

export const previewLoadingReleaseId = derived(playerStore, ($player) => $player.previewLoadingReleaseId)

export const playbackSpeed = derived(playerStore, ($player) => $player.playbackState.speed)

export const beatportTrack = derived(playerStore, ($player) => $player.beatportTrack)

export const standaloneTrack = derived(playerStore, ($player) => $player.standaloneTrack)

export const currentCues = derived(playerStore, ($player) => $player.currentCues)

export const currentWaveformBars = derived(playerStore, ($player) => $player.waveformBars)
