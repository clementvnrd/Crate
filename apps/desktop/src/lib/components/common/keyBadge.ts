// Pure logic of the KeyBadge (common/KeyBadge.svelte). The Camelot colours come only from
// `getCamelotColor()` in shared/utils/camelot.ts (DESIGN.md, Data palettes); nothing here copies them.
import { formatCamelotKey, getCamelotColor } from '$shared/utils/camelot'

/**
 * Where the key comes from, when the view knows it. `mik`: analysed by Mixed In Key (coloured badge and
 * MIK ring). `other`: any other source; the badge stays neutral so a colour always means "Mixed In Key".
 */
export type KeyAnalysis = 'mik' | 'other'

export interface KeyBadgeOptions {
	/** Text to show instead of the Camelot notation (e.g. the user's key notation setting). */
	label?: string
	zeroPad?: boolean
	analysis?: KeyAnalysis
	/** Also colour the border with the palette's border shade (the badges that draw a border). */
	bordered?: boolean
}

export interface KeyBadgeAppearance {
	/** Text shown in the badge; `-` when there is no key. */
	label: string
	/** Musical name of the key (e.g. "A minor"), or null when the key is not a Camelot key. */
	name: string | null
	/** Inline colours from the Camelot palette, or null when the badge is not coloured. */
	style: string | null
	/** Whether the Mixed In Key ring is drawn on the badge. */
	mikRing: boolean
	empty: boolean
}

export function keyBadgeAppearance(
	value: string | null | undefined,
	options: KeyBadgeOptions = {}
): KeyBadgeAppearance {
	const { label, zeroPad = false, analysis, bordered = false } = options
	const empty = !value || !value.trim()
	const info = empty ? null : getCamelotColor(value)
	const colored = info !== null && analysis !== 'other'
	let style: string | null = null
	if (colored && info) {
		style = `background-color: ${info.bg}; color: ${info.text};`
		if (bordered) style += ` border-color: ${info.border};`
	}
	return {
		label: empty ? '-' : (label ?? formatCamelotKey(value, zeroPad)),
		name: info?.name ?? null,
		style,
		mikRing: colored && analysis === 'mik',
		empty,
	}
}
