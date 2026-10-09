import { invoke } from '@tauri-apps/api/core'
import type { Track, TrackBeatGrid } from '../types'

/**
 * Analyze tracks for BPM and key detection
 * Progress events are emitted via 'analysis-track-event' Tauri event for each track
 */
export async function analyzeTracks(trackIds: string[]): Promise<void> {
	return invoke('analyze_tracks', { trackIds })
}

/**
 * Cancel analysis for a specific track
 * Returns true if the track was found and cancelled, false otherwise
 */
export async function cancelTrackAnalysis(trackId: string): Promise<boolean> {
	return invoke<boolean>('cancel_track_analysis', { trackId })
}

/**
 * Cancel all running analysis operations (legacy)
 */
export async function cancelAnalysis(): Promise<void> {
	return invoke('cancel_analysis')
}

/**
 * Get updated tracks after analysis
 */
export async function getAnalyzedTracks(trackIds: string[]): Promise<Track[]> {
	return invoke<Track[]>('get_analyzed_tracks', { trackIds })
}

/**
 * Stored beat grid of a track, or null when it has none (never analysed by Crate, analysed by
 * Mixed In Key only, or no detectable pulse)
 */
export async function getTrackBeatgrid(trackId: string): Promise<TrackBeatGrid | null> {
	return invoke<TrackBeatGrid | null>('get_track_beatgrid', { trackId })
}

/**
 * Recompute only the beat grid of a track from its audio file. BPM, key, energy and analysis
 * source are left untouched, so it is safe for tracks analysed by Mixed In Key.
 */
export async function analyzeTrackBeatgrid(trackId: string): Promise<TrackBeatGrid | null> {
	return invoke<TrackBeatGrid | null>('analyze_track_beatgrid', { trackId })
}
