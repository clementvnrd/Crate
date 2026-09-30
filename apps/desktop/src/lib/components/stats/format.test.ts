import { describe, expect, it } from 'vitest'
import {
	discoverySourceName,
	formatChange,
	formatHour,
	formatShare,
	formatTempoDelta,
	formatWeekday,
	funnelSince,
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
})
