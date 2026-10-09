import type { RecapPeriod } from '$shared/types'
import { isTimeRange } from '$shared/utils/statsRange'
import type { HandlerMap } from '../types'
import type { HarnessState } from '../state'
import {
	EXPORTED_LISTENS,
	REKORDBOX_SESSIONS,
	bpmStats,
	harmonicStats,
	listeningHeatmap,
	listeningYears,
	recap,
	recentListens,
	sessionTimeline,
	statsSummary,
	topArtists,
	topTracks,
} from '../fixtures/stats'

/** Like the backend: a range outside the grammar of `TimeRange` is rejected with a string, never read as all time. */
function range(value: unknown): string {
	if (isTimeRange(value)) return value
	throw `Invalid operation: invalid statistics range ${JSON.stringify(String(value).slice(0, 60))}`
}

function limit(value: unknown, fallback: number): number {
	return typeof value === 'number' ? value : fallback
}

export function statsHandlers(state: HarnessState): HandlerMap {
	return {
		get_stats_summary: ({ timeRange }) => statsSummary(range(timeRange)),
		get_top_tracks: ({ timeRange, limit: max }) => topTracks(range(timeRange), limit(max, 20)),
		get_top_artists: ({ timeRange, limit: max }) => topArtists(range(timeRange), limit(max, 20)),
		get_harmonic_stats: ({ timeRange }) => harmonicStats(range(timeRange)),
		get_bpm_stats: ({ timeRange }) => bpmStats(range(timeRange)),
		get_listening_heatmap: ({ timeRange }) => listeningHeatmap(range(timeRange)),
		get_recent_listens: ({ limit: max }) => recentListens(limit(max, 50)),
		get_listening_years: () => listeningYears(state.params.libraryEmpty),

		// Integrations: Spotify disconnected, Rekordbox and Mixed In Key detected.
		spotify_get_auth_state: () => ({ is_connected: false, user_id: null, user_name: null, expires_at: null }),
		spotify_get_now_playing: () => null,
		spotify_get_client_id: () => null,
		spotify_has_client_secret: () => false,
		rekordbox_detect_status: () => true,
		rekordbox_get_sessions: () => REKORDBOX_SESSIONS,
		mik_detect_status: () => true,
		mik_tracker_get_enabled: () => true,

		// Recap, set timeline and history export.
		get_recap: ({ period, offset }) =>
			recap(period === 'year' ? 'year' : ('week' as RecapPeriod), limit(offset, 0), state.params.libraryEmpty),
		get_rekordbox_session_timeline: ({ sessionId }) => {
			const timeline = sessionTimeline(String(sessionId))
			if (!timeline) throw `Rekordbox session not found: ${String(sessionId)}`
			return timeline
		},
		// Only reached when a save dialog answers a path, which the harness never does (it always cancels).
		export_listening_history: () => EXPORTED_LISTENS,

		// Reset Spotify history: the harness's Spotify integration starts disconnected, with no
		// recorded listens to remove.
		count_spotify_listens: () => 0,
		reset_spotify_listening_history: () => ({
			deleted_count: 0,
			backup_path: '/tmp/crate-harness/spotify-reset-backup.json',
		}),
	}
}
