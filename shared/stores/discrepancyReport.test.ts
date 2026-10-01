import { describe, it, expect, beforeEach, vi } from 'vitest'
import { get } from 'svelte/store'
import { discrepancyReportStore } from './discrepancyReport'
import type { DiscrepancyReport } from '../types'
import * as libraryApi from '../api/library'

vi.mock('../api/library', () => ({
	getDiscrepancyReport: vi.fn(),
}))

const emptyComparison = {
	matched: 0,
	differences: { total: 0, items: [] },
	missing_from_other: { total: 0, items: [] },
	missing_from_crate: { total: 0, items: [] },
}

const mikOnlyReport: DiscrepancyReport = {
	crate_tracks: 16,
	missing_files: {
		total: 1,
		items: [
			{
				track: { id: 'trk-05', title: 'Glasshouse', artist: 'Ilse Brandt', file_path: '/Volumes/Harness/Music/05.wav' },
				reason: 'missing',
			},
		],
	},
	mixed_in_key: {
		matched: 14,
		differences: {
			total: 1,
			items: [
				{
					track: { id: 'trk-02', title: 'Paper Lanterns', artist: 'Hollow Tide', file_path: '/x/02.mp3' },
					fields: [{ field: 'key', crate_value: '8A', other_value: '9A', note: null }],
				},
			],
		},
		missing_from_other: { total: 0, items: [] },
		missing_from_crate: {
			total: 1,
			items: [{ id: null, title: 'Only In MIK', artist: 'Someone', file_path: null }],
		},
	},
	rekordbox: null,
}

const withRekordboxReport: DiscrepancyReport = {
	...mikOnlyReport,
	rekordbox: emptyComparison,
}

describe('discrepancyReportStore', () => {
	beforeEach(() => {
		vi.clearAllMocks()
		discrepancyReportStore.reset()
	})

	it('starts with no report, not loading, no error', () => {
		const state = get(discrepancyReportStore)
		expect(state.report).toBeNull()
		expect(state.loading).toBe(false)
		expect(state.error).toBeNull()
		expect(state.rekordboxXmlPath).toBeNull()
	})

	it('loads the report without a Rekordbox export by default', async () => {
		vi.mocked(libraryApi.getDiscrepancyReport).mockResolvedValueOnce(mikOnlyReport)

		await discrepancyReportStore.load()

		expect(libraryApi.getDiscrepancyReport).toHaveBeenCalledWith(undefined)
		const state = get(discrepancyReportStore)
		expect(state.loading).toBe(false)
		expect(state.error).toBeNull()
		expect(state.report).toEqual(mikOnlyReport)
		expect(state.report?.rekordbox).toBeNull()
		expect(state.rekordboxXmlPath).toBeNull()
	})

	it('loads the Rekordbox comparison when a path is given, and remembers it', async () => {
		vi.mocked(libraryApi.getDiscrepancyReport).mockResolvedValueOnce(withRekordboxReport)

		await discrepancyReportStore.load('/Users/me/rekordbox.xml')

		expect(libraryApi.getDiscrepancyReport).toHaveBeenCalledWith('/Users/me/rekordbox.xml')
		const state = get(discrepancyReportStore)
		expect(state.report?.rekordbox).toEqual(emptyComparison)
		expect(state.rekordboxXmlPath).toBe('/Users/me/rekordbox.xml')
	})

	it('surfaces a failure and keeps it until the next load', async () => {
		vi.mocked(libraryApi.getDiscrepancyReport).mockRejectedValueOnce('mik database is locked')

		await discrepancyReportStore.load()

		const state = get(discrepancyReportStore)
		expect(state.loading).toBe(false)
		expect(state.report).toBeNull()
		expect(state.error).toBe('mik database is locked')
	})

	it('retry repeats the last load, including its Rekordbox path', async () => {
		vi.mocked(libraryApi.getDiscrepancyReport).mockResolvedValueOnce(withRekordboxReport)
		await discrepancyReportStore.load('/Users/me/rekordbox.xml')

		vi.mocked(libraryApi.getDiscrepancyReport).mockResolvedValueOnce(withRekordboxReport)
		await discrepancyReportStore.retry()

		expect(libraryApi.getDiscrepancyReport).toHaveBeenLastCalledWith('/Users/me/rekordbox.xml')
	})

	it('sets loading while the request is in flight', async () => {
		let resolveCall: (value: DiscrepancyReport) => void
		const pending = new Promise<DiscrepancyReport>((resolve) => {
			resolveCall = resolve
		})
		vi.mocked(libraryApi.getDiscrepancyReport).mockReturnValueOnce(pending)

		const loadPromise = discrepancyReportStore.load()
		expect(get(discrepancyReportStore).loading).toBe(true)

		resolveCall!(mikOnlyReport)
		await loadPromise
		expect(get(discrepancyReportStore).loading).toBe(false)
	})
})
