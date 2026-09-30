import type { Cue, Playlist, Tag, TagCategory, Track, TrackColor, UsbDevice } from '$shared/types'
import { artworkDataUrl } from './artwork'
import { isoAgo } from './reference'

// =============================================================================
// Tags
// =============================================================================

function tag(categoryId: string, slug: string, name: string, color: string, sortOrder: number): Tag {
	return { id: `tag-${slug}`, category_id: categoryId, name, color, sort_order: sortOrder }
}

const MOOD_COLOR = '#8b5cf6'
const SET_COLOR = '#14b8a6'
const STYLE_COLOR = '#f97316'

export const TAG_CATEGORIES: TagCategory[] = [
	{
		id: 'cat-mood',
		name: 'Mood',
		color: MOOD_COLOR,
		sort_order: 0,
		tags: [
			tag('cat-mood', 'dark', 'Dark', MOOD_COLOR, 0),
			tag('cat-mood', 'euphoric', 'Euphoric', MOOD_COLOR, 1),
			tag('cat-mood', 'melancholic', 'Melancholic', MOOD_COLOR, 2),
			tag('cat-mood', 'uplifting', 'Uplifting', MOOD_COLOR, 3),
		],
	},
	{
		id: 'cat-set',
		name: 'Set position',
		color: SET_COLOR,
		sort_order: 1,
		tags: [
			tag('cat-set', 'warmup', 'Warm-up', SET_COLOR, 0),
			tag('cat-set', 'peak', 'Peak time', SET_COLOR, 1),
			tag('cat-set', 'closing', 'Closing', SET_COLOR, 2),
		],
	},
	{
		id: 'cat-style',
		name: 'Style',
		color: STYLE_COLOR,
		sort_order: 2,
		tags: [
			tag('cat-style', 'deep', 'Deep', STYLE_COLOR, 0),
			tag('cat-style', 'driving', 'Driving', STYLE_COLOR, 1),
			tag('cat-style', 'hypnotic', 'Hypnotic', STYLE_COLOR, 2),
			tag('cat-style', 'groovy', 'Groovy', STYLE_COLOR, 3),
		],
	},
]

const ALL_TAGS = new Map(TAG_CATEGORIES.flatMap((category) => category.tags).map((entry) => [entry.id, entry]))

function tagsFor(slugs: string[]): Tag[] {
	return slugs.map((slug) => ALL_TAGS.get(`tag-${slug}`)).filter((entry): entry is Tag => entry !== undefined)
}

// =============================================================================
// Tracks
// =============================================================================

interface TrackSeed {
	title: string | null
	artist: string | null
	album: string | null
	genre: string
	label: string | null
	bpm: number | null
	key: string | null
	energy: number | null
	format: string
	durationSeconds: number
	year: number | null
	rating: number
	plays: number
	color: TrackColor | null
	/** Seed of the generated cover, or null for a track without artwork. */
	cover: number | null
	tags: string[]
}

// Bitrates in kbps, as the backend stores them (the duplicate service falls back to 320).
const BITRATE_BY_FORMAT: Record<string, number> = {
	mp3: 320,
	flac: 987,
	wav: 1411,
	aiff: 1411,
	m4a: 256,
}

