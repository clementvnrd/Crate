// Types of the DJ-preparation features: harmonic relations, next-track suggestions and the
// analysis of a set. They mirror `services/harmonic.rs`, `library/suggest.rs` and
// `library/set_planner.rs` field for field (snake_case).

import type { Track } from './index'

/** How a second key sits relative to the first on the Camelot wheel. */
export type HarmonicRelation = 'same' | 'adjacent' | 'relative' | 'clash' | 'unknown'

/** Where a suggestion comes from: a transition you really played, or only a compatible track. */
export type SuggestionSource = 'history' | 'compatible'

export interface NextTrackSuggestion {
	track: Track
	source: SuggestionSource
	/** How many times this track followed the current one in a Rekordbox set. */
	times_played_after: number
	last_played_after: string | null
	harmonic: HarmonicRelation
	bpm_delta_percent: number | null
}

/** A library track that could bridge a clashing or tempo-jumping transition. */
export interface SetBridge {
	id: string
	title: string
	artist: string
	key: string | null
	bpm: number | null
}

export interface SetTransition {
	harmonic: HarmonicRelation
	bpm_delta_percent: number | null
	energy_delta: number | null
	energy_jump: boolean
	/** Only filled for the transitions that clash or jump in tempo. */
	bridges: SetBridge[]
}

export interface SetEntry {
	/** 1-based position in the set. */
	position: number
	track_id: string
	title: string
	artist: string
	key: string | null
	bpm: number | null
	energy: number | null
	duration_ms: number
	/** How this track mixes out of the previous one; `null` for the first track. */
	from_previous: SetTransition | null
}

/** A DJ set checked in the given order. */
export interface SetAnalysis {
	entries: SetEntry[]
	total_duration_ms: number
	harmonic_transitions: number
	clashing_transitions: number
	unknown_transitions: number
	energy_jumps: number
	bpm_min: number | null
	bpm_max: number | null
}
