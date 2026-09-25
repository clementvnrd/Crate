export type DuplicateMatchType = 'exact_hash' | 'metadata'

export interface DuplicateTrackInfo {
	id: string
	title: string | null
	artist: string | null
	album: string | null
	year: number | null
	genre: string | null
	label: string | null
	duration_ms: number
	bpm: number | null
	key: string | null
	energy: number | null
	bitrate: number | null
	sample_rate: number | null
	format: string
	file_path: string
	file_hash: string | null
	file_size_bytes: number
	cue_count: number
	rating: number
	play_count: number
	artwork_path: string | null
	date_added: string
	quality_score: number
	recommended_keep: boolean
}

export interface DuplicateGroup {
	id: string
	match_type: DuplicateMatchType
	tracks: DuplicateTrackInfo[]
	reclaimable_bytes: number
}

export interface DuplicateScanResult {
	groups: DuplicateGroup[]
	total_duplicate_tracks: number
	total_groups: number
	total_reclaimable_bytes: number
}

export interface DuplicateCountInfo {
	group_count: number
	track_count: number
	reclaimable_bytes: number
}
