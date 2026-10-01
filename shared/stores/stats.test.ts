import { describe, it, expect, beforeEach, vi } from 'vitest'
import { get } from 'svelte/store'
import {
	statsStore,
	statsSummary,
	topTracks,
	topArtists,
	harmonicStats,
	bpmStats,
	listeningHeatmap,
	statsSelectedRange,
	spotifyAuth,
	spotifyNowPlaying,
} from './stats'
import * as statsApi from '../api/stats'
import type {
	StatsSummary,
	TopTrackItem,
	TopArtistItem,
	HarmonicStatsItem,
	BpmBucketItem,
	HeatmapCell,
	SpotifyAuthState,
	SpotifyNowPlaying,
} from '../types'

vi.mock('../api/stats', () => ({
	getStatsSummary: vi.fn(),
	getTopTracks: vi.fn(),
	getTopArtists: vi.fn(),
	getHarmonicStats: vi.fn(),
	getBpmStats: vi.fn(),
	getListeningHeatmap: vi.fn(),
	getRecentListens: vi.fn(),
	getSpotifyAuthState: vi.fn(),
	getSpotifyNowPlaying: vi.fn(),
	getSpotifyAuthUrl: vi.fn(),
	handleSpotifyCallback: vi.fn(),
	disconnectSpotify: vi.fn(),
	importSpotifyHistoryJson: vi.fn(),
	getRekordboxDetectStatus: vi.fn(),
	syncRekordboxHistory: vi.fn(),
	getRekordboxSessions: vi.fn(),
	getMikDetectStatus: vi.fn(),
	setSpotifyClientId: vi.fn(),
	getSpotifyClientId: vi.fn(),
	setSpotifyClientSecret: vi.fn(),
	hasSpotifyClientSecret: vi.fn(),
	syncSpotifyRecentlyPlayed: vi.fn().mockResolvedValue(0),
	countSpotifyListens: vi.fn(),
	resetSpotifyListeningHistory: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-opener', () => ({
	openUrl: vi.fn(),
}))

const mockSummary: StatsSummary = {
	total_minutes: 1250,
	today_minutes: 120,
	week_minutes: 840,
	month_minutes: 1250,
	total_plays: 350,
	source_breakdown: {
		spotify: 600,
		crate_local: 400,
		crate_beatport: 150,
		rekordbox: 100,
	},
}

const mockTopTracks: TopTrackItem[] = [
	{
		title: 'Innerbloom',
		artist: 'RÜFÜS DU SOL',
		album: 'Bloom',
		artwork_url: 'https://example.com/art.jpg',
		plays: 42,
		total_minutes: 380,
		bpm: 122,
		key: '8A',
		energy: 8,
		sources: ['spotify', 'crate_local'],
	},
]

const mockTopArtists: TopArtistItem[] = [
	{
		artist: 'RÜFÜS DU SOL',
		plays: 95,
		total_minutes: 650,
		top_track: 'Innerbloom',
		artwork_url: 'https://example.com/artist.jpg',
	},
]

const mockHarmonicStats: HarmonicStatsItem[] = [
	{
		key: '8A',
		plays: 85,
		total_minutes: 520,
		percentage: 24.3,
	},
	{
		key: '9A',
		plays: 60,
		total_minutes: 340,
		percentage: 17.1,
	},
]

const mockBpmStats: BpmBucketItem[] = [
	{
		bpm_range: '120-125',
		count: 180,
		total_minutes: 750,
	},
	{
		bpm_range: '125-130',
		count: 90,
		total_minutes: 400,
	},
]

const mockHeatmap: HeatmapCell[] = [
	{
		day_of_week: 3,
		hour_of_day: 18,
		minutes: 145,
		plays: 32,
	},
]

const mockSpotifyAuth: SpotifyAuthState = {
	is_connected: true,
	user_id: 'dj_alex',
	user_name: 'Alex Crate',
	expires_at: 1750000000,
}

