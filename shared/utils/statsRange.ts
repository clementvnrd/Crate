import type { CustomTimeRange, PresetTimeRange, TimeRange, YearTimeRange } from '../types'

// Constructors and a validator for the Pulse period string. The grammar is owned by the backend
// (`StatsRange` in src-tauri/src/services/stats/range.rs); these helpers only build and check strings
// that it accepts, so a bad range is caught before the IPC call instead of coming back as an error.

export const PRESET_TIME_RANGES: readonly PresetTimeRange[] = ['today', '7d', '30d', '3m', '6m', 'year', 'all']

/** First and last year the backend accepts. */
export const MIN_RANGE_YEAR = 1970
export const MAX_RANGE_YEAR = 2999

const DAY = /^(\d{4})-(\d{2})-(\d{2})$/

/** A local calendar day as `YYYY-MM-DD` (the date's own local year, month and day, never UTC). */
export function localDay(date: Date): string {
	const year = String(date.getFullYear()).padStart(4, '0')
	const month = String(date.getMonth() + 1).padStart(2, '0')
	const day = String(date.getDate()).padStart(2, '0')
	return `${year}-${month}-${day}`
}

/** True for a real calendar day written `YYYY-MM-DD` within the accepted years. */
function isDay(value: string): boolean {
	const match = DAY.exec(value)
	if (!match) return false
	const [year, month, day] = [Number(match[1]), Number(match[2]), Number(match[3])]
	if (year < MIN_RANGE_YEAR || year > MAX_RANGE_YEAR) return false
	const date = new Date(Date.UTC(year, month - 1, day))
	return date.getUTCFullYear() === year && date.getUTCMonth() === month - 1 && date.getUTCDate() === day
}

/** The whole local calendar year `year`. Throws on a year the backend would reject. */
export function yearRange(year: number): YearTimeRange {
	if (!Number.isInteger(year) || year < MIN_RANGE_YEAR || year > MAX_RANGE_YEAR) {
		throw new RangeError(`Year out of range: ${year}`)
	}
	return `year:${year}`
}

/**
 * The local days `from` to `to`, both included. Takes dates or `YYYY-MM-DD` strings. Throws when a day is not
 * a real calendar day or when `from` is after `to`.
 */
export function customRange(from: Date | string, to: Date | string): CustomTimeRange {
	const first = typeof from === 'string' ? from : localDay(from)
	const last = typeof to === 'string' ? to : localDay(to)
	if (!isDay(first) || !isDay(last)) throw new RangeError(`Invalid day in custom range: ${first}, ${last}`)
	// `YYYY-MM-DD` strings sort like the days they name.
	if (first > last) throw new RangeError(`Custom range ends before it starts: ${first}, ${last}`)
	return `custom:${first},${last}`
}

/** True when the backend accepts `value` as a Pulse period. `between:` is internal (the recap) and not a TimeRange. */
export function isTimeRange(value: unknown): value is TimeRange {
	if (typeof value !== 'string') return false
	if ((PRESET_TIME_RANGES as readonly string[]).includes(value)) return true
	if (value.startsWith('year:')) {
		const year = value.slice('year:'.length)
		return /^\d{4}$/.test(year) && Number(year) >= MIN_RANGE_YEAR && Number(year) <= MAX_RANGE_YEAR
	}
	if (value.startsWith('custom:')) {
		const parts = value.slice('custom:'.length).split(',')
		return parts.length === 2 && isDay(parts[0]) && isDay(parts[1]) && parts[0] <= parts[1]
	}
	return false
}

/** The days a range spans, when it is a calendar year or a custom window; undefined for the presets. */
export function rangeDays(range: string): number | undefined {
	if (!isTimeRange(range)) return undefined
	const dayNumber = (day: string) => {
		const [year, month, date] = day.split('-').map(Number)
		return Date.UTC(year, month - 1, date) / 86_400_000
	}
	if (range.startsWith('year:')) {
		const year = Number(range.slice('year:'.length))
		return dayNumber(`${year + 1}-01-01`) - dayNumber(`${year}-01-01`)
	}
	if (range.startsWith('custom:')) {
		const [from, to] = range.slice('custom:'.length).split(',')
		return dayNumber(to) - dayNumber(from) + 1
	}
	return undefined
}
