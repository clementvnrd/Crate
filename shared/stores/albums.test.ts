import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { get } from 'svelte/store'
import { albumsStore, playerAlbums, selectedAlbum, selectedAlbumTracks } from './albums'
import { playerStore } from './player'
import { toastStore, toasts } from './toast'
import * as albumApi from '../api/album'
import * as playerApi from '../api/player'
import * as standaloneApi from '../api/standalone'
import type { PlayerAlbum, PlayerAlbumTrack, AddAlbumResult, Track } from '../types'

vi.mock('../api/album', () => ({
	addPlayerAlbum: vi.fn(),
	getPlayerAlbums: vi.fn(),
	getPlayerAlbumTracks: vi.fn(),
	removePlayerAlbum: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
	open: vi.fn(),
}))

vi.mock('../api/standalone', () => ({
	playStandaloneTrack: vi.fn(),
	addRecentStandaloneTrack: vi.fn(),
}))

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

vi.mock('../api/library', () => ({
	getTrackCues: vi.fn().mockResolvedValue([]),
	getTrackWaveform: vi.fn().mockResolvedValue(null),
}))

const mockAlbum: PlayerAlbum = {
	id: 'album-1',
	folder_path: '/Users/test/Music/Discovery',
	title: 'Discovery',
	artist: 'Daft Punk',
	year: 2001,
	genre: 'Electronic',
	artwork_path: 'artwork/album-1.webp',
	track_count: 2,
	total_duration_ms: 450000,
	created_at: '2026-09-04T00:00:00Z',
}

const mockTracks: PlayerAlbumTrack[] = [
	{
		id: 'track-1',
		album_id: 'album-1',
		file_path: '/Users/test/Music/Discovery/01 One More Time.flac',
		track_number: 1,
		title: 'One More Time',
		artist: 'Daft Punk',
		duration_ms: 320000,
		format: 'flac',
		bitrate: 1411,
		sample_rate: 44100,
		bpm: 123,
		key: '11B',
		energy: 8,
		artwork_path: 'artwork/album-1.webp',
	},
	{
		id: 'track-2',
		album_id: 'album-1',
		file_path: '/Users/test/Music/Discovery/02 Aerodynamic.flac',
		track_number: 2,
		title: 'Aerodynamic',
		artist: 'Daft Punk',
		duration_ms: 130000,
		format: 'flac',
		bitrate: 1411,
		sample_rate: 44100,
		bpm: 123,
		key: '4A',
		energy: 9,
		artwork_path: 'artwork/album-1.webp',
	},
]

describe('albumsStore', () => {
	beforeEach(() => {
		vi.clearAllMocks()
	})

	it('initializes with empty albums and selection', () => {
		expect(get(playerAlbums)).toEqual([])
		expect(get(selectedAlbum)).toBeNull()
		expect(get(selectedAlbumTracks)).toEqual([])
	})

	it('loads albums from API', async () => {
		vi.mocked(albumApi.getPlayerAlbums).mockResolvedValueOnce([mockAlbum])

		await albumsStore.loadAlbums()

		expect(albumApi.getPlayerAlbums).toHaveBeenCalled()
		expect(get(playerAlbums)).toHaveLength(1)
		expect(get(playerAlbums)[0].title).toBe('Discovery')
	})

	it('selects an album and loads its tracks', async () => {
		vi.mocked(albumApi.getPlayerAlbumTracks).mockResolvedValueOnce(mockTracks)

		await albumsStore.selectAlbum(mockAlbum)

		expect(albumApi.getPlayerAlbumTracks).toHaveBeenCalledWith('album-1')
		expect(get(selectedAlbum)?.title).toBe('Discovery')
		expect(get(selectedAlbumTracks)).toHaveLength(2)
		expect(get(selectedAlbumTracks)[0].title).toBe('One More Time')
	})

	it('deselects an album and clears tracklist', async () => {
		await albumsStore.selectAlbum(null)

		expect(get(selectedAlbum)).toBeNull()
		expect(get(selectedAlbumTracks)).toEqual([])
	})

	it('removes an album and clears selection if currently selected', async () => {
		vi.mocked(albumApi.getPlayerAlbums).mockResolvedValueOnce([mockAlbum])
		vi.mocked(albumApi.getPlayerAlbumTracks).mockResolvedValueOnce(mockTracks)
		vi.mocked(albumApi.removePlayerAlbum).mockResolvedValueOnce()

		await albumsStore.loadAlbums()
		await albumsStore.selectAlbum(mockAlbum)

		await albumsStore.removeAlbum('album-1')

		expect(albumApi.removePlayerAlbum).toHaveBeenCalledWith('album-1')
		expect(get(playerAlbums)).toHaveLength(0)
		expect(get(selectedAlbum)).toBeNull()
	})
})

