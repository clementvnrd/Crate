import { describe, expect, it } from 'vitest'
import { FINE_STEP_MS, PAGE_STEP_MS, PHRASE_STEP_MS, SEEK_STEP_MS, waveformSeekTarget } from './waveformSeek'

describe('waveformSeekTarget', () => {
	const duration = 300_000 // 5:00
	const position = 60_000 // 1:00

	it('moves 10 s with the arrows, like the global shortcut', () => {
		expect(waveformSeekTarget({ key: 'ArrowRight' }, position, duration)).toBe(position + SEEK_STEP_MS)
		expect(waveformSeekTarget({ key: 'ArrowLeft' }, position, duration)).toBe(position - SEEK_STEP_MS)
	})

	it('moves a 15 s phrase with Shift and 1 s with Cmd or Ctrl', () => {
		expect(waveformSeekTarget({ key: 'ArrowRight', shiftKey: true }, position, duration)).toBe(
			position + PHRASE_STEP_MS
		)
		expect(waveformSeekTarget({ key: 'ArrowLeft', metaKey: true }, position, duration)).toBe(position - FINE_STEP_MS)
		expect(waveformSeekTarget({ key: 'ArrowRight', ctrlKey: true }, position, duration)).toBe(position + FINE_STEP_MS)
		// The fine step wins over Shift, as in the global shortcuts (checked first there).
		expect(waveformSeekTarget({ key: 'ArrowRight', ctrlKey: true, shiftKey: true }, position, duration)).toBe(
			position + FINE_STEP_MS
		)
	})

	it('moves 30 s with Page Up and Page Down, and jumps to the ends with Home and End', () => {
		expect(waveformSeekTarget({ key: 'PageUp' }, position, duration)).toBe(position + PAGE_STEP_MS)
		expect(waveformSeekTarget({ key: 'PageDown' }, position, duration)).toBe(position - PAGE_STEP_MS)
		expect(waveformSeekTarget({ key: 'Home' }, position, duration)).toBe(0)
		expect(waveformSeekTarget({ key: 'End' }, position, duration)).toBe(duration)
	})

	it('stays inside the track', () => {
		expect(waveformSeekTarget({ key: 'ArrowLeft' }, 4_000, duration)).toBe(0)
		expect(waveformSeekTarget({ key: 'PageUp' }, duration - 5_000, duration)).toBe(duration)
	})

	it('lets other keys through (volume arrows, digits for cues, Space, Alt combinations)', () => {
		expect(waveformSeekTarget({ key: 'ArrowUp' }, position, duration)).toBeNull()
		expect(waveformSeekTarget({ key: 'ArrowDown' }, position, duration)).toBeNull()
		expect(waveformSeekTarget({ key: '3' }, position, duration)).toBeNull()
		expect(waveformSeekTarget({ key: ' ' }, position, duration)).toBeNull()
		expect(waveformSeekTarget({ key: 'ArrowRight', altKey: true }, position, duration)).toBeNull()
	})

	it('does nothing without a duration', () => {
		expect(waveformSeekTarget({ key: 'ArrowRight' }, 0, 0)).toBeNull()
		expect(waveformSeekTarget({ key: 'Home' }, 0, Number.NaN)).toBeNull()
	})
})
