import { describe, it, expect, beforeEach, vi } from 'vitest'
import { get } from 'svelte/store'
import { suggestionsStore, nextTrackSuggestions, suggestionsLoading, suggestionsError } from './suggestions'
import type { NextTrackSuggestion, Track } from '$shared/types'
import * as libraryApi from '$shared/api/library'

vi.mock('$shared/api/library', () => ({
	suggestNextTracks: vi.fn(),
}))

function track(id: string, title: string): Track {
	return {
		id,
		file_path: `/library/${id}.flac`,
		file_hash: null,
		title,
		artist: 'Some Artist',
		album: null,
		year: null,
		genre: null,
		label: null,
		catalog_number: null,
		duration_ms: 300_000,
		bpm: 126,
		key: '9A',
		energy: 7,
		bitrate: 1411,
		sample_rate: 44100,
		format: 'flac',
		analysis_source: null,
		waveform_data: null,
		rating: 0,
		play_count: 0,
		date_added: '2026-01-01',
		date_modified: '2026-01-01',
		last_played: null,
		rekordbox_id: null,
		artwork_path: null,
		artwork_source: null,
		color: null,
		library_root_id: null,
		relative_path: null,
		tags: [],
	}
}

const mockSuggestions: NextTrackSuggestion[] = [
	{
		track: track('trk-02', 'Undertow'),
		source: 'history',
		times_played_after: 3,
		last_played_after: '2026-09-20 22:14:00',
		harmonic: 'adjacent',
		bpm_delta_percent: 1.6,
	},
	{
		track: track('trk-03', 'Low Gravity'),
		source: 'compatible',
		times_played_after: 0,
		last_played_after: null,
		harmonic: 'same',
		bpm_delta_percent: 0.4,
	},
]

describe('suggestionsStore', () => {
	beforeEach(() => {
		vi.clearAllMocks()
		suggestionsStore.clear()
	})

	it('starts empty, not loading, with no error', () => {
		expect(get(nextTrackSuggestions)).toEqual([])
		expect(get(suggestionsLoading)).toBe(false)
		expect(get(suggestionsError)).toBeNull()
	})

	it('loads suggestions for a track', async () => {
		vi.mocked(libraryApi.suggestNextTracks).mockResolvedValueOnce(mockSuggestions)

		await suggestionsStore.load('trk-01')

		expect(libraryApi.suggestNextTracks).toHaveBeenCalledWith('trk-01')
		expect(get(nextTrackSuggestions)).toHaveLength(2)
		expect(get(nextTrackSuggestions)[0].track.title).toBe('Undertow')
		expect(get(suggestionsLoading)).toBe(false)
		expect(get(suggestionsError)).toBeNull()
	})

	it('reports an error and clears the list when the backend call fails', async () => {
		vi.mocked(libraryApi.suggestNextTracks).mockRejectedValueOnce('Track not found: trk-01')

		await suggestionsStore.load('trk-01')

		expect(get(nextTrackSuggestions)).toEqual([])
		expect(get(suggestionsLoading)).toBe(false)
		expect(get(suggestionsError)).toBe('Track not found: trk-01')
	})

	it('ignores a stale response from a track the user has since moved away from', async () => {
		let resolveFirst!: (value: NextTrackSuggestion[]) => void
		vi.mocked(libraryApi.suggestNextTracks).mockImplementationOnce(
			() => new Promise((resolve) => (resolveFirst = resolve))
		)

		const first = suggestionsStore.load('trk-01')
		vi.mocked(libraryApi.suggestNextTracks).mockResolvedValueOnce([mockSuggestions[1]])
		await suggestionsStore.load('trk-02')

		// The first (slow) request resolves after the second: it must not overwrite trk-02's result.
		resolveFirst([mockSuggestions[0]])
		await first

		expect(get(nextTrackSuggestions)).toHaveLength(1)
		expect(get(nextTrackSuggestions)[0].track.id).toBe('trk-03')
	})

	it('removes a single suggestion, e.g. after a failed play', async () => {
		vi.mocked(libraryApi.suggestNextTracks).mockResolvedValueOnce(mockSuggestions)
		await suggestionsStore.load('trk-01')

		suggestionsStore.removeSuggestion('trk-02')

		expect(get(nextTrackSuggestions)).toHaveLength(1)
		expect(get(nextTrackSuggestions)[0].track.id).toBe('trk-03')
	})

	it('clears back to the initial empty state', async () => {
		vi.mocked(libraryApi.suggestNextTracks).mockResolvedValueOnce(mockSuggestions)
		await suggestionsStore.load('trk-01')

		suggestionsStore.clear()

		expect(get(nextTrackSuggestions)).toEqual([])
		expect(get(suggestionsLoading)).toBe(false)
		expect(get(suggestionsError)).toBeNull()
	})
})
