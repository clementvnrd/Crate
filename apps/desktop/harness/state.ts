import type { AppSettings, PlaybackState, Playlist, TagCategory, Track } from '$shared/types'
import type { HarnessParams } from './types'
import { createSettings } from './fixtures/system'
import { PLAYLISTS, PLAYLIST_TRACK_IDS, TAG_CATEGORIES, TRACKS } from './fixtures/library'

/** One file moved by an applied organisation run (see `handlers/maintenance.ts`). */
export interface OrganisationJournalEntry {
	track_id: string
	from: string
	to: string
	undone: boolean
}

/** An applied organisation run: every move it made, so it can be undone, in part or fully. */
export interface OrganisationBatchRecord {
	batch_id: string
	moved_at: string
	entries: OrganisationJournalEntry[]
}

/** The fake backend's in-memory "database": a mutable copy of the fixtures, reset on every page load. */
export interface HarnessState {
	params: HarnessParams
	settings: AppSettings
	tracks: Track[]
	playlists: Playlist[]
	playlistTrackIds: Record<string, string[]>
	tagCategories: TagCategory[]
	playback: PlaybackState
	/** `Date.now()` when the current playback (re)started, or null while paused/stopped. */
	playbackStartedAt: number | null
	beatportPersistedAuth: boolean
	/** Applied assisted-organisation runs (CRA-130), latest first. */
	organisationBatches: OrganisationBatchRecord[]
	organisationBatchCounter: number
}

export function createState(params: HarnessParams): HarnessState {
	const tracks = params.libraryEmpty ? [] : structuredClone(TRACKS)
	const playlistTrackIds = params.libraryEmpty
		? Object.fromEntries(Object.keys(PLAYLIST_TRACK_IDS).map((id) => [id, [] as string[]]))
		: structuredClone(PLAYLIST_TRACK_IDS)
	const playlists = structuredClone(PLAYLISTS).map((playlist) => ({
		...playlist,
		track_count: playlistTrackIds[playlist.id]?.length ?? 0,
	}))
	return {
		params,
		settings: createSettings(params),
		tracks,
		playlists,
		playlistTrackIds,
		tagCategories: structuredClone(TAG_CATEGORIES),
		playback: {
			is_playing: false,
			position_ms: 0,
			duration_ms: 0,
			volume: 1,
			speed: 1,
			current_track_id: null,
			current_track_path: null,
		},
		playbackStartedAt: null,
		beatportPersistedAuth: params.beatportLoggedIn,
		organisationBatches: [],
		organisationBatchCounter: 0,
	}
}
