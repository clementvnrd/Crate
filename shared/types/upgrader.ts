import type { BeatportTrack } from './beatport'

export interface UpgradeScoreBreakdown {
	title_score: number
	artist_score: number
	duration_score: number
	bpm_score: number
	key_score: number
}

export interface UpgradeMatch {
	track_id: string
	file_path: string
	current_format: string
	current_bitrate: number | null
	current_sample_rate: number | null
	current_duration_ms: number
	current_bpm: number | null
	current_key: string | null
	current_energy: number | null
	current_artwork_path: string | null
	current_file_size_bytes: number
	title: string
	artist: string
	album: string | null
	beatport_track: BeatportTrack
	confidence_score: number
	score_breakdown: UpgradeScoreBreakdown
	alternative_candidates?: BeatportTrack[]
}

export interface UpgradeScanResult {
	matches: UpgradeMatch[]
	total_scanned: number
	total_eligible_mp3s: number
	potential_upgrades_count: number
}

export interface UpgradeReplacementResult {
	success_count: number
	failed_count: number
	replaced_tracks: string[]
	errors: string[]
}

export interface UpgradeCountInfo {
	match_count: number
	eligible_mp3_count: number
}

/** Progress event `upgrade-progress`, emitted before each track of an upgrade batch. */
export interface UpgradeProgress {
	current: number
	total: number
	title: string
}
