import type { Cue } from '../types'

/** Number of hot cue slots (A–H), stored 0-based in `hot_cue_index` like upstream and Rekordbox. */
export const HOT_CUE_SLOTS = 8

/**
 * Returns the hot cue stored in `slot` (0–7), or null. Memory cues and loops never occupy a pad.
 */
export function hotCueForSlot(cues: readonly Cue[], slot: number): Cue | null {
	return cues.find((cue) => cue.cue_type === 'hot' && cue.hot_cue_index === slot) ?? null
}

/** The 8 pads, in order, each with its hot cue (or null). Labels are 1–8 for display. */
export function hotCuePads(cues: readonly Cue[]): { slot: number; label: number; cue: Cue | null }[] {
	return Array.from({ length: HOT_CUE_SLOTS }, (_, slot) => ({
		slot,
		label: slot + 1,
		cue: hotCueForSlot(cues, slot),
	}))
}

/** Short label shown on the waveform marker: 1–8 for hot cues, "M" for memory cues, "L" for loops. */
export function cueMarkerLabel(cue: Cue): string {
	if (cue.cue_type === 'hot' && cue.hot_cue_index != null) return String(cue.hot_cue_index + 1)
	return cue.cue_type === 'loop' ? 'L' : 'M'
}
