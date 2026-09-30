import { describe, expect, it } from 'vitest'
import { historyExportTarget } from './historyExport'

describe('the history export target', () => {
	it('takes the format from the extension the save dialog answered', () => {
		expect(historyExportTarget('/Music/history.json')).toEqual({ format: 'json', path: '/Music/history.json' })
		expect(historyExportTarget('/Music/History.CSV')).toEqual({ format: 'csv', path: '/Music/History.CSV' })
	})

	it('adds .csv to a path without a known extension', () => {
		expect(historyExportTarget('/Music/history')).toEqual({ format: 'csv', path: '/Music/history.csv' })
	})
})
