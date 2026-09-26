import { invoke } from '@tauri-apps/api/core'
import type { StandaloneTrack, PlaybackState } from '../types'

/**
 * Read standalone track metadata and check if it exists in the library.
 */
export async function readStandaloneTrack(path: string): Promise<StandaloneTrack> {
	return invoke<StandaloneTrack>('read_standalone_track', { path })
}

/**
 * Fetch recent standalone tracks history.
 */
export async function getRecentStandaloneTracks(limit?: number): Promise<StandaloneTrack[]> {
	return invoke<StandaloneTrack[]>('get_recent_standalone_tracks', { limit })
}

/**
 * Add a standalone track to the recent history.
 */
export async function addRecentStandaloneTrack(track: StandaloneTrack): Promise<void> {
	return invoke<void>('add_recent_standalone_track', { track })
}

/**
 * Remove a track from recent history.
 */
export async function removeRecentStandaloneTrack(id: string): Promise<void> {
	return invoke<void>('remove_recent_standalone_track', { id })
}

/**
 * Clear recent standalone tracks history.
 */
export async function clearRecentStandaloneTracks(): Promise<void> {
	return invoke<void>('clear_recent_standalone_tracks')
}

/**
 * Get startup file path if opened at launch.
 */
/** Files opened with Crate before the UI was ready (drains the queue; call after listening to `open-file`). */
export async function takeStartupFiles(): Promise<string[]> {
	return invoke<string[]>('take_startup_files')
}

/**
 * Play a standalone audio file with the Rodio backend engine.
 */
export async function playStandaloneTrack(path: string, id?: string, durationMs?: number): Promise<PlaybackState> {
	return invoke<PlaybackState>('play_standalone_track', { path, id, durationMs })
}