const mockNowPlaying: SpotifyNowPlaying = {
	is_playing: true,
	track_id: 'sp_123',
	title: 'On My Mind',
	artist: 'Diplo & SIDEPIECE',
	album: 'On My Mind',
	duration_ms: 185000,
	progress_ms: 45000,
	artwork_url: 'https://example.com/onmymind.jpg',
	device_name: 'MacBook Pro',
}

describe('statsStore', () => {
	beforeEach(() => {
		vi.clearAllMocks()
		statsStore.reset()
	})

	it('initializes with default 7d range and empty data', () => {
		expect(get(statsSelectedRange)).toBe('7d')
		expect(get(statsSummary)).toBeNull()
		expect(get(topTracks)).toEqual([])
		expect(get(topArtists)).toEqual([])
		expect(get(harmonicStats)).toEqual([])
		expect(get(bpmStats)).toEqual([])
		expect(get(listeningHeatmap)).toEqual([])
		expect(get(spotifyAuth)).toBeNull()
		expect(get(spotifyNowPlaying)).toBeNull()
	})

	it('refreshes all statistics concurrently from API', async () => {
		vi.mocked(statsApi.getStatsSummary).mockResolvedValueOnce(mockSummary)
		vi.mocked(statsApi.getTopTracks).mockResolvedValueOnce(mockTopTracks)
		vi.mocked(statsApi.getTopArtists).mockResolvedValueOnce(mockTopArtists)
		vi.mocked(statsApi.getHarmonicStats).mockResolvedValueOnce(mockHarmonicStats)
		vi.mocked(statsApi.getBpmStats).mockResolvedValueOnce(mockBpmStats)
		vi.mocked(statsApi.getListeningHeatmap).mockResolvedValueOnce(mockHeatmap)
		vi.mocked(statsApi.getRecentListens).mockResolvedValueOnce([])
		vi.mocked(statsApi.getSpotifyAuthState).mockResolvedValueOnce(mockSpotifyAuth)
		vi.mocked(statsApi.getSpotifyNowPlaying).mockResolvedValueOnce(mockNowPlaying)
		vi.mocked(statsApi.getRekordboxDetectStatus).mockResolvedValueOnce(true)
		vi.mocked(statsApi.getRekordboxSessions).mockResolvedValueOnce([])
		vi.mocked(statsApi.getMikDetectStatus).mockResolvedValueOnce(true)

		await statsStore.refreshAll()

		expect(get(statsSummary)).toEqual(mockSummary)
		expect(get(topTracks)).toHaveLength(1)
		expect(get(topTracks)[0].title).toBe('Innerbloom')
		expect(get(topArtists)).toHaveLength(1)
		expect(get(topArtists)[0].artist).toBe('RÜFÜS DU SOL')
		expect(get(harmonicStats)).toHaveLength(2)
		expect(get(bpmStats)).toHaveLength(2)
		expect(get(listeningHeatmap)).toHaveLength(1)
		expect(get(spotifyAuth)?.is_connected).toBe(true)
		expect(get(spotifyNowPlaying)?.is_playing).toBe(true)
	})

	it('changes selected range and triggers refresh', async () => {
		vi.mocked(statsApi.getStatsSummary).mockResolvedValue(mockSummary)
		vi.mocked(statsApi.getTopTracks).mockResolvedValue(mockTopTracks)
		vi.mocked(statsApi.getTopArtists).mockResolvedValue(mockTopArtists)
		vi.mocked(statsApi.getHarmonicStats).mockResolvedValue(mockHarmonicStats)
		vi.mocked(statsApi.getBpmStats).mockResolvedValue(mockBpmStats)
		vi.mocked(statsApi.getListeningHeatmap).mockResolvedValue(mockHeatmap)
		vi.mocked(statsApi.getRecentListens).mockResolvedValue([])
		vi.mocked(statsApi.getSpotifyAuthState).mockResolvedValue(mockSpotifyAuth)
		vi.mocked(statsApi.getSpotifyNowPlaying).mockResolvedValue(null)
		vi.mocked(statsApi.getRekordboxDetectStatus).mockResolvedValue(true)
		vi.mocked(statsApi.getRekordboxSessions).mockResolvedValue([])
		vi.mocked(statsApi.getMikDetectStatus).mockResolvedValue(true)

		await statsStore.setRange('30d')

		expect(get(statsSelectedRange)).toBe('30d')
		expect(statsApi.getStatsSummary).toHaveBeenCalledWith('30d')
	})

	it('imports Spotify history JSON and triggers refresh', async () => {
		vi.mocked(statsApi.importSpotifyHistoryJson).mockResolvedValueOnce({
			imported_count: 1500,
			skipped_count: 5,
			total_minutes: 6200,
		})
		vi.mocked(statsApi.getStatsSummary).mockResolvedValue(mockSummary)
		vi.mocked(statsApi.getTopTracks).mockResolvedValue(mockTopTracks)
		vi.mocked(statsApi.getTopArtists).mockResolvedValue(mockTopArtists)
		vi.mocked(statsApi.getHarmonicStats).mockResolvedValue(mockHarmonicStats)
		vi.mocked(statsApi.getBpmStats).mockResolvedValue(mockBpmStats)
		vi.mocked(statsApi.getListeningHeatmap).mockResolvedValue(mockHeatmap)
		vi.mocked(statsApi.getRecentListens).mockResolvedValue([])
		vi.mocked(statsApi.getSpotifyAuthState).mockResolvedValue(mockSpotifyAuth)
		vi.mocked(statsApi.getSpotifyNowPlaying).mockResolvedValue(null)
		vi.mocked(statsApi.getRekordboxDetectStatus).mockResolvedValue(true)
		vi.mocked(statsApi.getRekordboxSessions).mockResolvedValue([])
		vi.mocked(statsApi.getMikDetectStatus).mockResolvedValue(true)

		const res = await statsStore.importSpotifyJson('[{ "ts": "2026-01-01" }]')

		expect(res?.imported_count).toBe(1500)
		expect(statsApi.importSpotifyHistoryJson).toHaveBeenCalledWith('[{ "ts": "2026-01-01" }]')
	})

	it('syncs Rekordbox history and updates store', async () => {
		vi.mocked(statsApi.syncRekordboxHistory).mockResolvedValueOnce(85)
		vi.mocked(statsApi.getStatsSummary).mockResolvedValue(mockSummary)
		vi.mocked(statsApi.getTopTracks).mockResolvedValue(mockTopTracks)
		vi.mocked(statsApi.getTopArtists).mockResolvedValue(mockTopArtists)
		vi.mocked(statsApi.getHarmonicStats).mockResolvedValue(mockHarmonicStats)
		vi.mocked(statsApi.getBpmStats).mockResolvedValue(mockBpmStats)
		vi.mocked(statsApi.getListeningHeatmap).mockResolvedValue(mockHeatmap)
		vi.mocked(statsApi.getRecentListens).mockResolvedValue([])
		vi.mocked(statsApi.getSpotifyAuthState).mockResolvedValue(mockSpotifyAuth)
		vi.mocked(statsApi.getSpotifyNowPlaying).mockResolvedValue(null)
		vi.mocked(statsApi.getRekordboxDetectStatus).mockResolvedValue(true)
		vi.mocked(statsApi.getRekordboxSessions).mockResolvedValue([])
		vi.mocked(statsApi.getMikDetectStatus).mockResolvedValue(true)

		const count = await statsStore.syncRekordbox()

		expect(count).toBe(85)
		expect(statsApi.syncRekordboxHistory).toHaveBeenCalled()
	})

	it('resets Spotify history and triggers refresh', async () => {
		vi.mocked(statsApi.resetSpotifyListeningHistory).mockResolvedValueOnce({
			deleted_count: 42,
			backup_path: '/tmp/spotify-reset-backup-20261001-120000.json',
		})
		vi.mocked(statsApi.getStatsSummary).mockResolvedValue(mockSummary)
		vi.mocked(statsApi.getTopTracks).mockResolvedValue(mockTopTracks)
		vi.mocked(statsApi.getTopArtists).mockResolvedValue(mockTopArtists)
		vi.mocked(statsApi.getHarmonicStats).mockResolvedValue(mockHarmonicStats)
		vi.mocked(statsApi.getBpmStats).mockResolvedValue(mockBpmStats)
		vi.mocked(statsApi.getListeningHeatmap).mockResolvedValue(mockHeatmap)
		vi.mocked(statsApi.getRecentListens).mockResolvedValue([])
		vi.mocked(statsApi.getSpotifyAuthState).mockResolvedValue(mockSpotifyAuth)
		vi.mocked(statsApi.getSpotifyNowPlaying).mockResolvedValue(null)
		vi.mocked(statsApi.getRekordboxDetectStatus).mockResolvedValue(true)
		vi.mocked(statsApi.getRekordboxSessions).mockResolvedValue([])
		vi.mocked(statsApi.getMikDetectStatus).mockResolvedValue(true)

		const result = await statsStore.resetSpotifyHistory()

		expect(result?.deleted_count).toBe(42)
		expect(statsApi.resetSpotifyListeningHistory).toHaveBeenCalled()
	})

	it('reports a failed reset without throwing', async () => {
		vi.mocked(statsApi.resetSpotifyListeningHistory).mockRejectedValueOnce(new Error('backup failed'))

		const result = await statsStore.resetSpotifyHistory()

		expect(result).toBeNull()
	})

	it('disconnects Spotify and clears auth state', async () => {
		vi.mocked(statsApi.disconnectSpotify).mockResolvedValueOnce()

		await statsStore.disconnectSpotify()

		expect(statsApi.disconnectSpotify).toHaveBeenCalled()
		expect(get(spotifyAuth)?.is_connected).toBe(false)
		expect(get(spotifyNowPlaying)).toBeNull()
	})

	it('connects Spotify with client id, client secret and opens auth URL', async () => {
		vi.mocked(statsApi.getSpotifyAuthUrl).mockResolvedValueOnce('https://accounts.spotify.com/authorize?state=xyz')

		await statsStore.connectSpotify('my-client-id', 'my-client-secret')

		expect(statsApi.setSpotifyClientId).toHaveBeenCalledWith('my-client-id')
		expect(statsApi.setSpotifyClientSecret).toHaveBeenCalledWith('my-client-secret')
		expect(statsApi.getSpotifyAuthUrl).toHaveBeenCalledWith('my-client-id', undefined)
	})

	it('handles Spotify callback code with client id and state', async () => {
		vi.mocked(statsApi.handleSpotifyCallback).mockResolvedValueOnce(mockSpotifyAuth)
		vi.mocked(statsApi.getStatsSummary).mockResolvedValue(mockSummary)
		vi.mocked(statsApi.getTopTracks).mockResolvedValue(mockTopTracks)
		vi.mocked(statsApi.getTopArtists).mockResolvedValue(mockTopArtists)
		vi.mocked(statsApi.getHarmonicStats).mockResolvedValue(mockHarmonicStats)
		vi.mocked(statsApi.getBpmStats).mockResolvedValue(mockBpmStats)
		vi.mocked(statsApi.getListeningHeatmap).mockResolvedValue(mockHeatmap)
		vi.mocked(statsApi.getRecentListens).mockResolvedValue([])
		vi.mocked(statsApi.getSpotifyAuthState).mockResolvedValue(mockSpotifyAuth)
		vi.mocked(statsApi.getSpotifyNowPlaying).mockResolvedValue(null)
		vi.mocked(statsApi.getRekordboxDetectStatus).mockResolvedValue(true)
		vi.mocked(statsApi.getRekordboxSessions).mockResolvedValue([])
		vi.mocked(statsApi.getMikDetectStatus).mockResolvedValue(true)

		const res = await statsStore.handleSpotifyCallback('auth_code_123', 'my-client-id', undefined, 'state_abc')

		expect(res?.is_connected).toBe(true)
		expect(statsApi.handleSpotifyCallback).toHaveBeenCalledWith('auth_code_123', 'my-client-id', undefined, 'state_abc')
		expect(get(spotifyAuth)?.user_name).toBe('Alex Crate')
	})
})
