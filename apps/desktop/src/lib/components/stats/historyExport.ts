import type { HistoryExportFormat } from '$shared/types'

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
