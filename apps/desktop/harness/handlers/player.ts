import type { PlaybackState, StandaloneTrack } from '$shared/types'
import type { HandlerMap } from '../types'
import type { HarnessState } from '../state'
import { ALBUMS, RECENT_STANDALONE_TRACKS, albumTracksFor } from '../fixtures/player'

/** The position of the fake audio engine: it advances in real time while playing. */
function currentPlayback(state: HarnessState): PlaybackState {
	const { playback, playbackStartedAt } = state
	if (!playback.is_playing || playbackStartedAt === null) return { ...playback }
	const elapsed = (Date.now() - playbackStartedAt) * playback.speed
	return { ...playback, position_ms: Math.min(playback.duration_ms, Math.round(playback.position_ms + elapsed)) }
}

function freeze(state: HarnessState): PlaybackState {
	state.playback = currentPlayback(state)
	state.playbackStartedAt = state.playback.is_playing ? Date.now() : null
	return { ...state.playback }
}

function start(state: HarnessState, trackId: string, path: string, durationMs: number): PlaybackState {
	state.playback = {
		...state.playback,
		is_playing: true,
		position_ms: 0,
		duration_ms: durationMs,
		current_track_id: trackId,
		current_track_path: path,
	}
	state.playbackStartedAt = Date.now()
	return { ...state.playback }
}

export function playerHandlers(state: HarnessState): HandlerMap {
	const recent: StandaloneTrack[] = structuredClone(RECENT_STANDALONE_TRACKS)
	return {
		get_playback_state: () => currentPlayback(state),
		play_track: ({ id }) => {
			const track = state.tracks.find((entry) => entry.id === id)
			if (!track) {
				// Album and Beatport tracks are not in the library: play them as a generic 5-minute track.
				return start(state, String(id), '', 300_000)
			}
			return start(state, track.id, track.file_path, track.duration_ms)
		},
		play_standalone_track: ({ path, id, durationMs }) =>
			start(state, String(id ?? path), String(path), Number(durationMs ?? 300_000)),
		pause: () => {
			freeze(state)
			state.playback.is_playing = false
			state.playbackStartedAt = null
			return { ...state.playback }
		},
		resume: () => {
			state.playback.is_playing = state.playback.duration_ms > 0
			state.playbackStartedAt = state.playback.is_playing ? Date.now() : null
			return { ...state.playback }
		},
		stop: () => {
			state.playback = { ...state.playback, is_playing: false, position_ms: 0 }
			state.playbackStartedAt = null
			return { ...state.playback }
		},
		seek: ({ positionMs }) => {
			freeze(state)
			state.playback.position_ms = Math.max(0, Math.min(state.playback.duration_ms, Number(positionMs)))
			return { ...state.playback }
		},
		set_volume: ({ volume }) => {
			freeze(state)
			state.playback.volume = Number(volume)
			return { ...state.playback }
		},
		set_speed: ({ speed }) => {
			freeze(state)
			state.playback.speed = Number(speed)
			return { ...state.playback }
		},

		get_player_albums: () => ALBUMS,
		get_player_album_tracks: ({ albumId }) => albumTracksFor(String(albumId)),
		get_recent_standalone_tracks: ({ limit }) => recent.slice(0, typeof limit === 'number' ? limit : recent.length),
		add_recent_standalone_track: ({ track }) => {
			const entry = track as StandaloneTrack
			const index = recent.findIndex((item) => item.id === entry.id)
			if (index >= 0) recent.splice(index, 1)
			recent.unshift(entry)
			return null
		},
		remove_recent_standalone_track: ({ id }) => {
			const index = recent.findIndex((item) => item.id === id)
			if (index >= 0) recent.splice(index, 1)
			return null
		},
		clear_recent_standalone_tracks: () => {
			recent.length = 0
			return null
		},
		take_startup_files: () => [],
	}
}