// Album next / previous (CRA-180, CRA-181): albums honour the shuffle toggle with the same no-repeat session as the
// library, and skip a track whose file cannot be loaded.
describe('albumsStore navigation', () => {
	const sixTracks: PlayerAlbumTrack[] = Array.from({ length: 6 }, (_, i) => ({
		...mockTracks[0],
		id: `a-${i + 1}`,
		track_number: i + 1,
		title: `Track ${i + 1}`,
		file_path: `/Users/test/Music/Album/0${i + 1}.flac`,
	}))
	let missing = new Set<string>()

	const playing = () => get(playerStore).standaloneTrack?.id

	beforeEach(() => {
		vi.useFakeTimers()
		vi.clearAllMocks()
		playerStore.reset()
		toastStore.clear()
		missing = new Set()
		vi.mocked(standaloneApi.playStandaloneTrack).mockImplementation(async (path, id, durationMs) => {
			if (missing.has(String(id))) throw `File not found: ${path}`
			return {
				is_playing: true,
				position_ms: 0,
				duration_ms: durationMs ?? 0,
				volume: 1,
				speed: 1,
				current_track_id: String(id),
				current_track_path: path,
			}
		})
		vi.mocked(albumApi.getPlayerAlbumTracks).mockResolvedValue(sixTracks)
	})

	afterEach(() => {
		if (get(playerStore).shuffleEnabled) playerStore.toggleShuffle()
		playerStore.reset()
		toastStore.clear()
		vi.useRealTimers()
	})

	async function startAlbum() {
		await albumsStore.selectAlbum(null)
		await albumsStore.playAlbum(mockAlbum)
		expect(playing()).toBe('a-1')
	}

	it('plays the album in order with shuffle off', async () => {
		await startAlbum()
		await albumsStore.playNextAlbumTrack()
		expect(playing()).toBe('a-2')
		await albumsStore.playPreviousAlbumTrack()
		expect(playing()).toBe('a-1')
	})

	it('honours shuffle: every track once before any repeats', async () => {
		await startAlbum()
		playerStore.toggleShuffle()
		const heard = [playing()]
		for (let i = 0; i < sixTracks.length - 1; i++) {
			await albumsStore.playNextAlbumTrack()
			heard.push(playing())
		}
		expect(new Set(heard).size).toBe(sixTracks.length)
	})

	it('shuffle "previous" walks back through what was heard, then restarts the first track', async () => {
		await startAlbum()
		playerStore.toggleShuffle()
		await albumsStore.playNextAlbumTrack()
		const second = playing()
		await albumsStore.playNextAlbumTrack()
		await albumsStore.playPreviousAlbumTrack()
		expect(playing()).toBe(second)
		await albumsStore.playPreviousAlbumTrack()
		expect(playing()).toBe('a-1')
		vi.mocked(playerApi.seek).mockResolvedValue(get(playerStore).playbackState)
		await albumsStore.playPreviousAlbumTrack()
		expect(playing()).toBe('a-1')
		expect(playerApi.seek).toHaveBeenCalledWith(0)
	})

	it('skips a track whose file is missing and says so once', async () => {
		await startAlbum()
		missing.add('a-2')
		await albumsStore.playNextAlbumTrack()
		expect(playing()).toBe('a-3')
		expect(get(toasts).map((t) => t.message)).toEqual(['Skipped "Track 2": its file could not be loaded'])
	})

	it('stops skipping as soon as something else starts playing', async () => {
		await startAlbum()
		missing.add('a-2')
		missing.add('a-3')
		const libraryPick = {
			id: 'lib-1',
			file_path: '/music/lib-1.flac',
			title: 'Library pick',
			duration_ms: 200_000,
		} as unknown as Track
		vi.mocked(playerApi.playTrack).mockResolvedValue({
			is_playing: true,
			position_ms: 0,
			duration_ms: 200_000,
			volume: 1,
			speed: 1,
			current_track_id: 'lib-1',
			current_track_path: '/music/lib-1.flac',
		})
		const loadAlbumTrack = vi.mocked(standaloneApi.playStandaloneTrack).getMockImplementation()!
		vi.mocked(standaloneApi.playStandaloneTrack).mockImplementation(async (path, id, durationMs) => {
			// The DJ starts a library track while the album's next track is still failing to load.
			if (id === 'a-2') await playerStore.play(libraryPick)
			return loadAlbumTrack(path, id, durationMs)
		})

		await albumsStore.playNextAlbumTrack()

		const state = get(playerStore)
		expect(state.playbackSource).toBe('library')
		expect(state.currentTrack?.id).toBe('lib-1')
		expect(vi.mocked(standaloneApi.playStandaloneTrack).mock.calls.map((call) => call[1])).not.toContain('a-3')
		expect(get(toasts)).toEqual([])
	})

	it('"play album" starts at the first track that loads', async () => {
		missing.add('a-1')
		await albumsStore.selectAlbum(null)
		await albumsStore.playAlbum(mockAlbum)
		expect(playing()).toBe('a-2')
	})
})
