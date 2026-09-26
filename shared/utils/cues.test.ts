import { describe, expect, it } from 'vitest'
import type { Cue } from '../types'
import { cueMarkerLabel, hotCueForSlot, hotCuePads } from './cues'

function cue(partial: Partial<Cue>): Cue {
	return {
		id: partial.id ?? 'c',
		track_id: 't',
		position_ms: 0,
		cue_type: 'hot',
		loop_end_ms: null,
		hot_cue_index: null,
		name: null,
		color: null,
		...partial,
	}
}

describe('hot cue slots', () => {
	const cues = [
		cue({ id: 'memory', cue_type: 'memory', position_ms: 500 }),
		cue({ id: 'a', hot_cue_index: 0, position_ms: 1000 }),
		cue({ id: 'c', hot_cue_index: 2, position_ms: 3000 }),
	]

	it('maps key 3 to the third hot cue (slot 2), not the second', () => {
		expect(hotCueForSlot(cues, 2)?.id).toBe('c')
		expect(hotCueForSlot(cues, 1)).toBeNull()
	})

	it('never puts a memory cue on a pad', () => {
		const pads = hotCuePads(cues)
		expect(pads.map((p) => p.cue?.id ?? null)).toEqual(['a', null, 'c', null, null, null, null, null])
		expect(pads.map((p) => p.label)).toEqual([1, 2, 3, 4, 5, 6, 7, 8])
	})

	it('labels markers 1–8 for hot cues and M/L otherwise', () => {
		expect(cueMarkerLabel(cues[1])).toBe('1')
		expect(cueMarkerLabel(cues[0])).toBe('M')
		expect(cueMarkerLabel(cue({ cue_type: 'loop' }))).toBe('L')
	})
})
