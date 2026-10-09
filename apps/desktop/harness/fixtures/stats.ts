import type {
	BpmBucketItem,
	HarmonicStatsItem,
	HeatmapCell,
	HarmonicRelation,
	ListenEvent,
	Recap,
	RecapPeriod,
	RekordboxSession,
	SessionTimeline,
	SessionTrack,
	StatsSummary,
	TimeRange,
	TopArtistItem,
	TopTrackItem,
} from '$shared/types'
import { rangeDays } from '$shared/utils/statsRange'
import { artworkDataUrl } from './artwork'
import { REFERENCE_NOW, isoAgo } from './reference'
import { TRACKS } from './library'

// Crate Pulse: one listening history, scaled by the selected time range (deterministic, no randomness).

const RANGE_SCALE: Record<string, number> = {
	today: 0.12,
	'7d': 1,
	'30d': 4.2,
	'3m': 12.6,
	'6m': 25,
	year: 38,
	all: 52,
}

/** Calendar years and custom windows scale with their length (a week is 1), capped below "all". */
function scale(range: TimeRange | string): number {
	const days = rangeDays(range)
	if (days !== undefined) return Math.min(Math.max(days / 7, 0.12), 52)
	return RANGE_SCALE[range] ?? 1
}

/** Years with listening data, newest first: the reference year and the two before it (none for an empty library). */
export function listeningYears(empty: boolean): number[] {
	if (empty) return []
	const year = new Date(REFERENCE_NOW).getUTCFullYear()
	return [year, year - 1, year - 2]
}

export function statsSummary(range: TimeRange | string): StatsSummary {
	const factor = scale(range)
	return {
		total_minutes: Math.round(742 * factor),
		today_minutes: 86,
		week_minutes: 742,
		month_minutes: 3118,
		total_plays: Math.round(198 * factor),
		source_breakdown: {
			crate_local: Math.round(301 * factor),
			rekordbox: Math.round(214 * factor),
			spotify: Math.round(128 * factor),
			crate_beatport: Math.round(64 * factor),
			mixed_in_key: Math.round(35 * factor),
		},
		unique_artists: Math.max(1, Math.round(24 * Math.sqrt(factor))),
		dj_sessions: Math.max(1, Math.round(3 * factor)),
		dj_sessions_played_ms: Math.round(9_840_000 * factor),
	}
}

const SOURCES = ['crate_local', 'rekordbox', 'spotify', 'crate_beatport', 'mixed_in_key']

const PLAYED_TRACKS = TRACKS.filter((track) => track.title !== null && track.play_count > 0)

export function topTracks(range: TimeRange | string, limit: number): TopTrackItem[] {
	const factor = scale(range)
	return [...PLAYED_TRACKS]
		.sort((a, b) => b.play_count - a.play_count)
		.slice(0, limit)
		.map((track, index) => ({
			title: track.title ?? '',
			artist: track.artist ?? '',
			album: track.album,
			artwork_url: track.artwork_path,
			plays: Math.max(1, Math.round(track.play_count * factor * 0.5)),
			total_minutes: Math.max(1, Math.round((track.duration_ms / 60_000) * track.play_count * factor * 0.45)),
			bpm: track.bpm,
			key: track.key,
			energy: track.energy,
			sources: [SOURCES[index % SOURCES.length], ...(index % 3 === 0 ? [SOURCES[(index + 1) % SOURCES.length]] : [])],
		}))
}

export function topArtists(range: TimeRange | string, limit: number): TopArtistItem[] {
	const factor = scale(range)
	const byArtist = new Map<string, { plays: number; topTrack: string; artwork: string | null }>()
	for (const track of [...PLAYED_TRACKS].sort((a, b) => b.play_count - a.play_count)) {
		const artist = track.artist ?? ''
		const entry = byArtist.get(artist)
		if (entry) entry.plays += track.play_count
		else byArtist.set(artist, { plays: track.play_count, topTrack: track.title ?? '', artwork: track.artwork_path })
	}
	return [...byArtist.entries()]
		.sort((a, b) => b[1].plays - a[1].plays)
		.slice(0, limit)
		.map(([artist, entry]) => ({
			artist,
			plays: Math.max(1, Math.round(entry.plays * factor * 0.5)),
			total_minutes: Math.max(1, Math.round(entry.plays * 6.2 * factor * 0.45)),
			top_track: entry.topTrack,
			artwork_url: entry.artwork,
		}))
}

const CAMELOT_PLAYS: [string, number][] = [
	['8A', 34],
	['5A', 27],
	['11A', 22],
	['10B', 19],
	['9A', 17],
	['2A', 14],
	['4B', 12],
	['1A', 11],
	['6A', 8],
	['12A', 7],
	['7B', 5],
	['3B', 3],
]

