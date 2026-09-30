import { describe, expect, it } from 'vitest'
import { recentRowForKey } from './recentKeys'

describe('recentRowForKey', () => {
	it('moves down and up without wrapping', () => {
		expect(recentRowForKey('ArrowDown', 0, 4)).toBe(1)
		expect(recentRowForKey('ArrowDown', 3, 4)).toBe(3)
		expect(recentRowForKey('ArrowUp', 2, 4)).toBe(1)
		expect(recentRowForKey('ArrowUp', 0, 4)).toBe(0)
	})

	it('jumps to the ends with Home and End', () => {
		expect(recentRowForKey('Home', 2, 4)).toBe(0)
		expect(recentRowForKey('End', 0, 4)).toBe(3)
	})

	it('recovers from an out-of-range active row (the list was filtered)', () => {
		expect(recentRowForKey('ArrowUp', 9, 4)).toBe(2)
		expect(recentRowForKey('ArrowDown', -1, 4)).toBe(0)
	})

	it('ignores other keys and empty lists', () => {
		expect(recentRowForKey('Enter', 1, 4)).toBeNull()
		expect(recentRowForKey('ArrowLeft', 1, 4)).toBeNull()
		expect(recentRowForKey('ArrowDown', 0, 0)).toBeNull()
	})
})
