import { describe, it, expect } from 'vitest'
import { CAMELOT_COLORS, getCamelotColor, formatCamelotKey, getHarmonicKeys, isHarmonicallyCompatible } from './camelot'

describe('camelot utils', () => {
	describe('CAMELOT_COLORS', () => {
		it('contains all 24 Camelot keys (1A-12A and 1B-12B)', () => {
			for (let i = 1; i <= 12; i++) {
				const aKey = `${i}A`
				const bKey = `${i}B`
				expect(CAMELOT_COLORS[aKey]).toBeDefined()
				expect(CAMELOT_COLORS[bKey]).toBeDefined()
				expect(CAMELOT_COLORS[aKey].bg).toMatch(/^#[0-9A-Fa-f]{6}$/)
				expect(CAMELOT_COLORS[aKey].text).toMatch(/^#[0-9A-Fa-f]{6}$/)
				expect(CAMELOT_COLORS[aKey].border).toMatch(/^#[0-9A-Fa-f]{6}$/)
				expect(CAMELOT_COLORS[aKey].dot).toMatch(/^#[0-9A-Fa-f]{6}$/)
				expect(typeof CAMELOT_COLORS[aKey].name).toBe('string')
			}
			expect(Object.keys(CAMELOT_COLORS)).toHaveLength(24)
		})
	})

	describe('getCamelotColor', () => {
		it('returns null for null, undefined, or empty string', () => {
			expect(getCamelotColor(null)).toBeNull()
			expect(getCamelotColor(undefined)).toBeNull()
			expect(getCamelotColor('')).toBeNull()
			expect(getCamelotColor('   ')).toBeNull()
		})

		it('returns color info for Camelot notation keys', () => {
			const color8A = getCamelotColor('8A')
			expect(color8A).toBeDefined()
			expect(color8A?.name).toBe('A minor')
			expect(color8A?.bg).toBe('#ECA0F3')

			const color8B = getCamelotColor('8B')
			expect(color8B).toBeDefined()
			expect(color8B?.name).toBe('C major')
			expect(color8B?.bg).toBe('#DB57E6')

			const color1A = getCamelotColor('1A')
			expect(color1A?.name).toBe('Ab minor / G# minor')

			const color12B = getCamelotColor('12B')
			expect(color12B?.name).toBe('E major')
		})

		it('handles case-insensitivity and whitespace in Camelot notation', () => {
			expect(getCamelotColor('8a')).toEqual(CAMELOT_COLORS['8A'])
			expect(getCamelotColor(' 8A ')).toEqual(CAMELOT_COLORS['8A'])
			expect(getCamelotColor('11b')).toEqual(CAMELOT_COLORS['11B'])
		})

		it('resolves standard musical notation keys correctly', () => {
			// Minor keys
			expect(getCamelotColor('Am')).toEqual(CAMELOT_COLORS['8A'])
			expect(getCamelotColor('am')).toEqual(CAMELOT_COLORS['8A'])
			expect(getCamelotColor('Em')).toEqual(CAMELOT_COLORS['9A'])
			expect(getCamelotColor('Bm')).toEqual(CAMELOT_COLORS['10A'])
			expect(getCamelotColor('F#m')).toEqual(CAMELOT_COLORS['11A'])
			expect(getCamelotColor('Gbm')).toEqual(CAMELOT_COLORS['11A'])
			expect(getCamelotColor('C#m')).toEqual(CAMELOT_COLORS['12A'])
			expect(getCamelotColor('Dbm')).toEqual(CAMELOT_COLORS['12A'])
			expect(getCamelotColor('Abm')).toEqual(CAMELOT_COLORS['1A'])
			expect(getCamelotColor('G#m')).toEqual(CAMELOT_COLORS['1A'])
			expect(getCamelotColor('Ebm')).toEqual(CAMELOT_COLORS['2A'])
			expect(getCamelotColor('D#m')).toEqual(CAMELOT_COLORS['2A'])
			expect(getCamelotColor('Bbm')).toEqual(CAMELOT_COLORS['3A'])
			expect(getCamelotColor('A#m')).toEqual(CAMELOT_COLORS['3A'])
			expect(getCamelotColor('Fm')).toEqual(CAMELOT_COLORS['4A'])
			expect(getCamelotColor('Cm')).toEqual(CAMELOT_COLORS['5A'])
			expect(getCamelotColor('Gm')).toEqual(CAMELOT_COLORS['6A'])
			expect(getCamelotColor('Dm')).toEqual(CAMELOT_COLORS['7A'])

			// Major keys
			expect(getCamelotColor('C')).toEqual(CAMELOT_COLORS['8B'])
			expect(getCamelotColor('G')).toEqual(CAMELOT_COLORS['9B'])
			expect(getCamelotColor('D')).toEqual(CAMELOT_COLORS['10B'])
			expect(getCamelotColor('A')).toEqual(CAMELOT_COLORS['11B'])
			expect(getCamelotColor('E')).toEqual(CAMELOT_COLORS['12B'])
			expect(getCamelotColor('B')).toEqual(CAMELOT_COLORS['1B'])
			expect(getCamelotColor('F#')).toEqual(CAMELOT_COLORS['2B'])
			expect(getCamelotColor('Gb')).toEqual(CAMELOT_COLORS['2B'])
			expect(getCamelotColor('Db')).toEqual(CAMELOT_COLORS['3B'])
			expect(getCamelotColor('C#')).toEqual(CAMELOT_COLORS['3B'])
			expect(getCamelotColor('Ab')).toEqual(CAMELOT_COLORS['4B'])
			expect(getCamelotColor('G#')).toEqual(CAMELOT_COLORS['4B'])
			expect(getCamelotColor('Eb')).toEqual(CAMELOT_COLORS['5B'])
			expect(getCamelotColor('D#')).toEqual(CAMELOT_COLORS['5B'])
			expect(getCamelotColor('Bb')).toEqual(CAMELOT_COLORS['6B'])
			expect(getCamelotColor('A#')).toEqual(CAMELOT_COLORS['6B'])
			expect(getCamelotColor('F')).toEqual(CAMELOT_COLORS['7B'])
		})

		it('returns null for unknown/invalid key strings', () => {
			expect(getCamelotColor('InvalidKey')).toBeNull()
			expect(getCamelotColor('13A')).toBeNull()
			expect(getCamelotColor('0B')).toBeNull()
			expect(getCamelotColor('H major')).toBeNull()
		})
	})

	describe('formatCamelotKey', () => {
		it('returns "-" for null, undefined, or empty string', () => {
			expect(formatCamelotKey(null)).toBe('-')
			expect(formatCamelotKey(undefined)).toBe('-')
			expect(formatCamelotKey('')).toBe('-')
			expect(formatCamelotKey('   ')).toBe('-')
		})

		it('converts standard keys to Camelot notation without zero-padding', () => {
			expect(formatCamelotKey('Am')).toBe('8A')
			expect(formatCamelotKey('C')).toBe('8B')
			expect(formatCamelotKey('F#m')).toBe('11A')
			expect(formatCamelotKey('Db')).toBe('3B')
			expect(formatCamelotKey('8A')).toBe('8A')
			expect(formatCamelotKey('10B')).toBe('10B')
		})

		it('pads single-digit keys when zeroPad is true', () => {
			expect(formatCamelotKey('1A', true)).toBe('01A')
			expect(formatCamelotKey('5B', true)).toBe('05B')
			expect(formatCamelotKey('8A', true)).toBe('08A')
			expect(formatCamelotKey('Am', true)).toBe('08A')
			expect(formatCamelotKey('C', true)).toBe('08B')
			expect(formatCamelotKey('F', true)).toBe('07B')
		})

		it('does not pad two-digit keys even when zeroPad is true', () => {
			expect(formatCamelotKey('10A', true)).toBe('10A')
			expect(formatCamelotKey('11B', true)).toBe('11B')
			expect(formatCamelotKey('12A', true)).toBe('12A')
			expect(formatCamelotKey('E', true)).toBe('12B')
			expect(formatCamelotKey('Bm', true)).toBe('10A')
		})

		it('returns unmapped uppercase strings for unknown keys', () => {
			expect(formatCamelotKey('unknown')).toBe('UNKNOWN')
			expect(formatCamelotKey('xyz', true)).toBe('XYZ')
		})
	})

	describe('getHarmonicKeys & isHarmonicallyCompatible', () => {
		it('returns compatible keys for 8A (Am)', () => {
			const keys = getHarmonicKeys('8A')
			// Same key (8A), relative (8B), adjacent down (7A), adjacent up (9A), energy boost (10A)
			expect(keys).toContain('8A')
			expect(keys).toContain('8B')
			expect(keys).toContain('7A')
			expect(keys).toContain('9A')
			expect(keys).toContain('10A')
		})

		it('handles wheel wraparound for 12A and 1A', () => {
			const keys12A = getHarmonicKeys('12A')
			expect(keys12A).toContain('12A')
			expect(keys12A).toContain('12B')
			expect(keys12A).toContain('11A')
			expect(keys12A).toContain('1A') // 12 + 1 = 1
			expect(keys12A).toContain('2A') // 12 + 2 = 2

			const keys1A = getHarmonicKeys('1A')
			expect(keys1A).toContain('1A')
			expect(keys1A).toContain('1B')
			expect(keys1A).toContain('12A') // 1 - 1 = 12
			expect(keys1A).toContain('2A')
			expect(keys1A).toContain('3A')
		})

		it('checks compatibility correctly', () => {
			expect(isHarmonicallyCompatible('8A', '8A')).toBe(true)
			expect(isHarmonicallyCompatible('8A', '8B')).toBe(true)
			expect(isHarmonicallyCompatible('8A', '7A')).toBe(true)
			expect(isHarmonicallyCompatible('8A', '9A')).toBe(true)
			expect(isHarmonicallyCompatible('8A', '10A')).toBe(true)
			expect(isHarmonicallyCompatible('8A', '2A')).toBe(false)
			expect(isHarmonicallyCompatible(null, '8A')).toBe(false)
		})
	})
})
