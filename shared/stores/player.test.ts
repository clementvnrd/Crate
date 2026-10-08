import { describe, it, expect, beforeEach, afterEach, vi, type Mock } from 'vitest'
import { get } from 'svelte/store'
import { playerStore, reportSkippedTracks } from './player'
import { toastStore, toasts } from './toast'
import type { PlaybackState, Track, DiscoveryRelease } from '../types'
import type { NavigationOutcome } from '../utils/playbackQueue'
import * as playerApi from '../api/player'
import * as discoveryApi from '../api/discovery'
import * as previewPlayer from '../services/previewPlayer'

vi.mock('../api/player', () => ({
	playTrack: vi.fn(),
	pause: vi.fn(),
	resume: vi.fn(),
	stop: vi.fn(),
	seek: vi.fn(),
	setVolume: vi.fn(),
	setSpeed: vi.fn(),
	getPlaybackState: vi.fn(),
}))

vi.mock('../api/discovery', () => ({
	fetchPreviewStream: vi.fn(),
	invalidatePreviewStreamCache: vi.fn(),
	getRelease: vi.fn(),
}))

vi.mock('../api/standalone', () => ({
	playStandaloneTrack: vi.fn(),
	addRecentStandaloneTrack: vi.fn(),
}))

vi.mock('../api/library', () => ({
	getTrackCues: vi.fn().mockResolvedValue([]),
	getTrackWaveform: vi.fn().mockResolvedValue(null),
}))

vi.mock('../services/previewPlayer', () => ({
	play: vi.fn(),
	pause: vi.fn(),
	resume: vi.fn(),
	stop: vi.fn(),
	seek: vi.fn(),
	setVolume: vi.fn(),
	setPlaybackRate: vi.fn(),
	setOnEnded: vi.fn(),
	setOnTimeUpdate: vi.fn(),
	setOnDurationChange: vi.fn(),
	setOnError: vi.fn(),
	setOnWaiting: vi.fn(),
	setOnPlaying: vi.fn(),
}))

const DURATION_MS = 10_000

function engineState(overrides: Partial<PlaybackState> = {}): PlaybackState {
	return {
		is_playing: true,
		position_ms: 0,
		duration_ms: DURATION_MS,
		volume: 1,
		speed: 1,
		current_track_id: 'trk-1',
		current_track_path: '/music/one.flac',
		...overrides,
	}
}

function libraryTrack(id = 'trk-1'): Track {
	return { id, file_path: `/music/${id}.flac`, title: id, duration_ms: DURATION_MS } as unknown as Track
}

/** The engine drained its sink: not playing, position clamped at the duration. */
function engineFinished(overrides: Partial<PlaybackState> = {}): PlaybackState {
	return engineState({ is_playing: false, position_ms: DURATION_MS, ...overrides })
}

async function startLibraryTrack(durationMs = DURATION_MS) {
	vi.mocked(playerApi.playTrack).mockResolvedValue(engineState({ duration_ms: durationMs }))
	await playerStore.play(libraryTrack())
}

