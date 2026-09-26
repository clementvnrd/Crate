import { describe, it, expect, beforeEach, vi } from 'vitest'
import { get } from 'svelte/store'
import {
	upgraderStore,
	upgraderMatches,
	upgraderMatchCount,
	upgraderEligibleCount,
	selectedUpgradeCount,
} from './upgrader'
import type { UpgradeCountInfo, UpgradeScanResult } from '../types'
import * as upgraderApi from '../api/upgrader'

vi.mock('../api/upgrader', () => ({
	getUpgradeMatches: vi.fn(),
	getUpgradeCount: vi.fn(),
	ignoreUpgradeMatch: vi.fn(),
	unignoreUpgradeMatch: vi.fn(),
	executeUpgradeReplacements: vi.fn(),
}))

const mockScanResult: UpgradeScanResult = {
	total_scanned: 5,
	total_eligible_mp3s: 5,
	potential_upgrades_count: 2,
	matches: [
		{
			track_id: 't_mp3_1',
			file_path: '/music/Eric Prydz - Opus.mp3',
			current_format: 'mp3',
			current_bitrate: 320,
			current_sample_rate: 44100,
			current_duration_ms: 558000,
			current_bpm: 126,
			current_key: '10B',
			current_energy: 8,
			current_artwork_path: null,
			current_file_size_bytes: 14000000,
			title: 'Opus (Original Mix)',
			artist: 'Eric Prydz',
			album: 'Opus',
			confidence_score: 98,
			score_breakdown: {
				title_score: 40,
				artist_score: 30,
				duration_score: 15,
				bpm_score: 10,
				key_score: 3,
			},
			beatport_track: {
				id: 'bp_123',
				title: 'Opus',
				mix_name: 'Original Mix',
				artists: [{ id: 1, name: 'Eric Prydz' }],
				genre: 'Progressive House',
				release_name: 'Opus',
				release_date: '2015-07-27',
				duration_ms: 558000,
				duration_formatted: '9:18',
				bpm: 126,
				key: '10B',
				artwork_url: 'https://example.com/opus.jpg',
				preview_url: 'https://example.com/opus-preview.mp3',
			},
		},
		{
			track_id: 't_mp3_2',
			file_path: '/music/deadmau5 - Strobe.mp3',
			current_format: 'mp3',
			current_bitrate: 256,
			current_sample_rate: 44100,
			current_duration_ms: 637000,
			current_bpm: 128,
			current_key: '11A',
			current_energy: 7,
			current_artwork_path: null,
			current_file_size_bytes: 12000000,
			title: 'Strobe',
			artist: 'deadmau5',
			album: 'For Lack of a Better Name',
			confidence_score: 95,
			score_breakdown: {
				title_score: 40,
				artist_score: 30,
				duration_score: 15,
				bpm_score: 8,
				key_score: 2,
			},
			beatport_track: {
				id: 'bp_456',
				title: 'Strobe',
				mix_name: 'Original Mix',
				artists: [{ id: 2, name: 'deadmau5' }],
				genre: 'Progressive House',
				release_name: 'For Lack of a Better Name',
				release_date: '2009-09-22',
				duration_ms: 637000,
				duration_formatted: '10:37',
				bpm: 128,
				key: '11A',
				artwork_url: 'https://example.com/strobe.jpg',
				preview_url: 'https://example.com/strobe-preview.mp3',
			},
		},
	],
}

