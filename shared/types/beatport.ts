export interface BeatportArtist {
	id: number
	name: string
	slug?: string | null
	image_url?: string | null
}

export interface BeatportArtistDetail {
	id: number
	name: string
	slug?: string | null
	image_url?: string | null
	biography?: string | null
	genres: string[]
	tracks_count?: number | null
}

export interface BeatportGenre {
	id: number
	name: string
	slug: string
}

export interface BeatportRelease {
	id: number
	name: string
	image_url: string
	label?: string | null
}

export interface BeatportTrack {
	id: number | string
	title: string
	mix_name?: string | null
	artists: BeatportArtist[]
	remixers?: BeatportArtist[] | null
	genre: string
	genre_id?: number | null
	release_name?: string | null
	release_date: string
	duration_ms: number
	duration_formatted: string
	key?: string | null
	bpm?: number | null
	artwork_url?: string | null
	preview_url?: string | null
	waveform_url?: string | null
	is_favorite?: boolean
	in_cart?: boolean
	beatport_url?: string | null
}

export interface BeatportChart {
	id: number | string
	title: string
	subtitle?: string | null
	description?: string | null
	image_url: string
	genre_name?: string | null
	tracks_count?: number | null
}

export interface BeatportPlaylist {
	id: number | string
	name: string
	track_count: number
	image_url?: string | null
	is_public?: boolean
}

export interface BeatportCartItem {
	track: BeatportTrack
	addedAt: number
}

export type DownloadStatus = 'idle' | 'queued' | 'downloading' | 'importing' | 'completed' | 'failed'

export interface BeatportDownloadTask {
	id: string
	trackId: string | number
	title: string
	artist: string
	progress: number
	status: DownloadStatus
	filePath?: string
	errorMessage?: string
}

/** Mirror of the Rust `BeatportAuthState` (snake_case, as serialized by the backend). */
export interface BeatportAuthState {
	is_authenticated: boolean
	username: string | null
	token: string | null
	refresh_token: string | null
	has_subscription: boolean
	subscription_tier: string | null
}

export interface BeatportSearchResult {
	tracks: BeatportTrack[]
	artists: BeatportArtist[]
}
