// Formatting shared by the recap, funnel and set-timeline cards of Crate Pulse. Everything goes through `Intl`
// with the app language, so French reads "18 h", "+12 %" and "vendredi" without any hand-written format.

/** Hours and minutes of a listening time, as the `stats.duration.*` keys expect them. */
export function splitMinutes(total: number): { hours: number; minutes: number } {
	const safe = Math.max(0, Math.round(total))
	return { hours: Math.floor(safe / 60), minutes: safe % 60 }
}

/** Change from `previous` to `current` as a signed whole percentage ("+12%"), or null when there is no previous. */
export function formatChange(current: number, previous: number, locale: string): string | null {
	if (!previous || previous <= 0) return null
	const change = (current - previous) / previous
	return new Intl.NumberFormat(locale, {
		style: 'percent',
		maximumFractionDigits: 0,
		signDisplay: 'exceptZero',
	}).format(change)
}

/** A share between 0 and 1 as a whole percentage ("40%"); 0 when the whole is empty. */
export function formatShare(part: number, whole: number, locale: string): string {
	const share = whole > 0 ? part / whole : 0
	return new Intl.NumberFormat(locale, { style: 'percent', maximumFractionDigits: 0 }).format(share)
}

/** A tempo change in percent, signed, with one decimal ("+2.4%"). */
export function formatTempoDelta(percent: number, locale: string): string {
	return new Intl.NumberFormat(locale, {
		style: 'percent',
		maximumFractionDigits: 1,
		signDisplay: 'exceptZero',
	}).format(percent / 100)
}

/** An energy change, signed, as a whole number ("+3", "-2"). */
export function formatEnergyDelta(delta: number, locale: string): string {
	return new Intl.NumberFormat(locale, { maximumFractionDigits: 0, signDisplay: 'exceptZero' }).format(delta)
}

/** Local hour of the day (0 to 23) in the language's clock ("10 PM", "22 h"). */
export function formatHour(hour: number, locale: string): string {
	return new Intl.DateTimeFormat(locale, { hour: 'numeric' }).format(new Date(2026, 0, 1, hour))
}

/** Day of the week (0 = Sunday … 6 = Saturday), in full ("Friday", "vendredi"). */
export function formatWeekday(weekday: number, locale: string): string {
	// 4 January 2026 is a Sunday.
	return new Intl.DateTimeFormat(locale, { weekday: 'long' }).format(new Date(2026, 0, 4 + weekday))
}

/** Time of day of an ISO timestamp ("10:42 PM", "22:42"). */
export function formatTimeOfDay(iso: string, locale: string): string {
	return new Intl.DateTimeFormat(locale, { hour: 'numeric', minute: '2-digit' }).format(new Date(iso))
}

/** Brand name of a discovery source; null for `other` and unknown sources (the caller translates those). */
export function discoverySourceName(source: string): string | null {
	const names: Record<string, string> = {
		bandcamp: 'Bandcamp',
		soundcloud: 'SoundCloud',
		youtube: 'YouTube',
		discogs: 'Discogs',
		beatport: 'Beatport',
	}
	return names[source] ?? null
}

/** `date` moved back `months` calendar months, the day clamped to the end of a shorter month (31 August - 6 months is
 *  28 or 29 February), like the backend's rolling `3m` and `6m` ranges. */
function monthsBefore(date: Date, months: number): Date {
	const target = new Date(date.getFullYear(), date.getMonth() - months, 1)
	const lastDay = new Date(target.getFullYear(), target.getMonth() + 1, 0).getDate()
	target.setDate(Math.min(date.getDate(), lastDay))
	return target
}

/** Local day `YYYY-MM-DD` from which the funnel counts, for a Pulse period; undefined for all time. */
export function funnelSince(range: string, now: Date = new Date()): string | undefined {
	let day = new Date(now.getFullYear(), now.getMonth(), now.getDate())
	switch (range) {
		case 'today':
			break
		case '7d':
			day.setDate(day.getDate() - 7)
			break
		case '30d':
			day.setDate(day.getDate() - 30)
			break
		case '3m':
			day = monthsBefore(day, 3)
			break
		case '6m':
			day = monthsBefore(day, 6)
			break
		case 'year':
			day.setMonth(0, 1)
			break
		default:
			return undefined
	}
	const month = String(day.getMonth() + 1).padStart(2, '0')
	const date = String(day.getDate()).padStart(2, '0')
	return `${day.getFullYear()}-${month}-${date}`
}

/**
 * Whether the funnel can count exactly the Pulse period. It only takes a start day for now (CRA-186 adds the end), so
 * a calendar year or a custom window, which end before today, cannot be followed: the funnel then counts all time and
 * says so.
 */
export function funnelFollowsRange(range: string): boolean {
	return !range.startsWith('year:') && !range.startsWith('custom:')
}

/**
 * The recap of the calendar year a Pulse period names, as the number of years back from `now`: 0 for "this year",
 * 1 for last year's `year:YYYY`… Null for every other period (the recap then keeps its own choice).
 */
export function recapYearOffset(range: string, now: Date = new Date()): number | null {
	if (range === 'year') return 0
	if (!range.startsWith('year:')) return null
	const back = now.getFullYear() - Number(range.slice('year:'.length))
	return Number.isInteger(back) && back >= 0 ? back : null
}

/** The years with data that come before the current one, newest first: the "previous years" of the period bar (the
 *  current year already has its own "This year" preset). */
export function previousYears(years: readonly number[], now: Date = new Date()): number[] {
	const current = now.getFullYear()
	return [...new Set(years.filter((year) => Number.isInteger(year) && year < current))].sort((a, b) => b - a)
}
