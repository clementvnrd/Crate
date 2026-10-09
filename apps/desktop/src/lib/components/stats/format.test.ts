import { describe, expect, it } from 'vitest'
import {
	discoverySourceName,
	formatChange,
	formatHour,
	formatShare,
	formatTempoDelta,
	formatWeekday,
	funnelFollowsRange,
	funnelSince,
	previousYears,
	recapYearOffset,
	splitMinutes,
} from './format'

// Intl separates the number and the percent sign with a narrow no-break space in French.
const clean = (value: string | null) => value?.replace(/\u202f|\u00a0/g, ' ') ?? null

describe('Pulse formatting', () => {
	it('splits a listening time into hours and minutes', () => {
		expect(splitMinutes(742)).toEqual({ hours: 12, minutes: 22 })
		expect(splitMinutes(-5)).toEqual({ hours: 0, minutes: 0 })
	})

	it('signs the change against the previous period, and has none without a previous period', () => {
		expect(formatChange(112, 100, 'en')).toBe('+12%')
		expect(formatChange(90, 100, 'en')).toBe('-10%')
		expect(formatChange(100, 100, 'en')).toBe('0%')
		expect(clean(formatChange(112, 100, 'fr'))).toBe('+12 %')
		expect(formatChange(5, 0, 'en')).toBeNull()
	})

	it('writes shares and tempo changes in the app language', () => {
		expect(formatShare(2, 5, 'en')).toBe('40%')
		expect(formatShare(1, 0, 'en')).toBe('0%')
		expect(formatTempoDelta(2.4, 'en')).toBe('+2.4%')
		expect(clean(formatTempoDelta(-1.5, 'fr'))).toBe('-1,5 %')
	})

	it('names hours and weekdays through Intl', () => {
		expect(formatHour(22, 'en')).toBe('10 PM')
		expect(clean(formatHour(22, 'fr'))).toBe('22 h')
		expect(formatWeekday(5, 'en')).toBe('Friday')
		expect(formatWeekday(0, 'fr')).toBe('dimanche')
	})

	it('names the discovery sources and leaves the others to translation', () => {
		expect(discoverySourceName('soundcloud')).toBe('SoundCloud')
		expect(discoverySourceName('other')).toBeNull()
	})

	it('maps the Pulse period to the first day the funnel counts', () => {
		const now = new Date(2026, 8, 28, 18, 30)
		expect(funnelSince('today', now)).toBe('2026-09-28')
		expect(funnelSince('7d', now)).toBe('2026-09-21')
		expect(funnelSince('30d', now)).toBe('2026-08-29')
		expect(funnelSince('year', now)).toBe('2026-01-01')
		expect(funnelSince('all', now)).toBeUndefined()
	})

	it('counts the rolling months from the same day, clamped to the end of a shorter month', () => {
		const now = new Date(2026, 8, 28, 18, 30)
		expect(funnelSince('3m', now)).toBe('2026-06-28')
		expect(funnelSince('6m', now)).toBe('2026-03-28')
		expect(funnelSince('3m', new Date(2026, 4, 31, 9))).toBe('2026-02-28')
		expect(funnelSince('6m', new Date(2028, 7, 31, 9))).toBe('2028-02-29')
		expect(funnelSince('3m', new Date(2026, 1, 15, 9))).toBe('2025-11-15')
	})

	it('counts all time for a past calendar year, and says it cannot follow it', () => {
		expect(funnelSince('year:2025')).toBeUndefined()
		expect(funnelFollowsRange('year:2025')).toBe(false)
		expect(funnelFollowsRange('custom:2026-01-01,2026-01-31')).toBe(false)
		for (const range of ['today', '7d', '30d', '3m', '6m', 'year', 'all']) expect(funnelFollowsRange(range)).toBe(true)
	})

	it('points the recap at the calendar year the period names, and nowhere for the other periods', () => {
		const now = new Date(2026, 8, 28, 18, 30)
		expect(recapYearOffset('year', now)).toBe(0)
		expect(recapYearOffset('year:2026', now)).toBe(0)
		expect(recapYearOffset('year:2024', now)).toBe(2)
		expect(recapYearOffset('year:2027', now)).toBeNull()
		for (const range of ['today', '7d', '30d', '3m', '6m', 'all', 'custom:2026-01-01,2026-01-31']) {
			expect(recapYearOffset(range, now)).toBeNull()
		}
	})

	it('offers the years with data before the current one, newest first', () => {
		const now = new Date(2026, 8, 28, 18, 30)
		expect(previousYears([2026, 2024, 2025, 2021], now)).toEqual([2025, 2024, 2021])
		expect(previousYears([2026], now)).toEqual([])
		expect(previousYears([], now)).toEqual([])
		expect(previousYears([2027, 2025, 2025], now)).toEqual([2025])
	})
})
