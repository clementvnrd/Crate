import type { HandlerMap } from '../types'
import type { HarnessState } from '../state'
import { DUPLICATE_SCAN } from '../fixtures/maintenance'
import { UPGRADE_MATCHES } from '../fixtures/beatport'

// Duplicate Killer and Beatport Quality Upgrader: both report nothing on an empty library.

export function maintenanceHandlers(state: HarnessState): HandlerMap {
	const empty = state.params.libraryEmpty
	return {
		get_duplicate_groups: () =>
			empty ? { groups: [], total_duplicate_tracks: 0, total_groups: 0, total_reclaimable_bytes: 0 } : DUPLICATE_SCAN,
		get_duplicate_count: () => ({
			group_count: empty ? 0 : DUPLICATE_SCAN.total_groups,
			track_count: empty ? 0 : DUPLICATE_SCAN.total_duplicate_tracks,
			reclaimable_bytes: empty ? 0 : DUPLICATE_SCAN.total_reclaimable_bytes,
		}),

		get_upgrade_matches: () => ({
			matches: empty ? [] : UPGRADE_MATCHES,
			total_scanned: state.tracks.length,
			total_eligible_mp3s: empty ? 0 : 5,
			potential_upgrades_count: empty ? 0 : UPGRADE_MATCHES.length,
		}),
		get_upgrade_count: () => ({
			match_count: empty ? 0 : UPGRADE_MATCHES.length,
			eligible_mp3_count: empty ? 0 : 5,
		}),
	}
}
