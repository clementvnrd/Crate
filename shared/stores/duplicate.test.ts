import { describe, it, expect, beforeEach, vi } from 'vitest'
import { get } from 'svelte/store'
import {
	duplicateStore,
	duplicateGroups,
	duplicateGroupCount,
	duplicateTrackCount,
	duplicateTotalReclaimable,
	selectedDuplicateCount,
	selectedDuplicateReclaimableBytes,
} from './duplicate'
import type { DuplicateGroup, DuplicateScanResult } from '../types'
import * as duplicateApi from '../api/duplicate'
import * as libraryApi from '../api/library'

vi.mock('../api/duplicate', () => ({
	getDuplicateGroups: vi.fn(),
	getDuplicateCount: vi.fn(),
	ignoreDuplicateGroup: vi.fn(),
	unignoreDuplicateGroup: vi.fn(),
}))

vi.mock('../api/library', () => ({
	deleteTracksAndFiles: vi.fn(),
}))

const mockScanResult: DuplicateScanResult = {
	total_groups: 2,
	total_duplicate_tracks: 2,
	total_reclaimable_bytes: 45000000,
	groups: [
		{
			id: 'group_1',
			match_type: 'metadata',
			reclaimable_bytes: 15000000,
			tracks: [
				{
					id: 't1_flac',
					title: 'Strobe',
					artist: 'deadmau5',
					album: 'For Lack of a Better Name',
					year: 2009,
					genre: 'Progressive House',
					label: 'mau5trap',
					duration_ms: 637000,
					bpm: 128,
					key: '11A',
					energy: 8,
					bitrate: 1411,
					sample_rate: 44100,
					format: 'flac',
					file_path: '/music/Strobe.flac',
					file_hash: 'hash_flac',
					file_size_bytes: 45000000,
					cue_count: 8,
					rating: 5,
					play_count: 12,
					artwork_path: '/art/1.jpg',
					date_added: '2026-01-01T00:00:00Z',
					quality_score: 1350,
					recommended_keep: true,
				},
				{
					id: 't1_mp3',
					title: 'Strobe (Original Mix)',
					artist: 'deadmau5',
					album: 'For Lack of a Better Name',
					year: 2009,
					genre: 'Progressive House',
					label: 'mau5trap',
					duration_ms: 637000,
					bpm: 128,
					key: '11A',
					energy: 8,
					bitrate: 320,
					sample_rate: 44100,
					format: 'mp3',
					file_path: '/music/Strobe.mp3',
					file_hash: 'hash_mp3',
					file_size_bytes: 15000000,
					cue_count: 2,
					rating: 0,
					play_count: 0,
					artwork_path: null,
					date_added: '2026-01-02T00:00:00Z',
					quality_score: 650,
					recommended_keep: false,
				},
			],
		},
		{
			id: 'group_2',
			match_type: 'exact_hash',
			reclaimable_bytes: 30000000,
			tracks: [
				{
					id: 't2_wav',
					title: 'One',
					artist: 'Swedish House Mafia',
					album: 'Until One',
					year: 2010,
					genre: 'Electro House',
					label: 'Astralwerks',
					duration_ms: 345000,
					bpm: 126,
					key: '4A',
					energy: 9,
					bitrate: 1411,
					sample_rate: 44100,
					format: 'wav',
					file_path: '/music/One.wav',
					file_hash: 'same_hash_123',
					file_size_bytes: 30000000,
					cue_count: 4,
					rating: 4,
					play_count: 5,
					artwork_path: null,
					date_added: '2026-01-01T00:00:00Z',
					quality_score: 1200,
					recommended_keep: true,
				},
				{
					id: 't2_wav_dup',
					title: 'One (Copy)',
					artist: 'Swedish House Mafia',
					album: 'Until One',
					year: 2010,
					genre: 'Electro House',
					label: 'Astralwerks',
					duration_ms: 345000,
					bpm: 126,
					key: '4A',
					energy: 9,
					bitrate: 1411,
					sample_rate: 44100,
					format: 'wav',
					file_path: '/downloads/One.wav',
					file_hash: 'same_hash_123',
					file_size_bytes: 30000000,
					cue_count: 0,
					rating: 0,
					play_count: 0,
					artwork_path: null,
					date_added: '2026-01-03T00:00:00Z',
					quality_score: 1100,
					recommended_keep: false,
				},
			],
		},
	],
}

