import { invoke } from '@tauri-apps/api/core'
import type {
	DuplicateResolution,
	FileMatchResult,
	ImportResult,
	ImportResultWithDuplicates,
	Track,
	TrackColor,
	TrackFilter,
	TrackUpdate,
} from '../types'

/**
 * Import tracks from file paths into the library
 */
export async function importTracks(paths: string[]): Promise<ImportResult> {
	return invoke<ImportResult>('import_tracks', { paths })
}

/**
 * Get all tracks with optional filtering
 */
export async function getTracks(filter?: TrackFilter): Promise<Track[]> {
	return invoke<Track[]>('get_tracks', { filter: filter ?? null })
}

/**
 * Get a single track by ID
 */
export async function getTrack(id: string): Promise<Track> {
	return invoke<Track>('get_track', { id })
}

/**
 * Update track metadata
 */
export async function updateTrack(id: string, update: TrackUpdate): Promise<Track> {
	return invoke<Track>('update_track', { id, update })
}

/**
 * Delete tracks by IDs
 */
export async function deleteTracks(ids: string[]): Promise<void> {
	return invoke<void>('delete_tracks', { ids })
}

/**
 * Delete tracks from Crate DB and move their audio files to macOS Trash
 */
export async function deleteTracksAndFiles(ids: string[]): Promise<void> {
	return invoke<void>('delete_tracks_and_files', { ids })
}

/**
 * Search tracks by query string
 */
export async function searchTracks(query: string): Promise<Track[]> {
	return invoke<Track[]>('search_tracks', { query })
}

/**
 * Result of a rescan artwork operation
 */
export interface RescanResult {
	updated_count: number
	failed_count: number
}

/**
 * Rescan artwork for all tracks that don't have artwork yet
 */
export async function rescanArtwork(): Promise<RescanResult> {
	return invoke<RescanResult>('rescan_artwork')
}

/**
 * Rescan artwork for a single track
 */
export async function rescanTrackArtwork(id: string): Promise<boolean> {
	return invoke<boolean>('rescan_track_artwork', { id })
}

/**
 * Check if a track's file exists on disk
 */
export async function checkFileExists(trackId: string): Promise<boolean> {
	return invoke<boolean>('check_file_exists', { trackId })
}

/**
 * Validate if a replacement file matches the original track
 */
export async function validateReplacementFile(trackId: string, newPath: string): Promise<FileMatchResult> {
	return invoke<FileMatchResult>('validate_replacement_file', { trackId, newPath })
}

/**
 * Relocate a track to a new file path
 */
export async function relocateTrack(trackId: string, newPath: string, force: boolean = false): Promise<Track> {
	return invoke<Track>('relocate_track', { trackId, newPath, force })
}

/**
 * Set color for multiple tracks
 * @param trackIds - Array of track IDs to update
 * @param color - Color to set (null to remove color)
 */
export async function setTrackColors(trackIds: string[], color: TrackColor | null): Promise<void> {
	return invoke<void>('set_track_colors', { trackIds, color })
}

/**
 * Update multiple tracks with the same update data (bulk operation)
 */
export async function updateTracks(ids: string[], update: TrackUpdate): Promise<Track[]> {
	return invoke<Track[]>('update_tracks', { ids, update })
}

/**
 * Set artwork for a track from a user-provided file
 */
export async function setTrackArtwork(trackId: string, filePath: string): Promise<Track> {
	return invoke<Track>('set_track_artwork', { trackId, filePath })
}

/**
 * Delete artwork for a track
 */
export async function deleteTrackArtwork(trackId: string): Promise<Track> {
	return invoke<Track>('delete_track_artwork', { trackId })
}

/**
 * Re-extract artwork from the audio file
 */
export async function reextractTrackArtwork(trackId: string): Promise<Track> {
	return invoke<Track>('reextract_track_artwork', { trackId })
}

/**
 * Compare artwork files for multiple tracks to check if they are identical.
 * Returns the shared artwork path if all tracks have identical artwork, or null otherwise.
 */
export async function compareTrackArtworks(trackIds: string[]): Promise<string | null> {
	return invoke<string | null>('compare_track_artworks', { trackIds })
}

/**
 * Import tracks with duplicate detection based on content hash
 */
export async function importTracksWithDuplicates(paths: string[]): Promise<ImportResultWithDuplicates> {
	return invoke<ImportResultWithDuplicates>('import_tracks_with_duplicates', { paths })
}

/**
 * Resolve a duplicate track with the user's chosen action
 */
export async function resolveDuplicate(resolution: DuplicateResolution): Promise<Track | null> {
	return invoke<Track | null>('resolve_duplicate', { resolution })
}

/**
 * Resync Mixed In Key metadata (Key, BPM, Energy, Cues) from audio file tags
 */
export async function resyncMixedInKeyTracks(trackIds?: string[]): Promise<RescanResult> {
	return invoke<RescanResult>('resync_mixed_in_key_tracks', { trackIds: trackIds ?? null })
}

/**
 * Get status of connection to Mixed In Key's CoreData SQLite database
 */
export async function getMikDatabaseStatus(): Promise<import('../types').MikDatabaseStatus> {
	return invoke<import('../types').MikDatabaseStatus>('get_mik_database_status')
}

/**
 * Full synchronization directly with Mixed In Key 11 database (Collection11.mikdb)
 */
export async function syncFromMikDatabase(): Promise<import('../types').MikSyncResult> {
	return invoke<import('../types').MikSyncResult>('sync_from_mik_database')
}

/**
 * Prune ghost tracks from Crate DB whose physical audio files no longer exist on disk
 */
export async function pruneMissingTracks(): Promise<number> {
	return invoke<number>('prune_missing_tracks')
}

/**
 * Load waveform data on demand for a single track
 */
export async function getTrackWaveform(trackId: string): Promise<number[] | null> {
	return invoke<number[] | null>('get_track_waveform', { trackId })
}

/**
 * Load cues for a track (from Crate cues table or fallback to MIK DB)
 */
export async function getTrackCues(trackId: string): Promise<import('../types').Cue[]> {
	return invoke<import('../types').Cue[]>('get_track_cues', { trackId })
}



