import { describe, it, expect } from 'vitest'
import {
	calculateCoreDataTimestamp,
	parseCoreDataTimestamp,
	durationMsToSeconds,
	secondsToDurationMs,
	isCamelotKey,
	normalizeMikKey,
	isMikAnalyzed,
	isDuplicateMikTrack,
	formatMikSyncSummary,
	COREDATA_EPOCH_MS,
} from './mikSync'
import type { MikSyncResult, Track } from '../types'

describe('mikSync utils', () => {
	describe('CoreData timestamp conversions', () => {
		it('calculates 0 seconds for CoreData epoch (2001-01-01 00:00:00 UTC)', () => {
			const epochDate = new Date(COREDATA_EPOCH_MS)
			expect(calculateCoreDataTimestamp(epochDate)).toBe(0)
		})

		it('roundtrips dates through CoreData timestamp correctly', () => {
			const now = new Date('2026-08-30T14:00:00Z')
			const cdSecs = calculateCoreDataTimestamp(now)
			const parsed = parseCoreDataTimestamp(cdSecs)
			expect(parsed.toISOString()).toBe(now.toISOString())
		})
	})

	describe('duration conversions', () => {
		it('converts ms to seconds for ZTIME / ZCUEPOINT', () => {
			expect(durationMsToSeconds(0)).toBe(0)
			expect(durationMsToSeconds(185000)).toBe(185)
			expect(durationMsToSeconds(214500)).toBe(214.5)
			expect(durationMsToSeconds(-100)).toBe(0)
		})

		it('converts seconds to ms', () => {
			expect(secondsToDurationMs(0)).toBe(0)
			expect(secondsToDurationMs(185)).toBe(185000)
			expect(secondsToDurationMs(214.5)).toBe(214500)
			expect(secondsToDurationMs(-5)).toBe(0)
		})
	})

	describe('Camelot key detection and normalization', () => {
		it('identifies valid Camelot keys', () => {
			expect(isCamelotKey('11A')).toBe(true)
			expect(isCamelotKey('8B')).toBe(true)
			expect(isCamelotKey('1a')).toBe(true)
			expect(isCamelotKey('12b')).toBe(true)
			expect(isCamelotKey('4A')).toBe(true)
			expect(isCamelotKey(' 5B ')).toBe(true)
		})

		it('rejects invalid or non-Camelot musical keys', () => {
			expect(isCamelotKey('')).toBe(false)
			expect(isCamelotKey(null)).toBe(false)
			expect(isCamelotKey('13A')).toBe(false)
			expect(isCamelotKey('0B')).toBe(false)
			expect(isCamelotKey('F#m')).toBe(false)
			expect(isCamelotKey('C Major')).toBe(false)
		})

		it('normalizes Camelot keys to uppercase', () => {
			expect(normalizeMikKey('11a')).toBe('11A')
			expect(normalizeMikKey(' 8b ')).toBe('8B')
			expect(normalizeMikKey('1A')).toBe('1A')
			expect(normalizeMikKey('F#m')).toBe('F#m')
			expect(normalizeMikKey(null)).toBe(null)
			expect(normalizeMikKey('')).toBe(null)
		})
	})

	describe('isMikAnalyzed', () => {
		it('detects track as analyzed if analysis_source is mixed_in_key', () => {
			const track: Partial<Track> = {
				analysis_source: 'mixed_in_key',
				bpm: 128,
				key: '11A',
			}
			expect(isMikAnalyzed(track)).toBe(true)
		})

		it('detects track as analyzed if energy level is present', () => {
			const track: Partial<Track> = {
				analysis_source: null,
				energy: 7,
			}
			expect(isMikAnalyzed(track)).toBe(true)
		})

		it('returns false for unanalyzed tracks', () => {
			const track: Partial<Track> = {
				analysis_source: null,
				energy: null,
				bpm: 120,
			}
			expect(isMikAnalyzed(track)).toBe(false)
		})
	})

	describe('isDuplicateMikTrack', () => {
		it('detects duplicate by identical file path', () => {
			const existing = { filePath: '/Music/Track1.flac' }
			const incoming = { filePath: '/Music/Track1.flac' }
			expect(isDuplicateMikTrack(existing, incoming)).toBe(true)
		})

		it('detects duplicate by title, artist and matching duration within tolerance', () => {
			const existing = {
				title: 'Innerbloom',
				artist: 'RÜFÜS DU SOL',
				durationMs: 578000,
			}
			const incoming = {
				title: 'innerbloom',
				artist: 'rüfüs du sol',
				durationMs: 579000, // 1s difference -> duplicate
			}
			expect(isDuplicateMikTrack(existing, incoming)).toBe(true)
		})

		it('does NOT treat different versions/remixes as duplicates', () => {
			const original = {
				title: 'Innerbloom (Original Mix)',
				artist: 'RÜFÜS DU SOL',
				durationMs: 578000,
			}
			const remix = {
				title: 'Innerbloom (What So Not Remix)',
				artist: 'RÜFÜS DU SOL',
				durationMs: 275000,
			}
			expect(isDuplicateMikTrack(original, remix)).toBe(false)
		})

		it('does NOT treat tracks with different durations as duplicates even if title and artist match', () => {
			const radioEdit = {
				title: 'Strobe',
				artist: 'deadmau5',
				durationMs: 215000, // 3:35
			}
			const clubMix = {
				title: 'Strobe',
				artist: 'deadmau5',
				durationMs: 637000, // 10:37
			}
			expect(isDuplicateMikTrack(radioEdit, clubMix)).toBe(false)
		})
	})

	describe('formatMikSyncSummary', () => {
		it('formats summary with added, updated, and removed counts', () => {
			const result: MikSyncResult = {
				added: 3,
				updated: 14,
				removed: 2,
				total: 120,
				errors: [],
			}
			expect(formatMikSyncSummary(result)).toBe('Mixed In Key Sync: 3 added, 14 updated, 2 removed (120 total in MIK)')
		})

		it('formats summary when up to date', () => {
			const result: MikSyncResult = {
				added: 0,
				updated: 0,
				removed: 0,
				total: 120,
				errors: [],
			}
			expect(formatMikSyncSummary(result)).toBe('Mixed In Key Sync: Up to date (120 total in MIK)')
		})

		it('includes error count if errors occurred', () => {
			const result: MikSyncResult = {
				added: 1,
				updated: 0,
				removed: 0,
				total: 50,
				errors: ['Database locked'],
			}
			expect(formatMikSyncSummary(result)).toBe('Mixed In Key Sync: 1 added (50 total in MIK) [1 error(s)]')
		})
	})
})
