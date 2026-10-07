import { describe, expect, it } from 'vitest'
import { PRESET_TIME_RANGES, customRange, isTimeRange, localDay, rangeDays, yearRange } from './statsRange'

describe('localDay', () => {
	it('writes the local calendar day, zero padded', () => {
		expect(localDay(new Date(2026, 2, 5, 23, 59, 59))).toBe('2026-03-05')
		expect(localDay(new Date(2026, 11, 31, 0, 0, 0))).toBe('2026-12-31')
	})
})

describe('yearRange', () => {
	it('builds a calendar year', () => {
		expect(yearRange(2025)).toBe('year:2025')
		expect(yearRange(1970)).toBe('year:1970')
	})
	it('refuses a year the backend would reject', () => {
		for (const bad of [1969, 3000, 2025.5, NaN, -1]) expect(() => yearRange(bad)).toThrow(RangeError)
	})
})

describe('customRange', () => {
	it('builds an inclusive window from dates or day strings', () => {
		expect(customRange(new Date(2026, 2, 1), new Date(2026, 2, 31))).toBe('custom:2026-03-01,2026-03-31')
		expect(customRange('2026-03-01', '2026-03-01')).toBe('custom:2026-03-01,2026-03-01')
	})
	it('uses the local day of a date, not its UTC day', () => {
		// 00:30 local on 1 March is still 28 February in UTC for any zone east of UTC.
		expect(customRange(new Date(2026, 2, 1, 0, 30), new Date(2026, 2, 2, 23, 30))).toBe('custom:2026-03-01,2026-03-02')
	})
	it('refuses a reversed window and impossible days', () => {
		expect(() => customRange('2026-03-05', '2026-03-01')).toThrow(RangeError)
		expect(() => customRange('2026-02-30', '2026-03-01')).toThrow(RangeError)
		expect(() => customRange('2027-02-29', '2027-03-01')).toThrow(RangeError)
		expect(() => customRange('2026-3-1', '2026-3-5')).toThrow(RangeError)
		expect(() => customRange('1969-12-31', '2026-01-01')).toThrow(RangeError)
	})
	it('accepts 29 February in a leap year', () => {
		expect(customRange('2028-02-29', '2028-02-29')).toBe('custom:2028-02-29,2028-02-29')
	})
})

describe('isTimeRange', () => {
	it('accepts every preset and the built forms', () => {
		for (const preset of PRESET_TIME_RANGES) expect(isTimeRange(preset)).toBe(true)
		expect(isTimeRange('3m')).toBe(true)
		expect(isTimeRange('6m')).toBe(true)
		expect(isTimeRange(yearRange(2024))).toBe(true)
		expect(isTimeRange(customRange('2026-01-01', '2026-01-31'))).toBe(true)
	})
	it('rejects what the backend rejects', () => {
		for (const bad of [
			'',
			'yesterday',
			'12m',
			'year:',
			'year:26',
			'year:1969',
			'year:3000',
			'year:+2026',
			'custom:',
			'custom:2026-01-01',
			'custom:2026-01-31,2026-01-01',
			'custom:2026-02-30,2026-03-01',
			'custom:2026-01-01,2026-01-02,2026-01-03',
			'between:2026-09-21 00:00:00,2026-09-28 00:00:00',
			"7d'; DROP TABLE listen_events; --",
		]) {
			expect(isTimeRange(bad)).toBe(false)
		}
		expect(isTimeRange(undefined)).toBe(false)
		expect(isTimeRange(7)).toBe(false)
	})
})

describe('rangeDays', () => {
	it('counts the days of a calendar year, leap years included', () => {
		expect(rangeDays('year:2027')).toBe(365)
		expect(rangeDays('year:2028')).toBe(366)
	})
	it('counts both ends of a custom window', () => {
		expect(rangeDays('custom:2026-03-01,2026-03-01')).toBe(1)
		expect(rangeDays('custom:2026-03-01,2026-03-31')).toBe(31)
		expect(rangeDays('custom:2028-02-28,2028-03-01')).toBe(3)
	})
	it('is undefined for presets and invalid strings', () => {
		expect(rangeDays('30d')).toBeUndefined()
		expect(rangeDays('custom:2026-03-05,2026-03-01')).toBeUndefined()
	})
})
