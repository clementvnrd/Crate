// Keyboard navigation of the Player's recent files. The list is one tab stop (a grid with an active row, announced
// through aria-activedescendant), so the rows themselves never take focus. Arrows move the active row without
// wrapping, Home / End jump to the ends; Enter or Space plays it (handled by the view).

/** The row a key press moves the active row to, or `null` when the key does not move it. */
export function recentRowForKey(key: string, current: number, count: number): number | null {
	if (count <= 0) return null
	const from = Math.min(Math.max(current, 0), count - 1)
	switch (key) {
		case 'ArrowDown':
			return Math.min(count - 1, current < 0 ? 0 : from + 1)
		case 'ArrowUp':
			return Math.max(0, from - 1)
		case 'Home':
			return 0
		case 'End':
			return count - 1
		default:
			return null
	}
}
