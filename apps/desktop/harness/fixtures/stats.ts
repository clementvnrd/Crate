import type {
	BpmBucketItem,
	HarmonicStatsItem,
	HeatmapCell,
	ListenEvent,
	RekordboxSession,
	StatsSummary,
	TimeRange,
	TopArtistItem,
	TopTrackItem,
} from '$shared/types'
import { artworkDataUrl } from './artwork'
import { isoAgo } from './reference'
import { TRACKS } from './library'

// Crate Pulse: one listening history, scaled by the selected time range (deterministic, no randomness).

const RANGE_SCALE: Record<string, number> = { today: 0.12, '7d': 1, '30d': 4.2, year: 38, all: 52 }

function scale(range: TimeRange | string): number {
	return RANGE_SCALE[range] ?? 1
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
