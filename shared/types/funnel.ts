// The discovery funnel, see `services/discovery/funnel.rs` (snake_case, like the Rust structs).
// Nothing records which library track a release became, so the stages are matched by names: a
// release is in the library when a library track has the same artist and the release's title (as
// track title or album) or one of its track names, and played in a set when such a track appears
// in a Rekordbox set. Only exact matches count, ignoring case and surrounding spaces.

export interface FunnelStages {
	/** Releases found on the pages the user scanned (Bandcamp, SoundCloud, Beatport…). */
	discovered: number
	/** … that match a track of the library. */
	in_library: number
	/** … that match a track played in a Rekordbox set. */
	played_in_set: number
}

export interface FunnelSource {
	source_type: string
	stages: FunnelStages
}

export interface DiscoveryFunnel {
	/** First day counted (`YYYY-MM-DD`): only releases added on or after it. `null` for all time. */
	since: string | null
	total: FunnelStages
	/** By source, the most discovered first. */
	by_source: FunnelSource[]
}
