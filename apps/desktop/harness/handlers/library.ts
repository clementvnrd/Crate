import type {
	HarmonicRelation,
	NextTrackSuggestion,
	SetAnalysis,
	SetEntry,
	Track,
	TrackFilter,
	TrackUpdate,
} from '$shared/types'
import type { HandlerMap } from '../types'
import type { HarnessState } from '../state'
import { DEVICES, beatGridFor, cuesFor, waveformFor } from '../fixtures/library'

const ENERGY_JUMP = 3

/** Camelot wheel relation between two keys, the same vocabulary `set_planner.rs` uses. A rough
 * stand-in for the real harmonic logic — good enough for Set-mode scenarios in the harness. */
function harmonicRelation(a: string | null, b: string | null): HarmonicRelation {
	if (!a || !b) return 'unknown'
	if (a === b) return 'same'
	const match = (key: string) => /^(\d{1,2})([AB])$/.exec(key)
	const parsedA = match(a)
	const parsedB = match(b)
	if (!parsedA || !parsedB) return 'unknown'
	const [, numA, letterA] = parsedA
	const [, numB, letterB] = parsedB
	if (numA === numB && letterA !== letterB) return 'relative'
	if (letterA === letterB) {
		const diff = Math.abs(Number(numA) - Number(numB))
		if (diff === 1 || diff === 11) return 'adjacent'
	}
	return 'clash'
}

function analyze(tracks: Track[]): SetAnalysis {
	let harmonic = 0
	let clashing = 0
	let unknown = 0
	let energyJumps = 0
	let totalDuration = 0
	let bpmMin: number | null = null
	let bpmMax: number | null = null

	const entries: SetEntry[] = tracks.map((track, index) => {
		totalDuration += track.duration_ms
		if (track.bpm !== null) {
			bpmMin = bpmMin === null ? track.bpm : Math.min(bpmMin, track.bpm)
			bpmMax = bpmMax === null ? track.bpm : Math.max(bpmMax, track.bpm)
		}

		const previous = index > 0 ? tracks[index - 1] : null
		let fromPrevious: SetEntry['from_previous'] = null
		if (previous) {
			const relation = harmonicRelation(previous.key, track.key)
			if (relation === 'clash') clashing++
			else if (relation === 'unknown') unknown++
			else harmonic++

			const bpmDelta =
				previous.bpm !== null && track.bpm !== null && previous.bpm !== 0
					? ((track.bpm - previous.bpm) / previous.bpm) * 100
					: null
			const energyDelta = previous.energy !== null && track.energy !== null ? track.energy - previous.energy : null
			const energyJump = energyDelta !== null && Math.abs(energyDelta) >= ENERGY_JUMP
			if (energyJump) energyJumps++

			fromPrevious = {
				harmonic: relation,
				bpm_delta_percent: bpmDelta,
				energy_delta: energyDelta,
				energy_jump: energyJump,
				bridges: [],
			}
		}

		return {
			position: index + 1,
			track_id: track.id,
			title: track.title ?? 'Untitled',
			artist: track.artist ?? 'Unknown Artist',
			key: track.key,
			bpm: track.bpm,
			energy: track.energy,
			duration_ms: track.duration_ms,
			from_previous: fromPrevious,
		}
	})

	return {
		entries,
		total_duration_ms: totalDuration,
		harmonic_transitions: harmonic,
		clashing_transitions: clashing,
		unknown_transitions: unknown,
		energy_jumps: energyJumps,
		bpm_min: bpmMin,
		bpm_max: bpmMax,
	}
}

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

// =============================================================================
// Next-track suggestions (CRA-133)
// =============================================================================
// The harness has no Rekordbox listen-history table, so "history" suggestions come from a small
// static map pinned to a couple of library tracks; everything else falls back to the same
// same-key-then-tempo logic as the real backend (`suggest.rs`), for a realistic ranked list.

const CAMELOT = /^([1-9]|1[0-2])([AB])$/i
const PITCH_RANGE_PERCENT = 6

function relation(a: string | null, b: string | null): HarmonicRelation {
	const ma = a?.trim().match(CAMELOT)
	const mb = b?.trim().match(CAMELOT)
	if (!ma || !mb) return 'unknown'
	const numA = parseInt(ma[1], 10)
	const numB = parseInt(mb[1], 10)
	const letterA = ma[2].toUpperCase()
	const letterB = mb[2].toUpperCase()
	if (numA === numB && letterA === letterB) return 'same'
	if (numA === numB) return 'relative'
	const adjacent = [numA === 12 ? 1 : numA + 1, numA === 1 ? 12 : numA - 1]
	if (letterA === letterB && adjacent.includes(numB)) return 'adjacent'
	return 'clash'
}

function bpmDeltaPercent(current: number | null, candidate: number | null): number | null {
	if (!current || current <= 0 || candidate === null) return null
	return ((candidate - current) / current) * 100
}

