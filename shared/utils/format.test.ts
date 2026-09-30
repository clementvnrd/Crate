import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import {
	formatDuration,
	formatDurationCompact,
	formatBpm,
	formatKey,
	formatEnergy,
	formatBitrate,
	formatFileSize,
	formatBytes,
	formatNumber,
	formatDate,
	daysUntilRelease,
	formatRelativeDate,
	getTrackDisplayName,
	getTrackDisplayArtist,
} from './format'

describe('format utils', () => {
	describe('formatNumber', () => {
		// Intl separates French thousands with a narrow no-break space; compare on a plain space.
		const plain = (text: string) => text.replace(/\s/g, ' ')

		it('groups digits the way the interface language does, not the system one', () => {
			expect(formatNumber(2310, 'en')).toBe('2,310')
			expect(plain(formatNumber(2310, 'fr'))).toBe('2 310')
			expect(formatNumber(2310, 'de')).toBe('2.310')
		})

		it('keeps small numbers and zero untouched', () => {
			expect(formatNumber(0, 'fr')).toBe('0')
			expect(formatNumber(999, 'fr')).toBe('999')
		})

		it('uses the decimal comma in French', () => {
			expect(formatNumber(12.5, 'fr')).toBe('12,5')
		})
	})

	describe('formatDuration', () => {
		it('formats milliseconds to MM:SS when under an hour', () => {
			expect(formatDuration(0)).toBe('0:00')
			expect(formatDuration(5000)).toBe('0:05')
			expect(formatDuration(59000)).toBe('0:59')
			expect(formatDuration(60000)).toBe('1:00')
			expect(formatDuration(65000)).toBe('1:05')
			expect(formatDuration(215000)).toBe('3:35')
			expect(formatDuration(3599000)).toBe('59:59')
		})

		it('formats milliseconds to HH:MM:SS when an hour or more', () => {
			expect(formatDuration(3600000)).toBe('1:00:00')
			expect(formatDuration(3661000)).toBe('1:01:01')
			expect(formatDuration(7325000)).toBe('2:02:05')
			expect(formatDuration(36000000)).toBe('10:00:00')
		})
	})

	describe('formatDurationCompact', () => {
		it('formats durations in compact M:SS format without hour separator', () => {
			expect(formatDurationCompact(0)).toBe('0:00')
			expect(formatDurationCompact(65000)).toBe('1:05')
			expect(formatDurationCompact(215000)).toBe('3:35')
			expect(formatDurationCompact(3661000)).toBe('61:01')
			expect(formatDurationCompact(7325000)).toBe('122:05')
		})
	})

	describe('formatBpm', () => {
		it('returns "-" for null', () => {
			expect(formatBpm(null)).toBe('-')
		})

		it('formats BPM to 1 decimal place', () => {
			expect(formatBpm(128)).toBe('128.0')
			expect(formatBpm(128.456)).toBe('128.5')
			expect(formatBpm(124.0)).toBe('124.0')
			expect(formatBpm(0)).toBe('0.0')
		})
	})

	describe('formatKey', () => {
		it('returns "-" for null, empty or undefined key', () => {
			expect(formatKey(null)).toBe('-')
			expect(formatKey('')).toBe('-')
			// @ts-expect-error test invalid argument
			expect(formatKey(undefined)).toBe('-')
		})

		it('converts standard notation to camelot by default', () => {
			expect(formatKey('Am')).toBe('8A')
			expect(formatKey('C')).toBe('8B')
			expect(formatKey('F#m')).toBe('11A')
			expect(formatKey('Gb')).toBe('2B')
			expect(formatKey('G#')).toBe('4B')
			expect(formatKey('D#m')).toBe('2A')
			expect(formatKey('8A')).toBe('8A')
			expect(formatKey('12B')).toBe('12B')
		})

		it('converts camelot notation to standard when format is "standard"', () => {
			expect(formatKey('8A', 'standard')).toBe('Am')
			expect(formatKey('8B', 'standard')).toBe('C')
			expect(formatKey('1A', 'standard')).toBe('Abm')
			expect(formatKey('11A', 'standard')).toBe('F#m')
			expect(formatKey('12B', 'standard')).toBe('E')
			expect(formatKey('8a', 'standard')).toBe('Am')
		})

		it('preserves clean key if mapping is not found', () => {
			expect(formatKey('UnknownKey', 'camelot')).toBe('UnknownKey')
			expect(formatKey('UnknownKey', 'standard')).toBe('UnknownKey')
			expect(formatKey('Am', 'standard')).toBe('Am')
		})
	})

	describe('formatEnergy', () => {
		it('returns "-" for null or undefined', () => {
			expect(formatEnergy(null)).toBe('-')
			expect(formatEnergy(undefined)).toBe('-')
		})

		it('formats numeric energy as string', () => {
			expect(formatEnergy(1)).toBe('1')
			expect(formatEnergy(7)).toBe('7')
			expect(formatEnergy(10)).toBe('10')
			expect(formatEnergy(0)).toBe('0')
		})
	})

	describe('formatBitrate', () => {
		it('handles null, undefined or zero bitrate for compressed files', () => {
			expect(formatBitrate(null)).toBe('-')
			expect(formatBitrate(undefined)).toBe('-')
			expect(formatBitrate(0)).toBe('-')
			expect(formatBitrate(null, 'mp3')).toBe('MP3')
			expect(formatBitrate(0, 'aac')).toBe('AAC')
		})

		it('computes PCM bitrate for wav/aiff when bitrate is missing or <= 10', () => {
			// default sampleRate: 44100 -> (44100 * 2 * 24) / 1000 = 2116.8 -> 2117 kbps
			expect(formatBitrate(null, 'wav')).toBe('2117 kbps')
			expect(formatBitrate(0, 'aiff')).toBe('2117 kbps')
			expect(formatBitrate(null, 'wav', 48000)).toBe('2304 kbps')
			expect(formatBitrate(5, 'wav', 44100)).toBe('2117 kbps')
			expect(formatBitrate(5, 'flac', 96000)).toBe('4608 kbps')
		})

		it('converts bps (> 10000) to kbps', () => {
			expect(formatBitrate(320000)).toBe('320 kbps')
			expect(formatBitrate(1411200)).toBe('1411 kbps')
		})

		it('formats already normalized kbps directly', () => {
			expect(formatBitrate(320)).toBe('320 kbps')
			expect(formatBitrate(256)).toBe('256 kbps')
			expect(formatBitrate(1411)).toBe('1411 kbps')
		})
	})

	describe('formatFileSize', () => {
		it('formats 0 bytes', () => {
			expect(formatFileSize(0)).toBe('0 B')
		})

		it('formats byte units (B, KB, MB, GB)', () => {
			expect(formatFileSize(500)).toBe('500 B')
			expect(formatFileSize(1024)).toBe('1 KB')
			expect(formatFileSize(1536)).toBe('1.5 KB')
			expect(formatFileSize(1048576)).toBe('1 MB')
			expect(formatFileSize(1048576 * 15.5)).toBe('15.5 MB')
			expect(formatFileSize(1073741824)).toBe('1 GB')
		})
	})

	describe('formatBytes', () => {
		it('handles null and undefined', () => {
			expect(formatBytes(null)).toBe('-')
			expect(formatBytes(undefined)).toBe('-')
		})

		it('formats 0 bytes', () => {
			expect(formatBytes(0)).toBe('0 B')
		})

		it('formats units up to TB with 2 decimal places', () => {
			expect(formatBytes(500)).toBe('500 B')
			expect(formatBytes(1024)).toBe('1 KB')
			expect(formatBytes(1536)).toBe('1.5 KB')
			expect(formatBytes(1048576 * 2.25)).toBe('2.25 MB')
			expect(formatBytes(1073741824 * 3.5)).toBe('3.5 GB')
			expect(formatBytes(1099511627776)).toBe('1 TB')
		})
	})

	describe('formatDate', () => {
		it('returns year-only unchanged', () => {
			expect(formatDate('2024')).toBe('2024')
			expect(formatDate('1999')).toBe('1999')
		})

		it('formats year-month strings correctly', () => {
			expect(formatDate('2024-06', 'iso')).toBe('2024-06')
			expect(formatDate('2024-06', 'us')).toBe('06/2024')
			expect(formatDate('2024-06', 'eu')).toBe('06.2024')
			expect(formatDate('2024-06', 'dot')).toBe('06.2024')
			expect(formatDate('2024-06', 'locale', 'en-US')).toBe('06/2024')
		})

		it('formats date-only (YYYY-MM-DD) strings with different formats without timezone shift', () => {
			expect(formatDate('2024-03-15', 'iso')).toBe('2024-03-15')
			expect(formatDate('2024-03-15', 'us')).toBe('03/15/2024')
			expect(formatDate('2024-03-15', 'eu')).toBe('15/03/2024')
			expect(formatDate('2024-03-15', 'dot')).toBe('15.03.2024')
			expect(formatDate('2024-03-15', 'locale', 'en-US')).toBe('3/15/2024')
		})

		it('formats full ISO timestamp strings', () => {
			const isoString = '2024-07-20T12:00:00Z'
			expect(formatDate(isoString, 'iso')).toMatch(/^2024-\d{2}-\d{2}$/)
			expect(formatDate(isoString, 'us')).toMatch(/^\d{2}\/\d{2}\/2024$/)
			expect(formatDate(isoString, 'eu')).toMatch(/^\d{2}\/\d{2}\/2024$/)
			expect(formatDate(isoString, 'dot')).toMatch(/^\d{2}\.\d{2}\.2024$/)
		})
	})

	describe('daysUntilRelease', () => {
		beforeEach(() => {
			vi.useFakeTimers()
			vi.setSystemTime(new Date('2026-08-30T12:00:00Z'))
		})

		afterEach(() => {
			vi.useRealTimers()
		})

		it('returns null for null, empty or non-date strings', () => {
			expect(daysUntilRelease(null)).toBeNull()
			expect(daysUntilRelease('')).toBeNull()
			expect(daysUntilRelease('2026')).toBeNull()
			expect(daysUntilRelease('2026-08')).toBeNull()
			expect(daysUntilRelease('invalid-date')).toBeNull()
		})

		it('returns null for dates in the past or today', () => {
			expect(daysUntilRelease('2026-08-29')).toBeNull()
			expect(daysUntilRelease('2026-08-30')).toBeNull()
			expect(daysUntilRelease('2025-01-01')).toBeNull()
		})

		it('returns days count for future dates', () => {
			expect(daysUntilRelease('2026-08-31')).toBe(1)
			expect(daysUntilRelease('2026-09-04')).toBe(5)
			expect(daysUntilRelease('2026-09-30')).toBe(31)
		})
	})

	describe('formatRelativeDate', () => {
		beforeEach(() => {
			vi.useFakeTimers()
			vi.setSystemTime(new Date('2026-08-30T12:00:00Z'))
		})

		afterEach(() => {
			vi.useRealTimers()
		})

		const mockTranslate = vi.fn((key: string, opts?: { values?: Record<string, unknown> }) => {
			if (key === 'dates.today') return 'Today'
			if (key === 'dates.yesterday') return 'Yesterday'
			if (key === 'dates.daysAgo') return `${opts?.values?.count} days ago`
			if (key === 'dates.weeksAgo') return `${opts?.values?.count} weeks ago`
			if (key === 'dates.monthsAgo') return `${opts?.values?.count} months ago`
			if (key === 'dates.yearsAgo') return `${opts?.values?.count} years ago`
			return key
		})

		it('formats today and yesterday correctly', () => {
			expect(formatRelativeDate('2026-08-30T10:00:00Z', mockTranslate)).toBe('Today')
			expect(formatRelativeDate('2026-08-29T10:00:00Z', mockTranslate)).toBe('Yesterday')
		})

		it('formats days ago (< 7 days)', () => {
			expect(formatRelativeDate('2026-08-26T12:00:00Z', mockTranslate)).toBe('4 days ago')
			expect(formatRelativeDate('2026-08-24T12:00:00Z', mockTranslate)).toBe('6 days ago')
		})

		it('formats weeks ago (7-29 days)', () => {
			expect(formatRelativeDate('2026-08-16T12:00:00Z', mockTranslate)).toBe('2 weeks ago')
			expect(formatRelativeDate('2026-08-09T12:00:00Z', mockTranslate)).toBe('3 weeks ago')
		})

		it('formats months ago (30-364 days)', () => {
			expect(formatRelativeDate('2026-06-30T12:00:00Z', mockTranslate)).toBe('2 months ago')
			expect(formatRelativeDate('2026-01-01T12:00:00Z', mockTranslate)).toBe('8 months ago')
		})

		it('formats years ago (>= 365 days)', () => {
			expect(formatRelativeDate('2025-08-01T12:00:00Z', mockTranslate)).toBe('1 years ago')
			expect(formatRelativeDate('2023-08-01T12:00:00Z', mockTranslate)).toBe('3 years ago')
		})
	})

	describe('getTrackDisplayName', () => {
		it('returns track title when available', () => {
			expect(getTrackDisplayName({ title: 'Strobe', file_path: '/music/track01.mp3' })).toBe('Strobe')
		})

		it('extracts filename without extension when title is null or empty', () => {
			expect(getTrackDisplayName({ title: null, file_path: '/music/subfolder/track01.mp3' })).toBe('track01')
			expect(getTrackDisplayName({ title: '', file_path: 'C:\\Music\\Folder\\MySong.flac' })).toBe('MySong')
			expect(getTrackDisplayName({ title: null, file_path: 'audio.wav' })).toBe('audio')
		})
	})

	describe('getTrackDisplayArtist', () => {
		it('returns artist name when available', () => {
			expect(getTrackDisplayArtist({ artist: 'deadmau5' })).toBe('deadmau5')
		})

		it('returns "Unknown Artist" when artist is null or empty', () => {
			expect(getTrackDisplayArtist({ artist: null })).toBe('Unknown Artist')
			expect(getTrackDisplayArtist({ artist: '' })).toBe('Unknown Artist')
		})
	})
})
