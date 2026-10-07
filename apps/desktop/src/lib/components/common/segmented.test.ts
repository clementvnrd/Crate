import { describe, expect, it } from 'vitest'
import { focusableIndex, indexForKey, revealScrollLeft, scrollEdges, selectedIndex, stepScrollLeft } from './segmented'

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

	describe('scrollEdges', () => {
		it('reports nothing hidden when the content fits, sub-pixel rounding included', () => {
			expect(scrollEdges(0, 300, 300)).toEqual({ start: false, end: false })
			expect(scrollEdges(0, 300.6, 300)).toEqual({ start: false, end: false })
		})

		it('reports the hidden end at the start, both ends in the middle, the hidden start at the end', () => {
			expect(scrollEdges(0, 400, 200)).toEqual({ start: false, end: true })
			expect(scrollEdges(100, 400, 200)).toEqual({ start: true, end: true })
			expect(scrollEdges(200, 400, 200)).toEqual({ start: true, end: false })
			expect(scrollEdges(199.5, 400, 200)).toEqual({ start: true, end: false })
		})
	})

	describe('revealScrollLeft', () => {
		const view = { scrollLeft: 0, clientWidth: 200, scrollWidth: 500 }

		it('leaves a fully visible item where it is', () => {
			expect(revealScrollLeft({ start: 60, width: 60 }, view, 44)).toBe(0)
		})

		it('scrolls forward so a hidden item ends clear of the right arrow', () => {
			// 300 + 60 + 44 - 200
			expect(revealScrollLeft({ start: 300, width: 60 }, view, 44)).toBe(204)
		})

		it('scrolls back so an item hidden at the start begins clear of the left arrow', () => {
			expect(revealScrollLeft({ start: 120, width: 60 }, { ...view, scrollLeft: 250 }, 44)).toBe(76)
		})

		it('stays within the scrollable range for the first and last items', () => {
			expect(revealScrollLeft({ start: 4, width: 60 }, { ...view, scrollLeft: 250 }, 44)).toBe(0)
			expect(revealScrollLeft({ start: 436, width: 60 }, view, 44)).toBe(300)
		})

		it('centres an item wider than the clear area instead of hiding one of its ends', () => {
			// clear = (200 - 150) / 2 = 25 → 300 + 150 + 25 - 200
			expect(revealScrollLeft({ start: 300, width: 150 }, view, 44)).toBe(275)
		})
	})

	describe('stepScrollLeft', () => {
		it('moves by one view minus the two edges, within the range', () => {
			expect(stepScrollLeft(1, { scrollLeft: 0, clientWidth: 300, scrollWidth: 800 }, 44)).toBe(212)
			expect(stepScrollLeft(-1, { scrollLeft: 100, clientWidth: 300, scrollWidth: 800 }, 44)).toBe(0)
			expect(stepScrollLeft(1, { scrollLeft: 400, clientWidth: 300, scrollWidth: 800 }, 44)).toBe(500)
		})

		it('moves by at least half a view in a narrow scroller', () => {
			expect(stepScrollLeft(1, { scrollLeft: 0, clientWidth: 120, scrollWidth: 800 }, 44)).toBe(60)
		})
	})
})