export function harmonicStats(range: TimeRange | string): HarmonicStatsItem[] {
	const factor = scale(range)
	const total = CAMELOT_PLAYS.reduce((sum, [, plays]) => sum + plays, 0)
	return CAMELOT_PLAYS.map(([key, plays]) => ({
		key,
		plays: Math.max(1, Math.round(plays * factor)),
		total_minutes: Math.max(1, Math.round(plays * 6.4 * factor)),
		percentage: Math.round((plays / total) * 1000) / 10,
	}))
}

const BPM_BUCKETS: [string, number][] = [
	['< 110', 9],
	['110-115', 14],
	['115-120', 31],
	['120-125', 58],
	['125-130', 47],
	['130-135', 26],
	['135-140', 12],
	['140+', 4],
]

export function bpmStats(range: TimeRange | string): BpmBucketItem[] {
	const factor = scale(range)
	return BPM_BUCKETS.map(([bpmRange, count]) => ({
		bpm_range: bpmRange,
		count: Math.max(1, Math.round(count * factor)),
		total_minutes: Math.max(1, Math.round(count * 6.1 * factor)),
	}))
}

/** 7 days × 24 hours; evenings and the weekend are busy, nights and mornings quiet. */
export function listeningHeatmap(range: TimeRange | string): HeatmapCell[] {
	const factor = scale(range)
	const cells: HeatmapCell[] = []
	for (let day = 0; day < 7; day++) {
		for (let hour = 0; hour < 24; hour++) {
			const evening = hour >= 18 && hour <= 23 ? 1 : 0
			const afternoon = hour >= 13 && hour <= 17 ? 0.4 : 0
			const weekend = day === 0 || day === 5 || day === 6 ? 1.6 : 1
			const late = (day === 5 || day === 6) && hour <= 3 ? 1.2 : 0
			const base = (evening + afternoon + late) * weekend
			const wave = ((day * 7 + hour * 3) % 5) / 5
			const minutes = base === 0 ? (hour % 6 === 0 ? 2 : 0) : Math.round((base * 22 + wave * 9) * factor)
			cells.push({ day_of_week: day, hour_of_day: hour, minutes, plays: Math.round(minutes / 5.5) })
		}
	}
	return cells
}

export function recentListens(limit: number): ListenEvent[] {
	return PLAYED_TRACKS.slice(0, Math.min(limit, 12)).map((track, index) => ({
		id: `listen-${index + 1}`,
		source: SOURCES[index % SOURCES.length],
		track_id: track.id,
		title: track.title ?? '',
		artist: track.artist ?? '',
		album: track.album,
		duration_ms: track.duration_ms,
		played_ms: Math.round(track.duration_ms * (index % 4 === 0 ? 0.6 : 1)),
		bpm: track.bpm,
		key: track.key,
		energy: track.energy,
		format: track.format,
		artwork_url: track.artwork_path ?? artworkDataUrl(90 + index),
		played_at: isoAgo(0, index * 3 + 1),
		session_id: index < 4 ? 'rb-session-1' : null,
		metadata_json: null,
	}))
}

export const REKORDBOX_SESSIONS: RekordboxSession[] = [
	{
		id: 'rb-session-1',
		session_name: 'Friday warehouse',
		started_at: isoAgo(3, 3),
		ended_at: isoAgo(3, 0),
		total_tracks: 28,
		total_played_ms: 9_840_000,
	},
	{
		id: 'rb-session-2',
		session_name: 'Sunday terrace',
		started_at: isoAgo(5, 6),
		ended_at: isoAgo(5, 3),
		total_tracks: 21,
		total_played_ms: 7_560_000,
	},
	{
		id: 'rb-session-3',
		session_name: null,
		started_at: isoAgo(12, 4),
		ended_at: isoAgo(12, 2),
		total_tracks: 14,
		total_played_ms: 5_100_000,
	},
]

// ------------------------------------------------------------------
// Recap, set timeline and history export (CRA-125, CRA-127, CRA-128)
// ------------------------------------------------------------------

const DAY_MS = 86_400_000

function isoDay(ms: number): string {
	return new Date(ms).toISOString().slice(0, 10)
}

