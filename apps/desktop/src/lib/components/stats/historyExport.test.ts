import { describe, expect, it } from 'vitest'
import { historyExportName, historyExportTarget } from './historyExport'

describe('the history export target', () => {
	it('takes the format from the extension the save dialog answered', () => {
		expect(historyExportTarget('/Music/history.json')).toEqual({ format: 'json', path: '/Music/history.json' })
		expect(historyExportTarget('/Music/History.CSV')).toEqual({ format: 'csv', path: '/Music/History.CSV' })
	})

	it('adds .csv to a path without a known extension', () => {
		expect(historyExportTarget('/Music/history')).toEqual({ format: 'csv', path: '/Music/history.csv' })
	})
})

describe('the history export file name', () => {
	it('keeps the usual name for all time and names the period otherwise', () => {
		expect(historyExportName('all')).toBe('crate-listening-history.csv')
		expect(historyExportName('3m')).toBe('crate-listening-history-3m.csv')
		expect(historyExportName('year:2025')).toBe('crate-listening-history-year-2025.csv')
		expect(historyExportName('custom:2026-03-01,2026-03-31')).toBe(
			'crate-listening-history-custom-2026-03-01-2026-03-31.csv'
		)
	})
})
