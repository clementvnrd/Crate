import type { HistoryExportFormat, TimeRange } from '$shared/types'

/**
 * File name the save dialog proposes for an export of `range`: the whole history keeps its usual name, a period adds
 * itself so two exports do not overwrite each other (`crate-listening-history-year-2025.csv`).
 */
export function historyExportName(range: TimeRange): string {
	if (range === 'all') return 'crate-listening-history.csv'
	return `crate-listening-history-${range.replace(/[:,]/g, '-')}.csv`
}

/**
 * Format and final path of a listening-history export, from the path the native save dialog answered. The
 * extension picks the format; a path without `.csv` or `.json` gets `.csv`, because the backend refuses a path
 * whose extension does not match the format.
 */
export function historyExportTarget(path: string): { format: HistoryExportFormat; path: string } {
	const lower = path.toLowerCase()
	if (lower.endsWith('.json')) return { format: 'json', path }
	if (lower.endsWith('.csv')) return { format: 'csv', path }
	return { format: 'csv', path: `${path}.csv` }
}
