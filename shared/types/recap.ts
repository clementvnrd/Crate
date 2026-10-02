// Types of the listening-history features of Crate Pulse: the recap, the export and the
// timeline of a Rekordbox set. They mirror the Rust structs field for field (snake_case), see
// `services/stats/recap.rs`, `history_export.rs` and `session_timeline.rs`.

import type { HarmonicStatsItem, RekordboxSession, TopArtistItem, TopTrackItem } from './stats'
import type { HarmonicRelation } from './djprep'

/** A calendar period: a week (Monday to Sunday) or a year (1 January to 31 December). */
export type RecapPeriod = 'week' | 'year'

/** The busiest calendar day of a period. */
export interface RecapDay {
	/** Local date, `YYYY-MM-DD`. */
	date: string
	plays: number
	minutes: number
}

export interface Recap {
	period: RecapPeriod
	/** First and last local day of the period, `YYYY-MM-DD`, both included. */
	start_date: string
	end_date: string
	total_plays: number
	total_minutes: number
	/** Different tracks (and artists) heard at least once for 30 s or more. */
	unique_tracks: number
	unique_artists: number
	/** Tracks whose very first listen, all sources and all time considered, is in this period. */
	new_tracks: number
	/** Totals of the period just before, to show the change. */
	previous_plays: number
	previous_minutes: number
	top_tracks: TopTrackItem[]
	top_artists: TopArtistItem[]
	top_keys: HarmonicStatsItem[]
	busiest_day: RecapDay | null
	/** Local hour of the day (0 to 23) with the most plays, exact times only. */
	peak_hour: number | null
	/** Day of the week (0 = Sunday … 6 = Saturday) with the most listening minutes. */
	peak_weekday: number | null
	/** Listening minutes per source (`spotify`, `crate_local`, `rekordbox`…). */
	source_minutes: Record<string, number>
}

/** File format of the listening-history export. */
export type HistoryExportFormat = 'csv' | 'json'

export interface SessionTransition {
	harmonic: HarmonicRelation
	/** Tempo change from the previous track, in percent. */
	bpm_delta_percent: number | null
	/** Energy change from the previous track (positive = higher). `null` when either track's energy is unknown. */
	energy_delta: number | null
	/** The energy changes by the set planner's threshold (3 levels) or more. */
	energy_jump: boolean
}

export interface SessionTrack {
	/** 1-based position in the set. */
	position: number
	played_at: string
	title: string
	artist: string
	album: string | null
	duration_ms: number
	bpm: number | null
	key: string | null
	energy: number | null
	/** The matching library track, when Crate knows the file. */
	library_track_id: string | null
	/** How this track mixes out of the previous one; `null` for the first track. */
	from_previous: SessionTransition | null
}

/** The tracks of one Rekordbox set in order, with how every transition mixes. */
export interface SessionTimeline {
	session: RekordboxSession
	tracks: SessionTrack[]
	harmonic_transitions: number
	clashing_transitions: number
	unknown_transitions: number
	/** Transitions where the energy jumps by the set planner's threshold (3 levels) or more. */
	energy_jumps: number
}
