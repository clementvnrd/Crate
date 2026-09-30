import type { HandlerMap } from '../types'
import type { HarnessState } from '../state'
import {
	BEATPORT_AUTH_LOGGED_IN,
	BEATPORT_CHARTS,
	BEATPORT_FAVORITES,
	BEATPORT_GENRES,
	BEATPORT_PLAYLISTS,
	BEATPORT_PURCHASES,
	BEATPORT_TRACKS,
	beatportArtistDetail,
	beatportTracksForArtist,
	beatportTracksForGenre,
	searchBeatport,
} from '../fixtures/beatport'

export function beatportHandlers(state: HarnessState): HandlerMap {
	return {
		// The Keychain session: logged in by default, `?beatport=out` starts logged out.
		beatport_get_persisted_auth: () => (state.beatportPersistedAuth ? BEATPORT_AUTH_LOGGED_IN : null),
		beatport_save_persisted_auth: () => {
			state.beatportPersistedAuth = true
			return null
		},
		beatport_clear_persisted_auth: () => {
			state.beatportPersistedAuth = false
			return null
		},
		beatport_login_pkce: () => {
			state.beatportPersistedAuth = true
			return BEATPORT_AUTH_LOGGED_IN
		},
		beatport_validate_token: () => BEATPORT_AUTH_LOGGED_IN,
		beatport_refresh_token: () => BEATPORT_AUTH_LOGGED_IN,
		beatport_get_pkce_auth_url: () => 'https://api.beatport.com/v4/auth/o/authorize/?harness=1',

		beatport_get_genres: () => BEATPORT_GENRES,
		beatport_get_featured_charts: () => BEATPORT_CHARTS,
		beatport_get_top_tracks: ({ genreId }) => beatportTracksForGenre(typeof genreId === 'number' ? genreId : null),
		beatport_get_chart_tracks: ({ chartId }) => {
			const index = BEATPORT_CHARTS.findIndex((chart) => String(chart.id) === String(chartId))
			return BEATPORT_TRACKS.slice(Math.max(index, 0) * 2, Math.max(index, 0) * 2 + 12)
		},
		beatport_get_artist_detail: ({ artistId }) => beatportArtistDetail(Number(artistId)),
		beatport_get_artist_tracks: ({ artistId }) => beatportTracksForArtist(Number(artistId)),
		beatport_search: ({ query }) => searchBeatport(String(query ?? '')),
		beatport_get_user_playlists: () => BEATPORT_PLAYLISTS,
		beatport_get_playlist_tracks: ({ playlistId }) => {
			const index = BEATPORT_PLAYLISTS.findIndex((playlist) => String(playlist.id) === String(playlistId))
			return BEATPORT_TRACKS.slice(Math.max(index, 0), Math.max(index, 0) + 9)
		},
		beatport_get_user_favorites: () => BEATPORT_FAVORITES,
		beatport_get_user_purchases: () => BEATPORT_PURCHASES,
		beatport_download_tracks: ({ tracks }) => ({
			success_count: (tracks as unknown[]).length,
			failed_count: 0,
			downloaded_files: [],
			errors: [],
		}),
	}
}
