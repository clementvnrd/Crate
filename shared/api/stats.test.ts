import { beforeEach, describe, expect, it, vi } from 'vitest'

const invoke = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => invoke(...args) }))

import { customRange, yearRange } from '../utils/statsRange'
import {
	exportListeningHistory,
	getBpmStats,
	getHarmonicStats,
	getListeningHeatmap,
	getListeningYears,
	getStatsSummary,
	getTopArtists,
	getTopTracks,
} from './stats'

// The six Pulse queries send the period as `timeRange` (camelCase, Tauri turns it into the Rust
// `time_range`); the new range forms travel through the same key, unchanged.

describe('the Pulse queries and the typed range', () => {
	beforeEach(() => invoke.mockReset().mockResolvedValue(undefined))

	it('default to the last 7 days', async () => {
		await getStatsSummary()
		expect(invoke).toHaveBeenCalledWith('get_stats_summary', { timeRange: '7d' })
	})

	it('send rolling months, calendar years and custom windows as they are', async () => {
		const custom = customRange('2026-03-01', '2026-03-31')
		await getStatsSummary('3m')
		await getTopTracks(yearRange(2025), 10)
		await getTopArtists(custom, 5)
		await getHarmonicStats('6m')
		await getBpmStats(custom)
		await getListeningHeatmap(yearRange(2024))
		expect(invoke.mock.calls).toEqual([
			['get_stats_summary', { timeRange: '3m' }],
			['get_top_tracks', { timeRange: 'year:2025', limit: 10 }],
			['get_top_artists', { timeRange: 'custom:2026-03-01,2026-03-31', limit: 5 }],
			['get_harmonic_stats', { timeRange: '6m' }],
			['get_bpm_stats', { timeRange: 'custom:2026-03-01,2026-03-31' }],
			['get_listening_heatmap', { timeRange: 'year:2024' }],
		])
	})

	it('pass the backend rejection (a string) through', async () => {
		const message = 'Invalid operation: invalid statistics range "forever"'
		invoke.mockRejectedValueOnce(message)
		const rejection = await getStatsSummary('forever' as never).then(
			() => undefined,
			(error: unknown) => error
		)
		expect(rejection).toBe(message)
	})
})

describe('the years with listening data', () => {
	beforeEach(() => invoke.mockReset())

	it('asks the backend without arguments and returns its list as is', async () => {
		invoke.mockResolvedValueOnce([2026, 2024, 2022])
		await expect(getListeningYears()).resolves.toEqual([2026, 2024, 2022])
		expect(invoke).toHaveBeenCalledWith('get_listening_years')
	})
})

describe('the history export', () => {
	beforeEach(() => invoke.mockReset().mockResolvedValue(3))

	it('sends the selected period as `timeRange`, the whole history by default', async () => {
		await exportListeningHistory('csv', '/Music/history.csv', yearRange(2025))
		expect(invoke).toHaveBeenCalledWith('export_listening_history', {
			format: 'csv',
			path: '/Music/history.csv',
			timeRange: 'year:2025',
		})
		await exportListeningHistory('json', '/Music/history.json')
		expect(invoke).toHaveBeenLastCalledWith('export_listening_history', {
			format: 'json',
			path: '/Music/history.json',
			timeRange: 'all',
		})
	})
})