/** The recap of a week (Monday to Sunday) or a year, `offset` periods before the one holding the reference day. */
export function recap(period: RecapPeriod, offset: number, empty: boolean): Recap {
	const reference = Date.parse(`${REFERENCE_NOW.slice(0, 10)}T00:00:00.000Z`)
	let start: number
	let end: number
	if (period === 'week') {
		const weekday = (new Date(reference).getUTCDay() + 6) % 7 // 0 = Monday
		start = reference - (weekday + offset * 7) * DAY_MS
		end = start + 6 * DAY_MS
	} else {
		const year = new Date(reference).getUTCFullYear() - offset
		start = Date.UTC(year, 0, 1)
		end = Date.UTC(year, 11, 31)
	}
	const base: Recap = {
		period,
		start_date: isoDay(start),
		end_date: isoDay(end),
		total_plays: 0,
		total_minutes: 0,
		unique_tracks: 0,
		unique_artists: 0,
		new_tracks: 0,
		previous_plays: 0,
		previous_minutes: 0,
		top_tracks: [],
		top_artists: [],
		top_keys: [],
		busiest_day: null,
		peak_hour: null,
		peak_weekday: null,
		source_minutes: {},
	}
	if (empty) return base
	const range = period === 'week' ? '7d' : 'year'
	// Older periods were a little quieter, so the change against the previous period stays readable.
	const decay = 1 - Math.min(offset, 4) * 0.12
	const plays = Math.round(198 * scale(range) * decay)
	const minutes = Math.round(742 * scale(range) * decay)
	const summary = statsSummary(range)
	return {
		...base,
		total_plays: plays,
		total_minutes: minutes,
		unique_tracks: Math.round(plays * 0.42),
		unique_artists: Math.round(plays * 0.19),
		new_tracks: Math.round(plays * 0.08),
		previous_plays: Math.round(plays * 0.86),
		previous_minutes: Math.round(minutes * 1.07),
		top_tracks: topTracks(range, 5),
		top_artists: topArtists(range, 5),
		top_keys: harmonicStats(range).slice(0, 3),
		busiest_day: {
			date: isoDay(start + 4 * DAY_MS),
			plays: Math.round(plays * 0.24),
			minutes: Math.round(minutes * 0.23),
		},
		peak_hour: 22,
		peak_weekday: 5,
		source_minutes: Object.fromEntries(
			Object.entries(summary.source_breakdown).map(([source, value]) => [source, Math.round(value * decay)])
		),
	}
}

const TRANSITIONS: HarmonicRelation[] = ['adjacent', 'same', 'relative', 'adjacent', 'clash', 'adjacent', 'unknown']

/** Mirrors the Rust threshold in `services::harmonic::ENERGY_JUMP`, shared with the set planner,
 * so this harness fixture agrees with the real app on what counts as a jump. */
const ENERGY_JUMP = 3

/** The tracks of one Rekordbox set in order, cycling through the played library tracks. */
export function sessionTimeline(sessionId: string): SessionTimeline | null {
	const session = REKORDBOX_SESSIONS.find((entry) => entry.id === sessionId)
	if (!session) return null
	const tracks: SessionTrack[] = []
	let playedAt = Date.parse(session.started_at)
	const count = session.total_tracks
	for (let index = 0; index < count; index++) {
		const track = PLAYED_TRACKS[(index * 5 + session.total_tracks) % PLAYED_TRACKS.length]
		const previous = tracks[index - 1]
		const relation = TRANSITIONS[(index + session.total_tracks) % TRANSITIONS.length]
		const bpmDelta =
			previous && previous.bpm && track.bpm ? Math.round(((track.bpm - previous.bpm) / previous.bpm) * 1000) / 10 : null
		// Only a library track has an energy (Rekordbox never records one), so the tracks Crate does not know show a dash.
		const energy = index % 6 === 5 ? null : track.energy
		const energyDelta = previous && previous.energy !== null && energy !== null ? energy - previous.energy : null
		const energyJump = energyDelta !== null && Math.abs(energyDelta) >= ENERGY_JUMP
		tracks.push({
			position: index + 1,
			played_at: new Date(playedAt).toISOString(),
			title: track.title ?? '',
			artist: track.artist ?? '',
			album: track.album,
			duration_ms: track.duration_ms,
			bpm: track.bpm,
			key: relation === 'unknown' ? null : track.key,
			energy,
			library_track_id: index % 6 === 5 ? null : track.id,
			from_previous:
				index === 0
					? null
					: { harmonic: relation, bpm_delta_percent: bpmDelta, energy_delta: energyDelta, energy_jump: energyJump },
		})
		playedAt += Math.round(track.duration_ms * 0.7)
	}
	const transitions = tracks.slice(1).map((entry) => entry.from_previous?.harmonic)
	return {
		session,
		tracks,
		harmonic_transitions: transitions.filter(
			(relation) => relation === 'same' || relation === 'adjacent' || relation === 'relative'
		).length,
		clashing_transitions: transitions.filter((relation) => relation === 'clash').length,
		unknown_transitions: transitions.filter((relation) => relation === 'unknown').length,
		energy_jumps: tracks.filter((entry) => entry.from_previous?.energy_jump).length,
	}
}

/** How many listens the history export writes. */
export const EXPORTED_LISTENS = 2_184
