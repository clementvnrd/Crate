// Pure logic of the SegmentedControl (common/SegmentedControl.svelte), kept apart so it can be unit-tested.
// The control follows the WAI-ARIA radio group pattern: one tab stop, arrow keys move the selection.

/** Index of the option whose value is `value`, or -1 when none matches (e.g. a view that has no segment). */
export function selectedIndex<T>(values: readonly T[], value: T | null | undefined): number {
	return value === null || value === undefined ? -1 : values.indexOf(value)
}

/** The segment that takes the single tab stop: the selected one, or the first when nothing is selected. */
export function focusableIndex(selected: number, count: number): number {
	if (count <= 0) return -1
	return selected >= 0 && selected < count ? selected : 0
}

/**
 * The segment a key press moves to, or `null` when the key is not handled by the control.
 * Arrows wrap around (right/down = next, left/up = previous); Home and End jump to the ends.
 */
export function indexForKey(key: string, current: number, count: number): number | null {
	if (count <= 0) return null
	const from = current >= 0 && current < count ? current : 0
	switch (key) {
		case 'ArrowRight':
		case 'ArrowDown':
			return current < 0 ? 0 : (from + 1) % count
		case 'ArrowLeft':
		case 'ArrowUp':
			return current < 0 ? count - 1 : (from - 1 + count) % count
		case 'Home':
			return 0
		case 'End':
			return count - 1
		default:
			return null
	}
}
