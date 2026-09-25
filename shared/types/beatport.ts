export interface BeatportArtist {
	id: number
	name: string
	slug?: string
	image_url?: string
}

export interface BeatportArtistDetail {
	id: number
	name: string
	slug?: string
	image_url?: string
	biography?: string
	genres: string[]
	tracks_count?: number
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
	label?: string
}

export interface BeatportTrack {
	id: number | string
	title: string
	mix_name?: string
	artists: BeatportArtist[]
	remixers?: BeatportArtist[]
	genre: string
	genre_id?: number
	release_name?: string
	release_date: string
	duration_ms: number
	duration_formatted: string
	key?: string
	bpm?: number
	artwork_url?: string
	preview_url?: string
	waveform_url?: string
	is_favorite?: boolean
	in_cart?: boolean
	beatport_url?: string
}

export interface BeatportChart {
	id: number | string
	title: string
	subtitle?: string
	description?: string
	image_url: string
	genre_name?: string
	tracks_count?: number
}

export interface BeatportPlaylist {
	id: number | string
	name: string
	track_count: number
	image_url?: string
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

export interface BeatportAuthState {
	isAuthenticated: boolean
	username: string | null
	token: string | null
	refresh_token?: string | null
	refreshToken?: string | null
	hasSubscription: boolean
	subscriptionTier?: string
	is_authenticated?: boolean
	has_subscription?: boolean
	subscription_tier?: string
}

export interface BeatportSearchResult {
	tracks: BeatportTrack[]
	artists: BeatportArtist[]
}
