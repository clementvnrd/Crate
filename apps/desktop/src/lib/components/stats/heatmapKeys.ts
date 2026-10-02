// Keyboard navigation of the Pulse listening heatmap. The heatmap is one tab stop (a grid whose active cell is
// announced through aria-activedescendant), so the cells themselves never take focus. Arrows move one hour or one
// day without wrapping, Home / End jump to the first or last hour of the day, and Ctrl / Cmd + Home / End to the
// first or last cell of the week.

export interface HeatmapPosition {
	/** Day row, 0 = Monday. */
	row: number
	/** Hour column, 0 = 00:00. */
	col: number
}

export interface HeatmapKey {
	key: string
	ctrlKey?: boolean
	metaKey?: boolean
}

/** The cell a key press moves the active cell to, or `null` when the key does not move it. */
export function heatmapCellForKey(
	{ key, ctrlKey = false, metaKey = false }: HeatmapKey,
	current: HeatmapPosition,
	rows: number,
	cols: number
): HeatmapPosition | null {
	if (rows <= 0 || cols <= 0) return null
	const row = Math.min(Math.max(current.row, 0), rows - 1)
	const col = Math.min(Math.max(current.col, 0), cols - 1)
	const toEnds = ctrlKey || metaKey
	switch (key) {
		case 'ArrowRight':
			return { row, col: Math.min(cols - 1, col + 1) }
		case 'ArrowLeft':
			return { row, col: Math.max(0, col - 1) }
		case 'ArrowDown':
			return { row: Math.min(rows - 1, row + 1), col }
		case 'ArrowUp':
			return { row: Math.max(0, row - 1), col }
		case 'Home':
			return toEnds ? { row: 0, col: 0 } : { row, col: 0 }
		case 'End':
			return toEnds ? { row: rows - 1, col: cols - 1 } : { row, col: cols - 1 }
		default:
			return null
	}
}
