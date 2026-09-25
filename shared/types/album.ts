export interface PlayerAlbum {
	id: string
	folder_path: string
	title: string
	artist: string
	year?: number | null
	genre?: string | null
	artwork_path?: string | null
	track_count: number
	total_duration_ms: number
	created_at: string
}

export interface PlayerAlbumTrack {
	id: string
	album_id: string
	file_path: string
	track_number?: number | null
	title: string
	artist: string
	duration_ms: number
	format: string
	bitrate?: number | null
	sample_rate?: number | null
	bpm?: number | null
	key?: string | null
	energy?: number | null
	artwork_path?: string | null
}

export interface AddAlbumResult {
	album: PlayerAlbum
	tracks: PlayerAlbumTrack[]
}