// Stress cases on purpose: very long title (track 3), very long artist (4), missing artwork (5, 9, 12, 16),
// a track with no metadata at all (16), every format, BPM 90–138, all twelve Camelot numbers.
const SEEDS: TrackSeed[] = [
	{
		title: 'Afterglow Protocol',
		artist: 'Nova Kline',
		album: 'Signal Loss EP',
		genre: 'Melodic Techno',
		label: 'Low Orbit',
		bpm: 124,
		key: '8A',
		energy: 7,
		format: 'flac',
		durationSeconds: 402,
		year: 2025,
		rating: 5,
		plays: 14,
		color: 'aqua',
		cover: 1,
		tags: ['euphoric', 'peak', 'driving'],
	},
	{
		title: 'Paper Lanterns',
		artist: 'Hollow Tide',
		album: 'Quiet Harbour',
		genre: 'Deep House',
		label: 'Tidewater Records',
		bpm: 120,
		key: '5A',
		energy: 5,
		format: 'mp3',
		durationSeconds: 425,
		year: 2024,
		rating: 4,
		plays: 9,
		color: null,
		cover: 2,
		tags: ['melancholic', 'warmup', 'deep'],
	},
	{
		title: 'Everything You Ever Wanted To Hear In A Very Long Extended Dub Version (Club Mix Extended Remaster 2024)',
		artist: 'Marlo Vance & The Static Choir',
		album: 'The Complete Dub Sessions (Deluxe Anniversary Edition)',
		genre: 'Dub Techno',
		label: 'Basement Ritual',
		bpm: 118,
		key: '11A',
		energy: 4,
		format: 'wav',
		durationSeconds: 570,
		year: 2024,
		rating: 3,
		plays: 3,
		color: 'purple',
		cover: 3,
		tags: ['dark', 'hypnotic'],
	},
	{
		title: 'Neon Rain',
		artist: 'DJ Longname Featuring The Extraordinarily Named Collective & Friends',
		album: 'After Hours Vol. 2',
		genre: 'Progressive House',
		label: 'Night Shift',
		bpm: 126,
		key: '10B',
		energy: 8,
		format: 'aiff',
		durationSeconds: 370,
		year: 2023,
		rating: 4,
		plays: 21,
		color: 'pink',
		cover: 4,
		tags: ['uplifting', 'peak'],
	},
	{
		title: 'Glasshouse',
		artist: 'Ilse Brandt',
		album: 'Transparent',
		genre: 'Minimal',
		label: 'Kleinstadt',
		bpm: 128,
		key: '2A',
		energy: 6,
		format: 'mp3',
		durationSeconds: 492,
		year: 2022,
		rating: 3,
		plays: 6,
		color: null,
		cover: null,
		tags: ['deep', 'groovy'],
	},
	{
		title: 'Undertow',
		artist: 'KOVA',
		album: 'Riptide',
		genre: 'Breaks',
		label: 'Low Orbit',
		bpm: 130,
		key: '9A',
		energy: 9,
		format: 'flac',
		durationSeconds: 351,
		year: 2025,
		rating: 5,
		plays: 17,
		color: 'red',
		cover: 6,
		tags: ['dark', 'peak', 'driving'],
	},
	{
		title: 'Golden Hour Edit',
		artist: 'Sunset Regulars',
		album: 'Poolside Sessions',
		genre: 'Nu Disco',
		label: 'Sun Records Club',
		bpm: 112,
		key: '4B',
		energy: 6,
		format: 'm4a',
		durationSeconds: 338,
		year: 2023,
		rating: 4,
		plays: 11,
		color: 'yellow',
		cover: 7,
		tags: ['uplifting', 'warmup', 'groovy'],
	},
	{
		title: 'Low Gravity',
		artist: 'Tomasz Reyes',
		album: 'Orbital Decay',
		genre: 'Techno',
		label: 'Concrete Bloom',
		bpm: 135,
		key: '1A',
		energy: 10,
		format: 'flac',
		durationSeconds: 448,
		year: 2025,
		rating: 5,
		plays: 25,
		color: 'red',
		cover: 8,
		tags: ['dark', 'peak', 'driving', 'hypnotic'],
	},
	{
		title: 'Slow Burn',
		artist: 'Amber Field',
		album: 'Evenings',
		genre: 'Downtempo',
		label: null,
		bpm: 90,
		key: '6A',
		energy: 2,
		format: 'mp3',
		durationSeconds: 281,
		year: 2021,
		rating: 2,
		plays: 2,
		color: null,
		cover: null,
		tags: ['melancholic', 'closing'],
	},
	{
		title: 'Ghost Frequencies',
		artist: 'Circuit Bloom',
		album: 'Phantom Bands',
		genre: 'Electro',
		label: 'Concrete Bloom',
		bpm: 132,
		key: '12A',
		energy: 8,
		format: 'wav',
		durationSeconds: 396,
		year: 2024,
		rating: 4,
		plays: 8,
		color: 'blue',
		cover: 10,
		tags: ['dark', 'driving'],
	},
	{
		title: 'Tokyo Drift Sessions',
		artist: 'Yuki Aoyama',
		album: 'Night Trains',
		genre: 'House',
		label: 'Shibuya Underground',
		bpm: 122,
		key: '7B',
		energy: 6,
		format: 'flac',
		durationSeconds: 415,
		year: 2025,
		rating: 4,
		plays: 12,
		color: 'green',
		cover: 11,
		tags: ['euphoric', 'groovy'],
	},
	{
		title: 'Paris 1989',
		artist: 'The Velvet Machine',
		album: 'Cold Wave Revival',
		genre: 'Indie Dance',
		label: 'Rive Gauche',
		bpm: 116,
		key: '3B',
		energy: 5,
		format: 'mp3',
		durationSeconds: 318,
		year: 2022,
		rating: 3,
		plays: 4,
		color: null,
		cover: null,
		tags: ['melancholic'],
	},
	{
		title: 'Tidewater',
		artist: 'Saoirse',
		album: 'Coastlines',
		genre: 'Afro House',
		label: 'Tidewater Records',
		bpm: 122,
		key: '9B',
		energy: 7,
		format: 'flac',
		durationSeconds: 433,
		year: 2024,
		rating: 5,
		plays: 19,
		color: 'orange',
		cover: 13,
		tags: ['euphoric', 'peak', 'groovy'],
	},
	{
		title: 'Kaleidoscope',
		artist: 'Otto & Elise',
		album: 'Prism',
		genre: 'Progressive',
		label: 'Night Shift',
		bpm: 138,
		key: '5B',
		energy: 9,
		format: 'aiff',
		durationSeconds: 512,
		year: 2023,
		rating: 4,
		plays: 7,
		color: 'purple',
		cover: 14,
		tags: ['uplifting', 'peak'],
	},
	{
		title: 'Lo-Fi Sermon',
		artist: 'Basement Ritual',
		album: 'Tape Hiss',
		genre: 'Deep Tech',
		label: 'Basement Ritual',
		bpm: 106,
		key: '6B',
		energy: 3,
		format: 'mp3',
		durationSeconds: 367,
		year: 2021,
		rating: 2,
		plays: 1,
		color: null,
		cover: 15,
		tags: ['deep', 'warmup'],
	},
	{
		title: null,
		artist: null,
		album: null,
		genre: '',
		label: null,
		bpm: null,
		key: null,
		energy: null,
		format: 'wav',
		durationSeconds: 243,
		year: null,
		rating: 0,
		plays: 0,
		color: null,
		cover: null,
		tags: [],
	},
]

