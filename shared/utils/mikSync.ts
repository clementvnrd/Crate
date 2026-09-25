import type { MikSyncResult, Track } from '../types'

/** CoreData reference epoch: January 1, 2001, 00:00:00 UTC (in milliseconds) */
export const COREDATA_EPOCH_OFFSET_SECS = 978307200
export const COREDATA_EPOCH_MS = Date.UTC(2001, 0, 1, 0, 0, 0)

/**
 * Converts a JS Date into Apple CoreData timestamp (seconds since 2001-01-01 00:00:00 UTC)
 */
export function calculateCoreDataTimestamp(date: Date = new Date()): number {
	return (date.getTime() - COREDATA_EPOCH_MS) / 1000
}

/**
 * Converts an Apple CoreData timestamp (seconds since 2001-01-01) into a JS Date
 */
export function parseCoreDataTimestamp(coreDataSeconds: number): Date {
	return new Date(COREDATA_EPOCH_MS + coreDataSeconds * 1000)
}

/**
 * Converts milliseconds duration to seconds (as used in MIK ZTIME / ZCUEPOINT)
 */
export function durationMsToSeconds(durationMs: number): number {
	if (!durationMs || durationMs < 0) return 0
	return durationMs / 1000
}

/**
 * Converts seconds duration to milliseconds
 */
export function secondsToDurationMs(seconds: number): number {
	if (!seconds || seconds < 0) return 0
	return Math.round(seconds * 1000)
}

/**
 * Validates if a key string conforms to Camelot notation (1A-12A, 1B-12B)
 */
export function isCamelotKey(key?: string | null): boolean {
	if (!key) return false
	const clean = key.trim()
	return /^(1[0-2]|[1-9])[ABab]$/.test(clean)
}

/**
 * Normalizes a key string to standard Camelot uppercase notation
 */
export function normalizeMikKey(key?: string | null): string | null {
	if (!key) return null
	const clean = key.trim()
	if (!clean) return null
	const match = clean.match(/^(1[0-2]|[1-9])([ABab])$/)
	if (match) {
		return `${match[1]}${match[2].toUpperCase()}`
	}
	return clean
}

/**
 * Checks whether a track is considered analyzed by Mixed In Key
 */
export function isMikAnalyzed(
	track: Partial<Track> | { analysis_source?: string | null; energy?: number | null; bpm?: number | null }
): boolean {
	if (!track) return false
	return track.analysis_source === 'mixed_in_key' || (track.energy !== undefined && track.energy !== null && track.energy > 0)
}

/**
 * Determines if an incoming track is a duplicate of an existing MIK track:
 * - Direct match by file path
 * - Or match by title & artist (with duration tolerance), preserving different versions/remixes
 */
export function isDuplicateMikTrack(
	existing: {
		title?: string | null
		artist?: string | null
		durationMs?: number | null
		filePath?: string | null
	},
	incoming: {
		title?: string | null
		artist?: string | null
		durationMs?: number | null
		filePath?: string | null
	},
	durationToleranceSecs: number = 3
): boolean {
	// 1. Direct path match
	if (existing.filePath && incoming.filePath) {
		if (existing.filePath.trim().toLowerCase() === incoming.filePath.trim().toLowerCase()) {
			return true
		}
	}

	// 2. Metadata match by title & artist
	const exTitle = existing.title?.trim().toLowerCase()
	const inTitle = incoming.title?.trim().toLowerCase()
	const exArtist = existing.artist?.trim().toLowerCase()
	const inArtist = incoming.artist?.trim().toLowerCase()

	if (exTitle && inTitle && exArtist && inArtist && exTitle === inTitle && exArtist === inArtist) {
		// If both have durations, check if they match within tolerance
		if (
			existing.durationMs !== undefined &&
			existing.durationMs !== null &&
			existing.durationMs > 0 &&
			incoming.durationMs !== undefined &&
			incoming.durationMs !== null &&
			incoming.durationMs > 0
		) {
			const diffSecs = Math.abs(existing.durationMs - incoming.durationMs) / 1000
			return diffSecs <= durationToleranceSecs
		}
		// If durations are missing, treat matching title & artist as duplicate
		return true
	}

	return false
}

/**
 * Formats a user-friendly summary string of a Mixed In Key sync operation
 */
export function formatMikSyncSummary(result: MikSyncResult): string {
	const parts: string[] = []
	if (result.added > 0) {
		parts.push(`${result.added} added`)
	}
	if (result.updated > 0) {
		parts.push(`${result.updated} updated`)
	}
	if (result.removed > 0) {
		parts.push(`${result.removed} removed`)
	}
	if (parts.length === 0) {
		parts.push('Up to date')
	}

	let summary = `Mixed In Key Sync: ${parts.join(', ')} (${result.total} total in MIK)`
	if (result.errors && result.errors.length > 0) {
		summary += ` [${result.errors.length} error(s)]`
	}
	return summary
}