describe('upgraderStore', () => {
	beforeEach(() => {
		vi.clearAllMocks()
		upgraderStore.reset()
	})

	it('initializes with empty state', () => {
		expect(get(upgraderMatchCount)).toBe(0)
		expect(get(upgraderEligibleCount)).toBe(0)
		expect(get(selectedUpgradeCount)).toBe(0)
		expect(get(upgraderMatches)).toEqual([])
	})

	it('loads upgrade matches and automatically preselects all matches for upgrade', async () => {
		vi.mocked(upgraderApi.getUpgradeMatches).mockResolvedValueOnce(mockScanResult)

		await upgraderStore.load()

		expect(get(upgraderMatchCount)).toBe(2)
		expect(get(upgraderEligibleCount)).toBe(5)
		expect(get(selectedUpgradeCount)).toBe(2)
		expect(get(upgraderStore).selectedMatchTrackIds.has('t_mp3_1')).toBe(true)
		expect(get(upgraderStore).selectedMatchTrackIds.has('t_mp3_2')).toBe(true)
	})

	it('toggles match selection for upgrade', async () => {
		vi.mocked(upgraderApi.getUpgradeMatches).mockResolvedValueOnce(mockScanResult)
		await upgraderStore.load()

		// Toggle uncheck t_mp3_1
		upgraderStore.toggleMatchSelection('t_mp3_1')
		expect(get(selectedUpgradeCount)).toBe(1)
		expect(get(upgraderStore).selectedMatchTrackIds.has('t_mp3_1')).toBe(false)

		// Toggle re-check t_mp3_1
		upgraderStore.toggleMatchSelection('t_mp3_1')
		expect(get(selectedUpgradeCount)).toBe(2)
		expect(get(upgraderStore).selectedMatchTrackIds.has('t_mp3_1')).toBe(true)
	})

	it('deselects and selects all matches', async () => {
		vi.mocked(upgraderApi.getUpgradeMatches).mockResolvedValueOnce(mockScanResult)
		await upgraderStore.load()

		upgraderStore.deselectAll()
		expect(get(selectedUpgradeCount)).toBe(0)

		upgraderStore.selectAll()
		expect(get(selectedUpgradeCount)).toBe(2)
	})

	it('ignores a match', async () => {
		vi.mocked(upgraderApi.getUpgradeMatches).mockResolvedValueOnce(mockScanResult)
		vi.mocked(upgraderApi.ignoreUpgradeMatch).mockResolvedValueOnce()

		await upgraderStore.load()

		const matchToIgnore = mockScanResult.matches[0]
		await upgraderStore.ignoreMatch(matchToIgnore)

		expect(upgraderApi.ignoreUpgradeMatch).toHaveBeenCalledWith('t_mp3_1', 'bp_123')
		expect(get(upgraderMatchCount)).toBe(1)
		expect(get(selectedUpgradeCount)).toBe(1)
	})

	it('executes upgrade replacements on selected tracks and triggers reload', async () => {
		vi.mocked(upgraderApi.getUpgradeMatches).mockResolvedValueOnce(mockScanResult)
		vi.mocked(upgraderApi.executeUpgradeReplacements).mockResolvedValueOnce({
			success_count: 2,
			failed_count: 0,
			replaced_tracks: ['Opus (Original Mix)', 'Strobe'],
			errors: [],
		})
		vi.mocked(upgraderApi.getUpgradeMatches).mockResolvedValueOnce({
			matches: [],
			total_scanned: 5,
			total_eligible_mp3s: 3,
			potential_upgrades_count: 0,
		})

		await upgraderStore.load()

		const onUpgradedMock = vi.fn()
		const res = await upgraderStore.executeSelected(onUpgradedMock)

		expect(res?.success_count).toBe(2)
		expect(upgraderApi.executeUpgradeReplacements).toHaveBeenCalledWith(mockScanResult.matches)
		expect(onUpgradedMock).toHaveBeenCalled()
		expect(get(upgraderMatchCount)).toBe(0)
	})

	it('loads count with deduplication', async () => {
		let resolveFirst: (value: UpgradeCountInfo) => void
		const firstPromise = new Promise<UpgradeCountInfo>((resolve) => {
			resolveFirst = resolve
		})
		vi.mocked(upgraderApi.getUpgradeCount).mockReturnValueOnce(firstPromise)

		const call1 = upgraderStore.loadCount()
		const call2 = upgraderStore.loadCount() // concurrent call should be deduplicated

		expect(upgraderApi.getUpgradeCount).toHaveBeenCalledTimes(1)
		expect(await call2).toBeNull()

		resolveFirst!({ match_count: 3, eligible_mp3_count: 7 })
		const res1 = await call1
		expect(res1?.match_count).toBe(3)
		expect(get(upgraderMatchCount)).toBe(3)
		expect(get(upgraderEligibleCount)).toBe(7)
	})
})
