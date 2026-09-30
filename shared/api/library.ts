import { invoke } from '@tauri-apps/api/core'
import type {
	DiscrepancyReport,
	DuplicateResolution,
	FileMatchResult,
	ImportResult,
	ImportResultWithDuplicates,
	NextTrackSuggestion,
	OrganisationBatch,
	OrganisationPlan,
	OrganisationResult,
	OrganisationRule,
	SetAnalysis,
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
/** Syncs from Mixed In Key: all tracks, or only `trackIds` when given. */
export async function syncFromMikDatabase(trackIds?: string[]): Promise<import('../types').MikSyncResult> {
	return invoke<import('../types').MikSyncResult>('sync_from_mik_database', { trackIds: trackIds ?? null })
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

/**
 * The tracks to play after `trackId`: first the ones that really followed it in your Rekordbox
 * sets, then, if fewer than asked for, compatible tracks (a neighbouring key, a tempo within a
 * deck's pitch range). `limit` is 10 by default and at most 50.
 */
export async function suggestNextTracks(trackId: string, limit?: number): Promise<NextTrackSuggestion[]> {
	return invoke<NextTrackSuggestion[]>('suggest_next_tracks', { trackId, limit: limit ?? null })
}

/**
 * Checks a DJ set in the given order: the key, tempo and energy of every transition, and for the
 * ones that clash or jump in tempo, library tracks that could bridge them.
 */
export async function analyzeSet(trackIds: string[]): Promise<SetAnalysis> {
	return invoke<SetAnalysis>('analyze_set', { trackIds })
}

/**
 * The same tracks in an order that mixes well, as track ids. `startTrackId` opens the set; by
 * default it is the calmest track.
 */
export async function suggestSetOrder(trackIds: string[], startTrackId?: string): Promise<string[]> {
	return invoke<string[]>('suggest_set_order', { trackIds, startTrackId: startTrackId ?? null })
}

/**
 * Where Crate, Mixed In Key and (with a Rekordbox XML export) Rekordbox disagree: keys, tempos,
 * energy, cues, missing files, tracks one has and another lacks. Read only.
 */
export async function getDiscrepancyReport(rekordboxXmlPath?: string): Promise<DiscrepancyReport> {
	return invoke<DiscrepancyReport>('get_discrepancy_report', { rekordboxXmlPath: rekordboxXmlPath ?? null })
}

/**
 * Previews where each file would go under `rule`, without touching anything. `trackIds` limits the
 * plan to some tracks; omit it for the whole library.
 */
export async function planOrganisation(rule: OrganisationRule, trackIds?: string[]): Promise<OrganisationPlan> {
	return invoke<OrganisationPlan>('plan_organisation', { rule, trackIds: trackIds ?? null })
}

/**
 * Moves the files. It only runs if the plan is exactly the previewed one (`expectedPlanId` is the
 * `id` of the plan the user saw; pass the same `rule` and `trackIds`) and the user confirmed that
 * Rekordbox and other tools that remember file paths will lose them (`understandsExternalTools`).
 * Nothing is ever overwritten or deleted, and the batch can be undone.
 */
export async function applyOrganisation(
	rule: OrganisationRule,
	expectedPlanId: string,
	understandsExternalTools: boolean,
	trackIds?: string[]
): Promise<OrganisationResult> {
	return invoke<OrganisationResult>('apply_organisation', {
		rule,
		trackIds: trackIds ?? null,
		expectedPlanId,
		understandsExternalTools,
	})
}

/** Puts back the files of a batch that was applied. */
export async function undoOrganisation(batchId: string): Promise<OrganisationResult> {
	return invoke<OrganisationResult>('undo_organisation', { batchId })
}

/** The organisation batches that were applied, latest first (10 by default). */
export async function getOrganisationBatches(limit?: number): Promise<OrganisationBatch[]> {
	return invoke<OrganisationBatch[]>('get_organisation_batches', { limit: limit ?? null })
}
