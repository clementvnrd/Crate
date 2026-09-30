import type { Track, TrackFilter, TrackUpdate } from '$shared/types'
import type { HandlerMap } from '../types'
import type { HarnessState } from '../state'
import { DEVICES, cuesFor, waveformFor } from '../fixtures/library'

function matchesSearch(track: Track, search: string): boolean {
	const needle = search.trim().toLowerCase()
	if (!needle) return true
	return [track.title, track.artist, track.album, track.genre, track.label].some((field) =>
		field?.toLowerCase().includes(needle)
	)
}

function applyFilter(state: HarnessState, filter: TrackFilter | null | undefined): Track[] {
	if (!filter) return state.tracks
	let result = state.tracks
	if (filter.playlist_id) {
		const members = new Set(state.playlistTrackIds[filter.playlist_id] ?? [])
		result = result.filter((track) => members.has(track.id))
	}
	if (filter.search) result = result.filter((track) => matchesSearch(track, filter.search!))
	if (filter.tag_ids?.length) {
		const wanted = filter.tag_ids
		const tagIds = (track: Track) => new Set(track.tags.map((tag) => tag.id))
		result =
			filter.tag_filter_mode === 'and'
				? result.filter((track) => wanted.every((id) => tagIds(track).has(id)))
				: result.filter((track) => wanted.some((id) => tagIds(track).has(id)))
	}
	if (filter.bpm_min !== undefined) result = result.filter((track) => (track.bpm ?? 0) >= filter.bpm_min!)
	if (filter.bpm_max !== undefined) result = result.filter((track) => (track.bpm ?? Infinity) <= filter.bpm_max!)
	if (filter.energy_min !== undefined) result = result.filter((track) => (track.energy ?? 0) >= filter.energy_min!)
	if (filter.energy_max !== undefined) result = result.filter((track) => (track.energy ?? 99) <= filter.energy_max!)
	if (filter.keys?.length) result = result.filter((track) => track.key !== null && filter.keys!.includes(track.key))
	else if (filter.key) result = result.filter((track) => track.key === filter.key)
	return result
}

function requireTrack(state: HarnessState, id: unknown): Track {
	const track = state.tracks.find((entry) => entry.id === id)
	if (!track) throw `Track not found: ${String(id)}`
	return track
}

export function libraryHandlers(state: HarnessState): HandlerMap {
	return {
		get_tracks: ({ filter }) => applyFilter(state, filter as TrackFilter | null),
		get_track: ({ id }) => requireTrack(state, id),
		search_tracks: ({ query }) => state.tracks.filter((track) => matchesSearch(track, String(query ?? ''))),
		get_analyzed_tracks: ({ trackIds }) => state.tracks.filter((track) => (trackIds as string[]).includes(track.id)),
		get_track_waveform: ({ trackId }) => waveformFor(requireTrack(state, trackId)),
		get_track_cues: ({ trackId }) => cuesFor(requireTrack(state, trackId)),
		check_file_exists: () => true,

		update_track: ({ id, update }) => Object.assign(requireTrack(state, id), update as TrackUpdate),
		update_tracks: ({ ids, update }) =>
			(ids as string[]).map((id) => Object.assign(requireTrack(state, id), update as TrackUpdate)),
		set_track_colors: ({ trackIds, color }) => {
			for (const id of trackIds as string[]) requireTrack(state, id).color = color as Track['color']
			return null
		},
		delete_tracks: ({ ids }) => {
			const removed = new Set(ids as string[])
			state.tracks = state.tracks.filter((track) => !removed.has(track.id))
			return null
		},

		get_tag_categories: () => state.tagCategories,
		assign_tags: ({ trackIds, tagIds }) => {
			const allTags = state.tagCategories.flatMap((category) => category.tags)
			for (const id of trackIds as string[]) {
				const track = requireTrack(state, id)
				for (const tag of allTags.filter((entry) => (tagIds as string[]).includes(entry.id))) {
					if (!track.tags.some((existing) => existing.id === tag.id)) track.tags.push(tag)
				}
			}
			return null
		},
		remove_tags: ({ trackIds, tagIds }) => {
			for (const id of trackIds as string[]) {
				const track = requireTrack(state, id)
				track.tags = track.tags.filter((tag) => !(tagIds as string[]).includes(tag.id))
			}
			return null
		},

		get_playlists: ({ context }) => state.playlists.filter((playlist) => playlist.context === context),
		get_playlist_tracks: ({ playlistId }) => {
			const members = new Set(state.playlistTrackIds[String(playlistId)] ?? [])
			return state.tracks.filter((track) => members.has(track.id))
		},
		get_smart_playlist_tracks: ({ playlistId }) => {
			const members = new Set(state.playlistTrackIds[String(playlistId)] ?? [])
			return state.tracks.filter((track) => members.has(track.id))
		},
		preview_smart_rules_count: () => Math.min(state.tracks.length, 4),

		get_devices: () => DEVICES,
	}
}
