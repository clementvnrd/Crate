import { describe, it, expect, beforeEach, vi } from 'vitest'
import { get } from 'svelte/store'
import {
	recentTracksStore,
	recentStandaloneTracks,
	recentTracksLoading,
} from './recentTracks'
import type { StandaloneTrack } from '$shared/types'
import * as standaloneApi from '$shared/api/standalone'

vi.mock('$shared/api/standalone', () => ({
	readStandaloneTrack: vi.fn(),
	getRecentStandaloneTracks: vi.fn(),
	addRecentStandaloneTrack: vi.fn(),
	removeRecentStandaloneTrack: vi.fn(),
	clearRecentStandaloneTracks: vi.fn(),
	getStartupFile: vi.fn(),
	playStandaloneTrack: vi.fn(),
}))

const mockTracks: StandaloneTrack[] = [
	{
		id: 'standalone_1',
		file_path: '/external/music/Track1.flac',
		title: 'Unreleased Track',
		artist: 'Bicep',
		album: 'Live 2026',
		duration_ms: 320000,
		format: 'flac',
		bitrate: 1411,
		sample_rate: 44100,
		bpm: 128,
		key: '8A',
		energy: 8,
		artwork_path: 'artwork/standalone_1.webp',
		is_in_library: false,
		last_played_at: '2026-08-31T01:00:00Z',
	},
	{
		id: 'standalone_2',
		file_path: '/external/music/Track2.mp3',
		title: 'Demo Edit',
		artist: 'Four Tet',
		album: null,
		duration_ms: 245000,
		format: 'mp3',
		bitrate: 320,
		sample_rate: 44100,
		bpm: 130,
		key: '11B',
		energy: 6,
		artwork_path: null,
		is_in_library: false,
		last_played_at: '2026-08-31T00:30:00Z',
	},
]

describe('recentTracksStore', () => {
	beforeEach(async () => {
		vi.clearAllMocks()
		await recentTracksStore.clear()
	})

	it('initializes with an empty list', () => {
		expect(get(recentStandaloneTracks)).toEqual([])
		expect(get(recentTracksLoading)).toBe(false)
	})

	it('loads recent tracks from API', async () => {
		vi.mocked(standaloneApi.getRecentStandaloneTracks).mockResolvedValueOnce(mockTracks)

		await recentTracksStore.load()

		expect(standaloneApi.getRecentStandaloneTracks).toHaveBeenCalled()
		expect(get(recentStandaloneTracks)).toHaveLength(2)
		expect(get(recentStandaloneTracks)[0].title).toBe('Unreleased Track')
	})

	it('adds a standalone track to recent history', async () => {
		vi.mocked(standaloneApi.addRecentStandaloneTrack).mockResolvedValueOnce()

		const newTrack: StandaloneTrack = {
			id: 'standalone_3',
			file_path: '/external/music/Track3.wav',
			title: 'New Bangers',
			artist: 'Overmono',
			duration_ms: 300000,
			format: 'wav',
			is_in_library: false,
		}

		await recentTracksStore.addTrack(newTrack)

		expect(standaloneApi.addRecentStandaloneTrack).toHaveBeenCalledWith(newTrack)
		expect(get(recentStandaloneTracks)).toHaveLength(1)
		expect(get(recentStandaloneTracks)[0].id).toBe('standalone_3')
	})

	it('strictly prevents library tracks from being added to standalone recent history', async () => {
		const libraryTrack: StandaloneTrack = {
			id: 'lib_track_1',
			file_path: '/crate/library/Track.flac',
			title: 'Library Track',
			artist: 'Floating Points',
			duration_ms: 300000,
			format: 'flac',
			is_in_library: true,
		}

		await recentTracksStore.addTrack(libraryTrack)

		// API should not be called and store should remain empty
		expect(standaloneApi.addRecentStandaloneTrack).not.toHaveBeenCalled()
		expect(get(recentStandaloneTracks)).toHaveLength(0)
	})

	it('removes a track by id', async () => {
		vi.mocked(standaloneApi.getRecentStandaloneTracks).mockResolvedValueOnce(mockTracks)
		await recentTracksStore.load()

		vi.mocked(standaloneApi.removeRecentStandaloneTrack).mockResolvedValueOnce()
		await recentTracksStore.removeTrack('standalone_1')

		expect(standaloneApi.removeRecentStandaloneTrack).toHaveBeenCalledWith('standalone_1')
		expect(get(recentStandaloneTracks)).toHaveLength(1)
		expect(get(recentStandaloneTracks)[0].id).toBe('standalone_2')
	})

	it('clears all recent standalone tracks', async () => {
		vi.mocked(standaloneApi.getRecentStandaloneTracks).mockResolvedValueOnce(mockTracks)
		await recentTracksStore.load()

		vi.mocked(standaloneApi.clearRecentStandaloneTracks).mockResolvedValueOnce()
		await recentTracksStore.clear()

		expect(standaloneApi.clearRecentStandaloneTracks).toHaveBeenCalled()
		expect(get(recentStandaloneTracks)).toEqual([])
	})
})
