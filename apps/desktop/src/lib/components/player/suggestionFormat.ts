import type { HarmonicRelation } from '$shared/types'

/**
 * Only a `history` suggestion can show a key clash: it comes straight from a transition the owner
 * really played, clash or not. A `compatible` fallback never clashes — the backend excludes those.
 */
export function isClash(relation: HarmonicRelation): boolean {
	return relation === 'clash'
}

/**
 * `suggest_next_tracks`'s `last_played_after` is SQLite's `datetime()` output, `YYYY-MM-DD HH:MM:SS`
 * UTC with no offset marker. Most JS engines parse that shape as *local* time, not UTC, which would
 * shift the displayed date. Turning it into a proper ISO instant first keeps `formatDate` correct
 * regardless of the owner's timezone.
 */
export function toUtcIso(raw: string): string {
	return `${raw.replace(' ', 'T')}Z`
}
