import { invoke } from '@tauri-apps/api/core'
import type {
	BpmBucketItem,
	HarmonicStatsItem,
	HeatmapCell,
	HistoryExportFormat,
	ListenEvent,
	Recap,
	RecapPeriod,
	RekordboxSession,
	SessionTimeline,
	SpotifyAuthState,
	SpotifyImportResult,
	SpotifyNowPlaying,
	SpotifyResetResult,
	StatsSummary,
	TimeRange,
	TopArtistItem,
	TopTrackItem,
} from '../types'

// ==========================================
// Stats Recorder API
// ==========================================

export async function getStatsSummary(timeRange: TimeRange | string = '7d'): Promise<StatsSummary> {
	return invoke<StatsSummary>('get_stats_summary', { timeRange })
}

export async function getTopTracks(timeRange: TimeRange | string = '7d', limit: number = 20): Promise<TopTrackItem[]> {
	return invoke<TopTrackItem[]>('get_top_tracks', { timeRange, limit })
}

export async function getTopArtists(
	timeRange: TimeRange | string = '7d',
	limit: number = 20
): Promise<TopArtistItem[]> {
	return invoke<TopArtistItem[]>('get_top_artists', { timeRange, limit })
}

export async function getHarmonicStats(timeRange: TimeRange | string = '7d'): Promise<HarmonicStatsItem[]> {
	return invoke<HarmonicStatsItem[]>('get_harmonic_stats', { timeRange })
}

export async function getBpmStats(timeRange: TimeRange | string = '7d'): Promise<BpmBucketItem[]> {
	return invoke<BpmBucketItem[]>('get_bpm_stats', { timeRange })
}

export async function getListeningHeatmap(timeRange: TimeRange | string = '7d'): Promise<HeatmapCell[]> {
	return invoke<HeatmapCell[]>('get_listening_heatmap', { timeRange })
}

export async function getRecentListens(limit: number = 50): Promise<ListenEvent[]> {
	return invoke<ListenEvent[]>('get_recent_listens', { limit })
}

// ==========================================
// Spotify Tracker API
// ==========================================

export async function setSpotifyClientId(clientId: string): Promise<void> {
	return invoke<void>('set_spotify_client_id', { clientId })
}

export async function getSpotifyClientId(): Promise<string | null> {
	return invoke<string | null>('spotify_get_client_id')
}

export async function setSpotifyClientSecret(clientSecret: string): Promise<void> {
	return invoke<void>('set_spotify_client_secret', { clientSecret })
}

/** Whether a client secret is stored (the secret itself never reaches the webview). */
export async function hasSpotifyClientSecret(): Promise<boolean> {
	return invoke<boolean>('spotify_has_client_secret')
}

export async function getSpotifyAuthState(): Promise<SpotifyAuthState> {
	return invoke<SpotifyAuthState>('spotify_get_auth_state')
}

export async function getSpotifyAuthUrl(clientId?: string, redirectUri?: string): Promise<string> {
	return invoke<string>('spotify_get_auth_url', {
		clientId: clientId ?? null,
		redirectUri: redirectUri ?? null,
	})
}

export async function handleSpotifyCallback(
	code: string,
	clientId?: string,
	redirectUri?: string,
	state?: string
): Promise<SpotifyAuthState> {
	return invoke<SpotifyAuthState>('spotify_exchange_code', {
		code,
		clientId: clientId ?? null,
		redirectUri: redirectUri ?? null,
		state: state ?? null,
	})
}

export async function disconnectSpotify(): Promise<void> {
	return invoke<void>('spotify_disconnect')
}

export async function getSpotifyNowPlaying(): Promise<SpotifyNowPlaying | null> {
	return invoke<SpotifyNowPlaying | null>('spotify_get_now_playing')
}

export async function importSpotifyHistoryJson(jsonContent: string): Promise<SpotifyImportResult> {
	return invoke<SpotifyImportResult>('spotify_import_history', { jsonContent })
}

export async function syncSpotifyRecentlyPlayed(): Promise<number> {
	return invoke<number>('sync_spotify_recently_played')
}

/** Number of currently recorded Spotify listens, shown before the owner confirms a reset. */
export async function countSpotifyListens(): Promise<number> {
	return invoke<number>('count_spotify_listens')
}

/**
 * Deletes every Spotify-sourced listen, after writing a full history backup to the app's data
 * folder. Refuses to delete anything if that backup failed. Every other source (library, Mixed In
 * Key, Rekordbox) is left untouched.
 */
export async function resetSpotifyListeningHistory(): Promise<SpotifyResetResult> {
	return invoke<SpotifyResetResult>('reset_spotify_listening_history')
}

// ==========================================
// Rekordbox Tracker API
// ==========================================

export async function getRekordboxDetectStatus(): Promise<boolean> {
	return invoke<boolean>('rekordbox_detect_status')
}

export async function syncRekordboxHistory(): Promise<number> {
	return invoke<number>('rekordbox_sync_history')
}

export async function importRekordboxHistoryXml(xmlContent: string): Promise<number> {
	return invoke<number>('rekordbox_import_history_xml', { xmlContent })
}

export async function getRekordboxSessions(): Promise<RekordboxSession[]> {
	return invoke<RekordboxSession[]>('rekordbox_get_sessions')
}

// ==========================================
// Mixed In Key 11 Tracker API
// ==========================================

export async function getMikDetectStatus(): Promise<boolean> {
	return invoke<boolean>('mik_detect_status')
}

/** Whether listens heard in Mixed In Key are counted (off by default, inference-based). */
export async function getMikTrackerEnabled(): Promise<boolean> {
	return invoke<boolean>('mik_tracker_get_enabled')
}

export async function setMikTrackerEnabled(enabled: boolean): Promise<void> {
	return invoke<void>('mik_tracker_set_enabled', { enabled })
}

/**
 * "Your week" and "Your year": the recap of a calendar period. `offset` 0 is the current period,
 * 1 the one before, and so on.
 */
export async function getRecap(period: RecapPeriod, offset: number = 0): Promise<Recap> {
	return invoke<Recap>('get_recap', { period, offset })
}

/**
 * Writes the whole listening history, oldest first, to `path` and resolves to how many listens
 * were written. The path comes from the native save dialog and must end in `.csv` or `.json`
 * according to `format`.
 */
export async function exportListeningHistory(format: HistoryExportFormat, path: string): Promise<number> {
	return invoke<number>('export_listening_history', { format, path })
}

/** The tracks of one Rekordbox set in order, with how every transition mixes. */
export async function getRekordboxSessionTimeline(sessionId: string): Promise<SessionTimeline> {
	return invoke<SessionTimeline>('get_rekordbox_session_timeline', { sessionId })
}