describe('duplicateStore', () => {
	beforeEach(() => {
		vi.clearAllMocks()
		duplicateStore.reset()
	})

	it('initializes with empty state', () => {
		expect(get(duplicateGroupCount)).toBe(0)
		expect(get(duplicateTrackCount)).toBe(0)
		expect(get(duplicateTotalReclaimable)).toBe(0)
		expect(get(selectedDuplicateCount)).toBe(0)
		expect(get(selectedDuplicateReclaimableBytes)).toBe(0)
	})

	it('loads duplicate groups and automatically preselects non-recommended tracks for deletion', async () => {
		vi.mocked(duplicateApi.getDuplicateGroups).mockResolvedValueOnce(mockScanResult)

		await duplicateStore.load()

		expect(get(duplicateGroupCount)).toBe(2)
		expect(get(duplicateTrackCount)).toBe(2)
		expect(get(duplicateTotalReclaimable)).toBe(45000000)

		// Pre-selected tracks to delete should be the 2 non-recommended tracks
		expect(get(selectedDuplicateCount)).toBe(2)
		expect(get(selectedDuplicateReclaimableBytes)).toBe(45000000)
		expect(get(duplicateStore).selectedTrackIdsToDelete.has('t1_mp3')).toBe(true)
		expect(get(duplicateStore).selectedTrackIdsToDelete.has('t2_wav_dup')).toBe(true)
		expect(get(duplicateStore).selectedTrackIdsToDelete.has('t1_flac')).toBe(false)
		expect(get(duplicateStore).selectedTrackIdsToDelete.has('t2_wav')).toBe(false)
	})

	it('toggles track selection for deletion', async () => {
		vi.mocked(duplicateApi.getDuplicateGroups).mockResolvedValueOnce(mockScanResult)
		await duplicateStore.load()

		// Toggle uncheck t1_mp3
		duplicateStore.toggleTrackSelection('t1_mp3')
		expect(get(selectedDuplicateCount)).toBe(1)
		expect(get(selectedDuplicateReclaimableBytes)).toBe(30000000)

		// Toggle re-check t1_mp3
		duplicateStore.toggleTrackSelection('t1_mp3')
		expect(get(selectedDuplicateCount)).toBe(2)
		expect(get(selectedDuplicateReclaimableBytes)).toBe(45000000)
	})

	it('deselects and reselects all duplicates', async () => {
		vi.mocked(duplicateApi.getDuplicateGroups).mockResolvedValueOnce(mockScanResult)
		await duplicateStore.load()

		duplicateStore.deselectAll()
		expect(get(selectedDuplicateCount)).toBe(0)
		expect(get(selectedDuplicateReclaimableBytes)).toBe(0)

		duplicateStore.selectAllDuplicates()
		expect(get(selectedDuplicateCount)).toBe(2)
		expect(get(selectedDuplicateReclaimableBytes)).toBe(45000000)
	})

	it('ignores a group of duplicates', async () => {
		vi.mocked(duplicateApi.getDuplicateGroups).mockResolvedValueOnce(mockScanResult)
		vi.mocked(duplicateApi.ignoreDuplicateGroup).mockResolvedValueOnce()

		await duplicateStore.load()

		const groupToIgnore = mockScanResult.groups[0]
		await duplicateStore.ignoreGroup(groupToIgnore)

		expect(duplicateApi.ignoreDuplicateGroup).toHaveBeenCalledWith(['t1_flac', 't1_mp3'])
		expect(get(duplicateGroupCount)).toBe(1)
		expect(get(duplicateTrackCount)).toBe(1)
		expect(get(selectedDuplicateCount)).toBe(1)
	})

	it('deletes selected tracks and triggers reload', async () => {
		vi.mocked(duplicateApi.getDuplicateGroups).mockResolvedValueOnce(mockScanResult)
		vi.mocked(libraryApi.deleteTracksAndFiles).mockResolvedValueOnce()
		vi.mocked(duplicateApi.getDuplicateGroups).mockResolvedValueOnce({
			groups: [],
			total_groups: 0,
			total_duplicate_tracks: 0,
			total_reclaimable_bytes: 0,
		})

		await duplicateStore.load()

		const onDeletedMock = vi.fn()
		await duplicateStore.deleteSelected(onDeletedMock)

		expect(libraryApi.deleteTracksAndFiles).toHaveBeenCalledWith(expect.arrayContaining(['t1_mp3', 't2_wav_dup']))
		expect(onDeletedMock).toHaveBeenCalled()
		expect(get(duplicateGroupCount)).toBe(0)
	})
})