describe('playerStore end of track', () => {
	let onEnd: Mock<() => void>

	beforeEach(() => {
		vi.useFakeTimers()
		vi.clearAllMocks()
		playerStore.reset()
		onEnd = vi.fn<() => void>()
		playerStore.onTrackEnd(onEnd)
	})

	afterEach(() => {
		playerStore.reset()
		vi.useRealTimers()
	})

	it('fires once when the backend sync reports the engine finished', async () => {
		await startLibraryTrack()
		vi.mocked(playerApi.getPlaybackState).mockResolvedValue(engineFinished())

		// Tick 10 is the first backend sync.
		await vi.advanceTimersByTimeAsync(1050)
		expect(playerApi.getPlaybackState).toHaveBeenCalledTimes(1)
		expect(onEnd).toHaveBeenCalledTimes(1)
		const { playbackState } = get(playerStore)
		expect(playbackState.is_playing).toBe(false)
		expect(playbackState.position_ms).toBe(DURATION_MS)

		// The position tracking is over: nothing fires again, nothing syncs again.
		await vi.advanceTimersByTimeAsync(5000)
		expect(onEnd).toHaveBeenCalledTimes(1)
		expect(playerApi.getPlaybackState).toHaveBeenCalledTimes(1)
	})

	it('uses the store duration when the backend reports none', async () => {
		await startLibraryTrack()
		vi.mocked(playerApi.getPlaybackState).mockResolvedValue(engineFinished({ duration_ms: 0 }))

		await vi.advanceTimersByTimeAsync(1050)
		expect(onEnd).toHaveBeenCalledTimes(1)
	})

	it('does not fire when the backend reports a pause in the middle of the track', async () => {
		await startLibraryTrack()
		vi.mocked(playerApi.getPlaybackState).mockResolvedValue(engineState({ is_playing: false, position_ms: 4000 }))

		await vi.advanceTimersByTimeAsync(3000)
		expect(onEnd).not.toHaveBeenCalled()
		expect(get(playerStore).playbackState.is_playing).toBe(false)
	})

	it('does not fire on a user pause, a seek or a stop', async () => {
		await startLibraryTrack()
		vi.mocked(playerApi.getPlaybackState).mockResolvedValue(engineState({ position_ms: 2000 }))
		await vi.advanceTimersByTimeAsync(1050)

		vi.mocked(playerApi.pause).mockResolvedValue(engineState({ is_playing: false, position_ms: 2000 }))
		await playerStore.pause()
		await vi.advanceTimersByTimeAsync(3000)

		vi.mocked(playerApi.resume).mockResolvedValue(engineState({ position_ms: 2000 }))
		await playerStore.resume()
		vi.mocked(playerApi.seek).mockResolvedValue(engineState({ position_ms: 6000 }))
		await playerStore.seek(6000)
		await vi.advanceTimersByTimeAsync(500)

		vi.mocked(playerApi.stop).mockResolvedValue(engineState({ is_playing: false, position_ms: 0 }))
		await playerStore.stop()
		await vi.advanceTimersByTimeAsync(3000)

		expect(onEnd).not.toHaveBeenCalled()
	})

	it('still fires once from the interpolation when the sync does not see the end', async () => {
		await startLibraryTrack(1500)
		// The engine is still playing at tick 10 (position 1000 of 1500): interpolation finishes the job.
		vi.mocked(playerApi.getPlaybackState).mockResolvedValue(engineState({ duration_ms: 1500, position_ms: 1000 }))

		await vi.advanceTimersByTimeAsync(3000)
		expect(onEnd).toHaveBeenCalledTimes(1)
		expect(get(playerStore).playbackState.is_playing).toBe(false)
	})

	it('still fires once from the interpolation when the backend sync fails', async () => {
		await startLibraryTrack(1500)
		vi.mocked(playerApi.getPlaybackState).mockRejectedValue('boom')

		await vi.advanceTimersByTimeAsync(3000)
		expect(onEnd).toHaveBeenCalledTimes(1)
	})

	it('does not double fire when the interpolation and a late sync both see the end', async () => {
		await startLibraryTrack(1050)
		let resolveSync: (state: PlaybackState) => void = () => {}
		vi.mocked(playerApi.getPlaybackState).mockReturnValue(
			new Promise<PlaybackState>((resolve) => {
				resolveSync = resolve
			})
		)

		// Tick 10 starts a sync that stays pending while ticks 11 and 12 interpolate to the end.
		await vi.advanceTimersByTimeAsync(1250)
		expect(onEnd).toHaveBeenCalledTimes(1)

		resolveSync(engineFinished({ duration_ms: 1050, position_ms: 1050 }))
		await vi.advanceTimersByTimeAsync(3000)
		expect(onEnd).toHaveBeenCalledTimes(1)
	})

	it('fires again for the next track', async () => {
		await startLibraryTrack()
		vi.mocked(playerApi.getPlaybackState).mockResolvedValue(engineFinished())
		await vi.advanceTimersByTimeAsync(1050)
		expect(onEnd).toHaveBeenCalledTimes(1)

		await startLibraryTrack()
		await vi.advanceTimersByTimeAsync(1050)
		expect(onEnd).toHaveBeenCalledTimes(2)
	})

	describe('preview player', () => {
		const release = {
			id: 'rel-1',
			tracks: [{ id: 'pt-1', position: 1, duration_ms: 30_000 }],
		} as unknown as DiscoveryRelease

		async function startPreview() {
			vi.mocked(discoveryApi.fetchPreviewStream).mockResolvedValue('https://example.test/stream.mp3')
			await playerStore.playPreview(release, 0)
			const lastHandler = <T>(fn: unknown): T => {
				const calls = (fn as { mock: { calls: unknown[][] } }).mock.calls
				return calls[calls.length - 1][0] as T
			}
			return {
				timeUpdate: lastHandler<(positionMs: number) => void>(previewPlayer.setOnTimeUpdate),
				ended: lastHandler<() => void>(previewPlayer.setOnEnded),
			}
		}

		it('fires once when the audio element reports both timeupdate at the end and ended', async () => {
			const { timeUpdate, ended } = await startPreview()

			timeUpdate(30_000)
			ended()
			await vi.advanceTimersByTimeAsync(10)
			expect(onEnd).toHaveBeenCalledTimes(1)

			// A late event after the stop must not fire again either.
			ended()
			await vi.advanceTimersByTimeAsync(10)
			expect(onEnd).toHaveBeenCalledTimes(1)
		})

		it('fires once on ended alone, and again after the next preview starts', async () => {
			const first = await startPreview()
			first.ended()
			await vi.advanceTimersByTimeAsync(10)
			expect(onEnd).toHaveBeenCalledTimes(1)

			const second = await startPreview()
			second.ended()
			await vi.advanceTimersByTimeAsync(10)
			expect(onEnd).toHaveBeenCalledTimes(2)
		})
	})
})

