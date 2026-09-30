// Fixed reference moment for every fixture: data never depends on the real clock, so two runs are identical.

export const REFERENCE_NOW = '2026-09-28T18:30:00.000Z'

const REFERENCE_MS = Date.parse(REFERENCE_NOW)

/** ISO timestamp `days` days (and `hours` hours) before the reference moment. */
export function isoAgo(days: number, hours = 0): string {
	return new Date(REFERENCE_MS - days * 86_400_000 - hours * 3_600_000).toISOString()
}

/** `m:ss` label of a duration in milliseconds. */
export function formatClock(durationMs: number): string {
	const totalSeconds = Math.round(durationMs / 1000)
	const seconds = totalSeconds % 60
	return `${Math.floor(totalSeconds / 60)}:${seconds.toString().padStart(2, '0')}`
}
