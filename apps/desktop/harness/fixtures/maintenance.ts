import type {
	DiscrepancyReport,
	DuplicateGroup,
	DuplicateScanResult,
	DuplicateTrackInfo,
	MissingFile,
	ReportComparison,
	ReportSection,
	ReportTrackRef,
	Track,
} from '$shared/types'
import { isoAgo } from './reference'
import { TRACKS } from './library'

// Duplicate Killer: two groups built from library tracks (an exact-hash pair and a metadata-match triple).

function trackById(id: string): Track {
	const track = TRACKS.find((entry) => entry.id === id)
	if (!track) throw new Error(`[harness] unknown fixture track ${id}`)
	return track
}

function duplicateInfo(
	track: Track,
	overrides: Partial<DuplicateTrackInfo> & Pick<DuplicateTrackInfo, 'id' | 'recommended_keep' | 'quality_score'>
): DuplicateTrackInfo {
	return {
		title: track.title,
		artist: track.artist,
		album: track.album,
		year: track.year,
		genre: track.genre,
		label: track.label,
		duration_ms: track.duration_ms,
		bpm: track.bpm,
		key: track.key,
		energy: track.energy,
		bitrate: track.bitrate,
		sample_rate: track.sample_rate,
		format: track.format,
		file_path: track.file_path,
		file_hash: track.file_hash,
		file_size_bytes: Math.round((track.duration_ms / 1000) * (track.format === 'mp3' ? 40_000 : 120_000)),
		cue_count: 0,
		rating: track.rating,
		play_count: track.play_count,
		artwork_path: track.artwork_path,
		date_added: track.date_added,
		...overrides,
	}
}

const paperLanterns = trackById('trk-02')
const goldenHour = trackById('trk-07')

const GROUPS: DuplicateGroup[] = [
	{
		id: 'dup-group-1',
		match_type: 'exact_hash',
		tracks: [
			duplicateInfo(paperLanterns, { id: paperLanterns.id, recommended_keep: true, quality_score: 92, cue_count: 4 }),
			duplicateInfo(paperLanterns, {
				id: 'trk-02-copy',
				file_path: '/Volumes/Harness/Downloads/Hollow Tide - Paper Lanterns (1).mp3',
				recommended_keep: false,
				quality_score: 88,
				play_count: 0,
				rating: 0,
				date_added: isoAgo(2),
			}),
		],
		reclaimable_bytes: 17_000_000,
	},
	{
		id: 'dup-group-2',
		match_type: 'metadata',
		tracks: [
			duplicateInfo(goldenHour, { id: goldenHour.id, recommended_keep: true, quality_score: 84, cue_count: 2 }),
			duplicateInfo(goldenHour, {
				id: 'trk-07-mp3',
				file_path: '/Volumes/Harness/Old Library/Sunset Regulars - Golden Hour Edit.mp3',
				format: 'mp3',
				bitrate: 192,
				recommended_keep: false,
				quality_score: 61,
				date_added: isoAgo(120),
			}),
			duplicateInfo(goldenHour, {
				id: 'trk-07-wav',
				file_path: '/Volumes/Harness/Exports/Golden Hour Edit.wav',
				format: 'wav',
				bitrate: 1411,
				file_size_bytes: 57_000_000,
				recommended_keep: false,
				quality_score: 79,
				date_added: isoAgo(45),
			}),
		],
		reclaimable_bytes: 71_000_000,
	},
]

export const DUPLICATE_SCAN: DuplicateScanResult = {
	groups: GROUPS,
	total_duplicate_tracks: GROUPS.reduce((sum, group) => sum + group.tracks.length - 1, 0),
	total_groups: GROUPS.length,
	total_reclaimable_bytes: GROUPS.reduce((sum, group) => sum + group.reclaimable_bytes, 0),
}

// Discrepancy report (CRA-129): Mixed In Key is always available in the harness, Rekordbox only once
// an XML export path is given. Read only — nothing here is ever mutated or deleted.

function trackRef(id: string): ReportTrackRef {
	const track = trackById(id)
	return { id: track.id, title: track.title ?? '', artist: track.artist ?? '', file_path: track.file_path }
}

const MISSING_FILES: ReportSection<MissingFile> = {
	total: 2,
	items: [
		{ track: trackRef('trk-04'), reason: 'missing' },
		{ track: trackRef('trk-06'), reason: 'volume_unmounted' },
	],
}

const MIK_COMPARISON: ReportComparison = {
	matched: TRACKS.length - 2,
	differences: {
		total: 1,
		items: [
			{
				track: trackRef('trk-08'),
				fields: [
					{ field: 'key', crate_value: '8A', other_value: '9A', note: null },
					{ field: 'bpm', crate_value: '128.00', other_value: '64.00', note: 'half_or_double_time' },
				],
			},
		],
	},
	missing_from_other: { total: 1, items: [trackRef('trk-10')] },
	missing_from_crate: {
		total: 1,
		items: [
			{
				id: null,
				title: 'Analog Dreams',
				artist: 'Night Lab',
				file_path: '/Volumes/Harness/MIK Only/Analog Dreams.mp3',
			},
		],
	},
}

const REKORDBOX_COMPARISON: ReportComparison = {
	matched: TRACKS.length - 1,
	differences: { total: 0, items: [] },
	missing_from_other: { total: 1, items: [trackRef('trk-11')] },
	missing_from_crate: { total: 0, items: [] },
}

const CLEAN_COMPARISON: ReportComparison = {
	matched: 0,
	differences: { total: 0, items: [] },
	missing_from_other: { total: 0, items: [] },
	missing_from_crate: { total: 0, items: [] },
}

/** `rekordboxXmlPath` mirrors the real command: give one to include the Rekordbox comparison. */
export function discrepancyReport(libraryEmpty: boolean, rekordboxXmlPath: string | null): DiscrepancyReport {
	if (libraryEmpty) {
		return {
			crate_tracks: 0,
			missing_files: { total: 0, items: [] },
			mixed_in_key: CLEAN_COMPARISON,
			rekordbox: rekordboxXmlPath ? CLEAN_COMPARISON : null,
		}
	}
	return {
		crate_tracks: TRACKS.length,
		missing_files: MISSING_FILES,
		mixed_in_key: MIK_COMPARISON,
		rekordbox: rekordboxXmlPath ? REKORDBOX_COMPARISON : null,
	}
}
