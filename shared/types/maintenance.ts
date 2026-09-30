// Types of the library-maintenance features: the discrepancy report and the assisted physical
// organisation. They mirror `library/report.rs` and `library/organise.rs` (snake_case).

/** A list capped at 500 items: `total` is the real count, `items` the first ones. */
export interface ReportSection<T> {
	total: number
	items: T[]
}

export interface ReportTrackRef {
	id: string | null
	title: string
	artist: string
	file_path: string | null
}

export interface FieldDifference {
	/** `key`, `bpm`, `energy`, `cues`… */
	field: string
	crate_value: string | null
	other_value: string | null
	note: string | null
}

export interface TrackDifference {
	track: ReportTrackRef
	fields: FieldDifference[]
}

/** Crate compared with one other tool (Mixed In Key or a Rekordbox export). */
export interface ReportComparison {
	matched: number
	differences: ReportSection<TrackDifference>
	missing_from_other: ReportSection<ReportTrackRef>
	missing_from_crate: ReportSection<ReportTrackRef>
}

/** `volume_unmounted`: the folder is gone too (an unplugged drive), so the file may well exist. */
export type MissingReason = 'missing' | 'volume_unmounted'

export interface MissingFile {
	track: ReportTrackRef
	reason: MissingReason
}

export interface DiscrepancyReport {
	crate_tracks: number
	missing_files: ReportSection<MissingFile>
	/** `null` when Mixed In Key is not installed or not readable. */
	mixed_in_key: ReportComparison | null
	/** `null` unless a Rekordbox XML export was given. */
	rekordbox: ReportComparison | null
}

export interface OrganisationRule {
	destination_root: string
	/**
	 * Naming template relative to the destination. It must contain `{title}` and must not start with a
	 * slash, for example `{artist}/{album}/{artist} - {title}`.
	 */
	template: string
}

export type MoveStatus = 'move' | 'already_in_place' | 'source_missing' | 'target_exists' | 'cross_volume'

export interface PlannedMove {
	track_id: string
	title: string
	artist: string
	from: string
	to: string
	status: MoveStatus
}

export interface OrganisationPlan {
	/** Fingerprint of the plan: applying it requires passing it back unchanged. */
	id: string
	rule: OrganisationRule
	moves: PlannedMove[]
	to_move: number
	already_in_place: number
	blocked: number
	/** Moving the files makes Rekordbox and other tools that remember paths lose them. */
	breaks_external_paths: boolean
}

export interface OrganisationFailure {
	track_id: string
	reason: string
}

export interface OrganisationResult {
	batch_id: string
	moved: number
	failed: OrganisationFailure[]
}

export interface OrganisationBatch {
	batch_id: string
	moved_at: string
	files: number
	undone: number
}