/** A fixed "you've played this after it" transition, so the panel has a realistic history-sourced
 * row in screenshots and e2e without a real listen-history store behind it. `trk-04` (10B) is a key
 * clash against `trk-01` (8A): the owner mixed into it anyway, which `trk-06` (9A, adjacent and in
 * range) is left free to illustrate as the compatible fallback below it. */
const SUGGESTION_HISTORY: Record<string, { id: string; times: number; lastAt: string }[]> = {
	'trk-01': [{ id: 'trk-04', times: 1, lastAt: '2026-09-06 23:02:00' }],
}

function compatibleIds(state: HarnessState, current: Track, taken: Set<string>, want: number): string[] {
	if (want <= 0 || !current.bpm || current.bpm <= 0) return []
	const spread = (current.bpm * PITCH_RANGE_PERCENT) / 100
	return state.tracks
		.filter(
			(track) =>
				!taken.has(track.id) &&
				track.bpm !== null &&
				track.bpm >= current.bpm! - spread &&
				track.bpm <= current.bpm! + spread
		)
		.map((track) => {
			const rel = relation(current.key, track.key)
			const level = rel === 'same' ? 0 : rel === 'adjacent' || rel === 'relative' ? 1 : rel === 'unknown' ? 2 : -1
			return { id: track.id, level, delta: Math.abs((track.bpm ?? 0) - current.bpm!) }
		})
		.filter((entry) => entry.level >= 0)
		.sort((a, b) => a.level - b.level || a.delta - b.delta || a.id.localeCompare(b.id))
		.slice(0, want)
		.map((entry) => entry.id)
}

function suggestNextTracks(state: HarnessState, trackId: unknown, limit: unknown): NextTrackSuggestion[] {
	const current = requireTrack(state, trackId)
	const max = typeof limit === 'number' ? limit : 10
	const taken = new Set([current.id])
	const suggestions: NextTrackSuggestion[] = []

	for (const entry of SUGGESTION_HISTORY[current.id] ?? []) {
		if (suggestions.length >= max) break
		if (taken.has(entry.id)) continue
		const track = state.tracks.find((candidate) => candidate.id === entry.id)
		if (!track) continue
		taken.add(track.id)
		suggestions.push({
			track,
			source: 'history',
			times_played_after: entry.times,
			last_played_after: entry.lastAt,
			harmonic: relation(current.key, track.key),
			bpm_delta_percent: bpmDeltaPercent(current.bpm, track.bpm),
		})
	}

	if (suggestions.length < max) {
		for (const id of compatibleIds(state, current, taken, max - suggestions.length)) {
			const track = state.tracks.find((candidate) => candidate.id === id)
			if (!track) continue
			suggestions.push({
				track,
				source: 'compatible',
				times_played_after: 0,
				last_played_after: null,
				harmonic: relation(current.key, track.key),
				bpm_delta_percent: bpmDeltaPercent(current.bpm, track.bpm),
			})
		}
	}

	return suggestions
}

export function libraryHandlers(state: HarnessState): HandlerMap {
	return {
		get_tracks: ({ filter }) => applyFilter(state, filter as TrackFilter | null),
		get_track: ({ id }) => requireTrack(state, id),
		search_tracks: ({ query }) => state.tracks.filter((track) => matchesSearch(track, String(query ?? ''))),
		get_analyzed_tracks: ({ trackIds }) => state.tracks.filter((track) => (trackIds as string[]).includes(track.id)),
		get_track_waveform: ({ trackId }) => waveformFor(requireTrack(state, trackId)),
		get_track_beatgrid: ({ trackId }) => {
			const track = state.tracks.find((candidate) => candidate.id === trackId)
			return track ? beatGridFor(track) : null
		},
		analyze_track_beatgrid: ({ trackId }) => beatGridFor(requireTrack(state, trackId)),
		get_track_cues: ({ trackId }) => cuesFor(requireTrack(state, trackId)),
		check_file_exists: ({ trackId }) => !state.params.missingTrackIds.includes(String(trackId)),

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

		suggest_next_tracks: ({ trackId, limit }) => suggestNextTracks(state, trackId, limit),

		get_devices: () => DEVICES,

		analyze_set: ({ trackIds }) => analyze((trackIds as string[]).map((id) => requireTrack(state, id))),
		suggest_set_order: ({ trackIds }) => {
			const tracks = (trackIds as string[]).map((id) => requireTrack(state, id))
			// Calmest (lowest energy, then lowest BPM) first, as `set_planner.rs`'s nearest-neighbour
			// pass also starts there — not the real 2-opt route, but a deterministic, clearly
			// different order from whatever the caller passed in.
			const sorted = [...tracks].sort((a, b) => (a.energy ?? 0) - (b.energy ?? 0) || (a.bpm ?? 0) - (b.bpm ?? 0))
			return sorted.map((track) => track.id)
		},
	}
}