// A next track that fails to load (CRA-180): `play` tells the queue whether the track started, a failure leaves the
// playing track alone, and the skipped tracks are reported with one toast per navigation.
describe('playerStore.play result and skipped-track notices', () => {
	beforeEach(() => {
		vi.useFakeTimers()
		vi.clearAllMocks()
		playerStore.reset()
		toastStore.clear()
	})

	afterEach(() => {
		playerStore.reset()
		toastStore.clear()
		vi.useRealTimers()
	})

	it('resolves true when the track starts', async () => {
		vi.mocked(playerApi.playTrack).mockResolvedValue(engineState())
		expect(await playerStore.play(libraryTrack())).toBe(true)
		expect(get(playerStore).currentTrack?.id).toBe('trk-1')
	})

	it('resolves false on a missing file, flags it, and keeps the track that was playing', async () => {
		const onMissing = vi.fn()
		playerStore.setTrackMissingHandler(onMissing)
		await startLibraryTrack()
		vi.mocked(playerApi.playTrack).mockRejectedValue('File not found: /music/trk-2.flac')

		expect(await playerStore.play(libraryTrack('trk-2'))).toBe(false)

		const state = get(playerStore)
		expect(state.currentTrack?.id).toBe('trk-1')
		expect(state.playbackState.is_playing).toBe(true)
		expect(state.error).toBe('File not found: /music/trk-2.flac')
		expect(onMissing).toHaveBeenCalledWith('trk-2')
		playerStore.setTrackMissingHandler(null)
	})

	it('restartTrack seeks a loaded track back to its start', async () => {
		await startLibraryTrack()
		vi.mocked(playerApi.seek).mockResolvedValue(engineState({ position_ms: 0 }))
		await playerStore.restartTrack()
		expect(playerApi.seek).toHaveBeenCalledWith(0)
	})

	const outcome = (overrides: Partial<NavigationOutcome<Track>>): NavigationOutcome<Track> => ({
		started: null,
		skipped: [],
		gaveUp: false,
		restarted: false,
		cancelled: false,
		...overrides,
	})
	const titled = (id: string, title: string | null) => ({ ...libraryTrack(id), title }) as Track
	const messages = () => get(toasts).map((t) => ({ type: t.type, message: t.message }))

	it('shows nothing when no track was skipped', () => {
		reportSkippedTracks(outcome({ started: libraryTrack() }), (t) => t.title)
		expect(messages()).toEqual([])
	})

	it('names the skipped track in one warning when playback moved on', () => {
		reportSkippedTracks(outcome({ started: libraryTrack('c'), skipped: [titled('b', 'Glasshouse')] }), (t) => t.title)
		expect(messages()).toEqual([{ type: 'warning', message: 'Skipped "Glasshouse": its file could not be loaded' }])
	})

	it('names the first one and counts the others', () => {
		reportSkippedTracks(
			outcome({ started: libraryTrack('d'), skipped: [titled('b', null), titled('c', 'Undertow')] }),
			(t) => t.title
		)
		expect(messages()).toEqual([
			{
				type: 'warning',
				message: 'Skipped "Track" and 1 other track: their files could not be loaded',
			},
		])
	})

	it('shows a single error when nothing could start', () => {
		reportSkippedTracks(
			outcome({ skipped: Array.from({ length: 10 }, (_, i) => titled(`t${i}`, `T${i}`)), gaveUp: true }),
			(t) => t.title
		)
		expect(messages()).toEqual([
			{
				type: 'error',
				message: 'Playback stopped: 10 tracks in a row could not be loaded. Is the drive holding your music connected?',
			},
		])
	})
})
