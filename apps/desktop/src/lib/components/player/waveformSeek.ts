// Keyboard seeking on the Player's waveform. The steps mirror the global shortcuts (useKeyboardShortcuts.ts) so a
// key does the same thing whether the waveform has focus or not: arrows move 10 s, Shift+arrows a 15 s phrase in the
// Player, Cmd/Ctrl+arrows 1 s. Page Up / Page Down move 30 s, Home / End go to the start / the end. Up and Down stay
// the global volume shortcuts, so they are not handled here.

export const SEEK_STEP_MS = 10_000
export const PHRASE_STEP_MS = 15_000
export const FINE_STEP_MS = 1_000
export const PAGE_STEP_MS = 30_000

export interface SeekKeyEvent {
	key: string
	shiftKey?: boolean
	metaKey?: boolean
	ctrlKey?: boolean
	altKey?: boolean
}

/**
 * Where a key press on the waveform should seek to, in milliseconds, clamped to the track; `null` when the key is not
 * a seek key (let it through to the global shortcuts) or when there is nothing to seek (no duration).
 */
export function waveformSeekTarget(event: SeekKeyEvent, positionMs: number, durationMs: number): number | null {
	if (!(durationMs > 0) || event.altKey) return null

	const fine = event.metaKey || event.ctrlKey
	const step = fine ? FINE_STEP_MS : event.shiftKey ? PHRASE_STEP_MS : SEEK_STEP_MS

	let target: number
	switch (event.key) {
		case 'ArrowLeft':
			target = positionMs - step
			break
		case 'ArrowRight':
			target = positionMs + step
			break
		case 'PageDown':
			target = positionMs - PAGE_STEP_MS
			break
		case 'PageUp':
			target = positionMs + PAGE_STEP_MS
			break
		case 'Home':
			target = 0
			break
		case 'End':
			target = durationMs
			break
		default:
			return null
	}

	return Math.round(Math.min(durationMs, Math.max(0, target)))
}
