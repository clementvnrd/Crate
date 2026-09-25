import { describe, it, expect, beforeEach, vi } from 'vitest'
import { get } from 'svelte/store'
import {
	albumsStore,
	playerAlbums,
	selectedAlbum,
	selectedAlbumTracks,
} from './albums'
import * as albumApi from '../api/album'
import type { PlayerAlbum, PlayerAlbumTrack, AddAlbumResult } from '../types'

vi.mock('../api/album', () => ({
	addPlayerAlbum: vi.fn(),
	getPlayerAlbums: vi.fn(),
	getPlayerAlbumTracks: vi.fn(),
	removePlayerAlbum: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
	open: vi.fn(),
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
