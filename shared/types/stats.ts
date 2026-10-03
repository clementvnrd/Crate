// =============================================================================
// Crate Pulse & Stats Types
// =============================================================================

export type ListenSource = 'spotify' | 'crate_local' | 'crate_beatport' | 'rekordbox' | 'mixed_in_key'

export type TimeRange = 'today' | '7d' | '30d' | 'year' | 'all'

/** High-level listening statistics summary */
export interface StatsSummary {
	total_minutes: number
	today_minutes: number
	week_minutes: number
	month_minutes: number
	total_plays: number
	source_breakdown: Record<string, number>
	/** Distinct artists with at least one stream in the range (not capped by the top-artists limit). */
	unique_artists: number
	/** Rekordbox sets that started in the range. */
	dj_sessions: number
	/** Total played time of those sets, in milliseconds. */
	dj_sessions_played_ms: number
}

/** Top listened track aggregation item */
export interface TopTrackItem {
	title: string
	artist: string
	album: string | null
	artwork_url: string | null
	plays: number
	total_minutes: number
	bpm: number | null
	key: string | null
	energy: number | null
	sources: string[]
}

/** Top listened artist aggregation item */
export interface TopArtistItem {
	artist: string
	plays: number
	total_minutes: number
	top_track: string | null
	artwork_url: string | null
}

/** Harmonic key listening breakdown */
export interface HarmonicStatsItem {
	key: string
	plays: number
	total_minutes: number
	percentage: number
}

/** BPM range bucket distribution */
export interface BpmBucketItem {
	bpm_range: string
	count: number
	total_minutes: number
}

/** Hourly heatmap matrix cell (7 days x 24 hours) */
export interface HeatmapCell {
	day_of_week: number // 0 = Sunday, 1 = Monday, ..., 6 = Saturday
	hour_of_day: number // 0 to 23
	minutes: number
	plays: number
}

/** Spotify Authentication & connection state */
export interface SpotifyAuthState {
	is_connected: boolean
	user_id: string | null
	user_name: string | null
	expires_at: number | null
}

/** Spotify currently playing track info */
export interface SpotifyNowPlaying {
	is_playing: boolean
	track_id: string | null
	title: string | null
	artist: string | null
	album: string | null
	duration_ms: number | null
	progress_ms: number | null
	artwork_url: string | null
	device_name: string | null
}

/** Result summary of a Spotify archive JSON import */
export interface SpotifyImportResult {
	imported_count: number
	skipped_count: number
	total_minutes: number
}

/** Result of resetting the Spotify-sourced part of the listening history. */
export interface SpotifyResetResult {
	deleted_count: number
	backup_path: string
}

/** Rekordbox DJ performance session */
export interface RekordboxSession {
	id: string
	session_name: string | null
	started_at: string
	ended_at: string | null
	total_tracks: number
	total_played_ms: number
}

/** Represents a single track listening event */
export interface ListenEvent {
	id: string
	source: string
	track_id: string | null
	title: string
	artist: string
	album: string | null
	duration_ms: number
	played_ms: number
	bpm: number | null
	key: string | null
	energy: number | null
	format: string | null
	artwork_url: string | null
	played_at: string
	session_id: string | null
	metadata_json: string | null
}
