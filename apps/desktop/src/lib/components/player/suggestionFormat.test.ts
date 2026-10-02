import { describe, it, expect } from 'vitest'
import { isClash, toUtcIso } from './suggestionFormat'

describe('isClash', () => {
	it('is true only for a key clash', () => {
		expect(isClash('clash')).toBe(true)
		expect(isClash('same')).toBe(false)
		expect(isClash('adjacent')).toBe(false)
		expect(isClash('relative')).toBe(false)
		expect(isClash('unknown')).toBe(false)
	})
})

describe('toUtcIso', () => {
	it('turns the backend datetime() shape into a parseable UTC instant', () => {
		expect(toUtcIso('2026-09-20 22:14:00')).toBe('2026-09-20T22:14:00Z')
	})

	it('parses as the exact UTC instant, regardless of the host timezone', () => {
		const date = new Date(toUtcIso('2026-09-20 22:14:00'))
		expect(date.getUTCFullYear()).toBe(2026)
		expect(date.getUTCMonth()).toBe(8) // 0-indexed: September
		expect(date.getUTCDate()).toBe(20)
		expect(date.getUTCHours()).toBe(22)
		expect(date.getUTCMinutes()).toBe(14)
	})
})
