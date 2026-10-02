import { describe, expect, it } from 'vitest'
import { heatmapCellForKey } from './heatmapKeys'

const at = (row: number, col: number) => ({ row, col })

describe('heatmapCellForKey', () => {
	it('moves one hour left or right without wrapping', () => {
		expect(heatmapCellForKey({ key: 'ArrowRight' }, at(2, 5), 7, 24)).toEqual(at(2, 6))
		expect(heatmapCellForKey({ key: 'ArrowRight' }, at(2, 23), 7, 24)).toEqual(at(2, 23))
		expect(heatmapCellForKey({ key: 'ArrowLeft' }, at(2, 5), 7, 24)).toEqual(at(2, 4))
		expect(heatmapCellForKey({ key: 'ArrowLeft' }, at(2, 0), 7, 24)).toEqual(at(2, 0))
	})

	it('moves one day up or down without wrapping', () => {
		expect(heatmapCellForKey({ key: 'ArrowDown' }, at(0, 9), 7, 24)).toEqual(at(1, 9))
		expect(heatmapCellForKey({ key: 'ArrowDown' }, at(6, 9), 7, 24)).toEqual(at(6, 9))
		expect(heatmapCellForKey({ key: 'ArrowUp' }, at(3, 9), 7, 24)).toEqual(at(2, 9))
		expect(heatmapCellForKey({ key: 'ArrowUp' }, at(0, 9), 7, 24)).toEqual(at(0, 9))
	})

	it('jumps to the ends of the day with Home and End, of the week with Ctrl or Cmd', () => {
		expect(heatmapCellForKey({ key: 'Home' }, at(4, 12), 7, 24)).toEqual(at(4, 0))
		expect(heatmapCellForKey({ key: 'End' }, at(4, 12), 7, 24)).toEqual(at(4, 23))
		expect(heatmapCellForKey({ key: 'Home', ctrlKey: true }, at(4, 12), 7, 24)).toEqual(at(0, 0))
		expect(heatmapCellForKey({ key: 'End', metaKey: true }, at(4, 12), 7, 24)).toEqual(at(6, 23))
	})

	it('recovers from an out-of-range position', () => {
		expect(heatmapCellForKey({ key: 'ArrowLeft' }, at(9, 40), 7, 24)).toEqual(at(6, 22))
		expect(heatmapCellForKey({ key: 'ArrowRight' }, at(-1, -1), 7, 24)).toEqual(at(0, 1))
	})

	it('ignores other keys and an empty grid', () => {
		expect(heatmapCellForKey({ key: 'Enter' }, at(1, 1), 7, 24)).toBeNull()
		expect(heatmapCellForKey({ key: 'PageDown' }, at(1, 1), 7, 24)).toBeNull()
		expect(heatmapCellForKey({ key: 'ArrowDown' }, at(0, 0), 0, 24)).toBeNull()
	})
})