function buildTrack(seed: TrackSeed, index: number): Track {
	const id = `trk-${String(index + 1).padStart(2, '0')}`
	const fileName = seed.title ? `${seed.artist} - ${seed.title}`.slice(0, 80) : 'untitled_export_final_v3'
	return {
		id,
		file_path: `/Volumes/Harness/Music/${fileName}.${seed.format}`,
		file_hash: `hash-${id}`,
		title: seed.title,
		artist: seed.artist,
		album: seed.album,
		year: seed.year,
		genre: seed.genre || null,
		label: seed.label,
		catalog_number: seed.label ? `CAT${String(100 + index)}` : null,
		duration_ms: seed.durationSeconds * 1000,
		bpm: seed.bpm,
		key: seed.key,
		energy: seed.energy,
		bitrate: BITRATE_BY_FORMAT[seed.format] ?? null,
		sample_rate: seed.format === 'mp3' || seed.format === 'm4a' ? 44100 : 48000,
		format: seed.format,
		analysis_source: seed.bpm === null ? null : 'mixed_in_key',
		waveform_data: null,
		rating: seed.rating,
		play_count: seed.plays,
		date_added: isoAgo(3 + index * 4),
		date_modified: isoAgo(2 + index * 3),
		last_played: seed.plays > 0 ? isoAgo(index % 9, index) : null,
		rekordbox_id: null,
		artwork_path: seed.cover === null ? null : artworkDataUrl(seed.cover),
		artwork_source: seed.cover === null ? null : 'extracted',
		color: seed.color,
		library_root_id: null,
		relative_path: null,
		tags: tagsFor(seed.tags),
	}
}

export const TRACKS: Track[] = SEEDS.map(buildTrack)

// =============================================================================
// Cues and waveform (Player view and player bar)
// =============================================================================

const CUE_COLORS = ['#ef4444', '#f97316', '#22c55e', '#3b82f6']

/** Four hot cues and two memory cues spread over the track. */
export function cuesFor(track: Track): Cue[] {
	const duration = track.duration_ms
	const hot: Cue[] = [0.04, 0.22, 0.48, 0.74].map((ratio, index) => ({
		id: `${track.id}-hot-${index}`,
		track_id: track.id,
		position_ms: Math.round(duration * ratio),
		cue_type: 'hot',
		loop_end_ms: null,
		hot_cue_index: index,
		name: ['Intro', 'Build', 'Drop', 'Outro'][index],
		color: CUE_COLORS[index],
	}))
	const memory: Cue[] = [0.12, 0.6].map((ratio, index) => ({
		id: `${track.id}-mem-${index}`,
		track_id: track.id,
		position_ms: Math.round(duration * ratio),
		cue_type: 'memory',
		loop_end_ms: null,
		hot_cue_index: null,
		name: null,
		color: null,
	}))
	return [...hot, ...memory]
}

