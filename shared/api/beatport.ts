import { invoke } from '@tauri-apps/api/core'
import type {
	BeatportTrack,
	BeatportChart,
	BeatportGenre,
	BeatportPlaylist,
	BeatportAuthState,
	BeatportArtistDetail,
	BeatportSearchResult,
} from '../types/beatport'

export async function getBeatportPkceAuthUrl(): Promise<string> {
	return invoke<string>('beatport_get_pkce_auth_url')
}

export async function loginBeatportPkce(code: string): Promise<BeatportAuthState> {
	return invoke<BeatportAuthState>('beatport_login_pkce', { code })
}

export async function validateBeatportToken(token: string, refreshToken?: string | null): Promise<BeatportAuthState> {
	return invoke<BeatportAuthState>('beatport_validate_token', {
		token,
		refreshToken: refreshToken ?? null,
	})
}

export async function refreshBeatportToken(refreshToken: string): Promise<BeatportAuthState> {
	return invoke<BeatportAuthState>('beatport_refresh_token', { refreshToken })
}

export async function getBeatportGenres(token?: string | null): Promise<BeatportGenre[]> {
	return invoke<BeatportGenre[]>('beatport_get_genres', { token: token ?? null })
}

export async function getBeatportFeaturedCharts(token?: string | null): Promise<BeatportChart[]> {
	return invoke<BeatportChart[]>('beatport_get_featured_charts', { token: token ?? null })
}

export async function getBeatportTopTracks(token?: string | null, genreId?: number | null): Promise<BeatportTrack[]> {
	return invoke<BeatportTrack[]>('beatport_get_top_tracks', {
		token: token ?? null,
		genreId: genreId ?? null,
	})
}

export async function getBeatportChartTracks(
	token: string | null | undefined,
	chartId: string
): Promise<BeatportTrack[]> {
	return invoke<BeatportTrack[]>('beatport_get_chart_tracks', {
		token: token ?? null,
		chartId,
	})
}

export async function getBeatportArtistTracks(
	token: string | null | undefined,
	artistId: number
): Promise<BeatportTrack[]> {
	return invoke<BeatportTrack[]>('beatport_get_artist_tracks', {
		token: token ?? null,
		artistId,
	})
}

export async function getBeatportArtistDetail(
	token: string | null | undefined,
	artistId: number
): Promise<BeatportArtistDetail> {
	return invoke<BeatportArtistDetail>('beatport_get_artist_detail', {
		token: token ?? null,
		artistId,
	})
}

/** Searches the Beatport catalog; the result always contains both tracks and artists. */
export async function searchBeatport(token: string | null | undefined, query: string): Promise<BeatportSearchResult> {
	return invoke<BeatportSearchResult>('beatport_search', {
		token: token ?? null,
		query,
	})
}

export async function getBeatportUserPlaylists(token: string): Promise<BeatportPlaylist[]> {
	return invoke<BeatportPlaylist[]>('beatport_get_user_playlists', { token })
}

export async function getBeatportPlaylistTracks(
	token: string | null | undefined,
	playlistId: string
): Promise<BeatportTrack[]> {
	return invoke<BeatportTrack[]>('beatport_get_playlist_tracks', {
		token: token ?? null,
		playlistId,
	})
}

export async function getBeatportUserFavorites(token?: string | null): Promise<BeatportTrack[]> {
	return invoke<BeatportTrack[]>('beatport_get_user_favorites', {
		token: token ?? null,
	})
}

export async function getBeatportUserPurchases(token?: string | null): Promise<BeatportTrack[]> {
	return invoke<BeatportTrack[]>('beatport_get_user_purchases', {
		token: token ?? null,
	})
}

export interface BeatportDownloadResult {
	success_count: number
	failed_count: number
	downloaded_files: string[]
	errors: string[]
}

export async function downloadBeatportTracks(
	tracks: BeatportTrack[],
	destinationDir?: string,
	beatportdlPath?: string
): Promise<BeatportDownloadResult> {
	return invoke<BeatportDownloadResult>('beatport_download_tracks', {
		tracks,
		destinationDir: destinationDir ?? null,
		beatportdlPath: beatportdlPath ?? null,
	})
}

export async function getBeatportPersistedAuth(): Promise<BeatportAuthState | null> {
	return invoke<BeatportAuthState | null>('beatport_get_persisted_auth')
}

export async function saveBeatportPersistedAuth(auth: BeatportAuthState): Promise<void> {
	return invoke<void>('beatport_save_persisted_auth', { auth })
}

export async function clearBeatportPersistedAuth(): Promise<void> {
	return invoke<void>('beatport_clear_persisted_auth')
}
