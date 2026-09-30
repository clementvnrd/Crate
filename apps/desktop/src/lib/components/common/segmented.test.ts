import { describe, expect, it } from 'vitest'
import { focusableIndex, indexForKey, selectedIndex } from './segmented'

describe('SegmentedControl logic', () => {
	const views = ['player', 'library', 'beatport'] as const

	describe('selectedIndex', () => {
		it('finds the selected option', () => {
			expect(selectedIndex(views, 'library')).toBe(1)
		})

		it('returns -1 when the value matches no option (Discovery or Pulse in the view switcher)', () => {
			expect(selectedIndex<string>(views, 'stats')).toBe(-1)
			expect(selectedIndex(views, null)).toBe(-1)
			expect(selectedIndex(views, undefined)).toBe(-1)
		})
	})

	describe('focusableIndex', () => {
		it('gives the tab stop to the selected segment', () => {
			expect(focusableIndex(2, 3)).toBe(2)
		})

		it('gives it to the first segment when nothing is selected', () => {
			expect(focusableIndex(-1, 3)).toBe(0)
		})

		it('has no tab stop without options', () => {
			expect(focusableIndex(-1, 0)).toBe(-1)
		})
	})

	describe('indexForKey', () => {
		it('moves to the next segment with ArrowRight and ArrowDown, wrapping at the end', () => {
			expect(indexForKey('ArrowRight', 0, 3)).toBe(1)
			expect(indexForKey('ArrowDown', 1, 3)).toBe(2)
			expect(indexForKey('ArrowRight', 2, 3)).toBe(0)
		})

		it('moves to the previous segment with ArrowLeft and ArrowUp, wrapping at the start', () => {
			expect(indexForKey('ArrowLeft', 2, 3)).toBe(1)
			expect(indexForKey('ArrowUp', 0, 3)).toBe(2)
		})

		it('jumps to the ends with Home and End', () => {
			expect(indexForKey('Home', 2, 5)).toBe(0)
			expect(indexForKey('End', 0, 5)).toBe(4)
		})

		it('starts from the ends when nothing is selected yet', () => {
			expect(indexForKey('ArrowRight', -1, 3)).toBe(0)
			expect(indexForKey('ArrowLeft', -1, 3)).toBe(2)
		})

		it('ignores the other keys (Enter and Space are handled by the button itself)', () => {
			expect(indexForKey('Enter', 1, 3)).toBeNull()
			expect(indexForKey(' ', 1, 3)).toBeNull()
			expect(indexForKey('a', 1, 3)).toBeNull()
		})

		it('does nothing without options', () => {
			expect(indexForKey('ArrowRight', -1, 0)).toBeNull()
		})
	})
})