/** 1 200 peaks on the 0–100 scale the backend uses, with an intro, a breakdown and a drop, derived only from the track. */
export function waveformFor(track: Track): number[] {
	const phase = Number(track.id.replace(/\D/g, '')) || 1
	return Array.from({ length: 1200 }, (_, i) => {
		const t = i / 1200
		const envelope = t < 0.08 ? t / 0.08 : t > 0.92 ? (1 - t) / 0.08 : 0.55 + 0.45 * Math.sin(t * Math.PI * 4 + phase)
		const grain = 0.5 + 0.5 * Math.sin(i * 0.9 + phase * 1.7)
		return Math.round(Math.max(4, Math.min(100, 100 * envelope * (0.45 + 0.55 * grain))))
	})
}

// =============================================================================
// Playlists
// =============================================================================

interface PlaylistSeed {
	id: string
	name: string
	parentId: string | null
	folder?: boolean
	smart?: boolean
	context?: 'library' | 'discovery'
	trackIds?: string[]
	smartRules?: object
}

const PLAYLIST_SEEDS: PlaylistSeed[] = [
	{ id: 'pl-sets', name: 'Sets', parentId: null, folder: true },
	{
		id: 'pl-warmup',
		name: 'Warm-up 2025',
		parentId: 'pl-sets',
		trackIds: ['trk-02', 'trk-07', 'trk-09', 'trk-12', 'trk-15'],
	},
	{
		id: 'pl-peak',
		name: 'Peak time',
		parentId: 'pl-sets',
		smart: true,
		smartRules: {
			match_mode: 'all',
			conditions: [{ type: 'numeric', field: 'energy', operator: 'greater_than', value: 7 }],
		},
	},
	{ id: 'pl-archive', name: 'Archive', parentId: 'pl-sets', folder: true },
	{ id: 'pl-favorites', name: 'Favorites', parentId: null, trackIds: ['trk-01', 'trk-06', 'trk-08', 'trk-13'] },
	{ id: 'pl-afterhours', name: 'Afterhours', parentId: null, trackIds: ['trk-03', 'trk-04', 'trk-10', 'trk-14'] },
	{
		id: 'pl-long',
		name: 'Crate digging: a very long playlist name to test truncation in the sidebar',
		parentId: null,
		trackIds: ['trk-05', 'trk-11'],
	},
	{ id: 'dpl-shortlist', name: 'To buy', parentId: null, context: 'discovery', trackIds: [] },
	{ id: 'dpl-labels', name: 'Label watch', parentId: null, context: 'discovery', trackIds: [] },
]

/** Smart playlist "Peak time": energy above 7. */
export function smartPlaylistTrackIds(): string[] {
	return TRACKS.filter((track) => (track.energy ?? 0) > 7).map((track) => track.id)
}

/** Playlist members by playlist id. */
export const PLAYLIST_TRACK_IDS: Record<string, string[]> = Object.fromEntries(
	PLAYLIST_SEEDS.map((seed) => [seed.id, seed.smart ? smartPlaylistTrackIds() : (seed.trackIds ?? [])])
)

export const PLAYLISTS: Playlist[] = PLAYLIST_SEEDS.map((seed, index) => ({
	id: seed.id,
	name: seed.name,
	parent_id: seed.parentId,
	is_folder: seed.folder ?? false,
	is_smart: seed.smart ?? false,
	smart_rules: seed.smartRules ? JSON.stringify(seed.smartRules) : null,
	sort_order: index,
	date_created: isoAgo(60 - index),
	date_modified: isoAgo(10 - Math.min(index, 9)),
	track_count: PLAYLIST_TRACK_IDS[seed.id].length,
	context: seed.context ?? 'library',
}))

// =============================================================================
// Devices
// =============================================================================

export const DEVICES: UsbDevice[] = [
	{
		id: 'dev-usb-01',
		name: 'CDJ-STICK',
		mount_point: '/Volumes/CDJ-STICK',
		volume_uuid: '00000000-0000-4000-8000-000000000001',
		total_space_bytes: 32_000_000_000,
		available_space_bytes: 21_500_000_000,
		is_removable: true,
		file_system: 'exFAT',
		disk_kind: 'usb',
	},
]
