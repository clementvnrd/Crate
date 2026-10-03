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

// Horizontal scrolling of a `scrollable` control (DESIGN.md "Scroll affordance"): the options stay on one line,
// arrows and edge fades show where more options are hidden, the selected option is always in view.

/** Which ends of a horizontal scroller hide content. A 1 px tolerance absorbs sub-pixel layouts. */
export function scrollEdges(
	scrollLeft: number,
	scrollWidth: number,
	clientWidth: number
): { start: boolean; end: boolean } {
	const max = scrollWidth - clientWidth
	if (max <= 1) return { start: false, end: false }
	return { start: scrollLeft > 1, end: scrollLeft < max - 1 }
}

function clamp(value: number, min: number, max: number): number {
	return Math.min(Math.max(value, min), max)
}

type ScrollView = { scrollLeft: number; clientWidth: number; scrollWidth: number }

/**
 * The scroll position that shows the whole item, keeping `inset` px clear at both ends (where the arrows and the
 * fades sit), or the current position when the item is already fully visible. `item.start` is measured from the
 * scroller's padding edge (`offsetLeft`, with the scroller as offset parent).
 */
export function revealScrollLeft(item: { start: number; width: number }, view: ScrollView, inset: number): number {
	const max = Math.max(0, view.scrollWidth - view.clientWidth)
	// An item wider than the clear area is centred rather than pushed against one arrow.
	const clear = Math.max(0, Math.min(inset, (view.clientWidth - item.width) / 2))
	const end = item.start + item.width
	let target = view.scrollLeft
	if (item.start - clear < view.scrollLeft) target = item.start - clear
	else if (end + clear > view.scrollLeft + view.clientWidth) target = end + clear - view.clientWidth
	return clamp(target, 0, max)
}

/**
 * The scroll position after an arrow press: one view width minus the two insets, so the option that was partly
 * hidden under the arrow comes into full view; never less than half a view.
 */
export function stepScrollLeft(direction: -1 | 1, view: ScrollView, inset: number): number {
	const max = Math.max(0, view.scrollWidth - view.clientWidth)
	const step = Math.max(view.clientWidth - 2 * inset, view.clientWidth / 2)
	return clamp(view.scrollLeft + direction * step, 0, max)
}
