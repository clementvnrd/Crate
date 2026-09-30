import type { PlayerAlbum, PlayerAlbumTrack, StandaloneTrack } from '$shared/types'
import { artworkDataUrl } from './artwork'
import { isoAgo } from './reference'

// =============================================================================
// Albums of the Player view's grid
// =============================================================================

interface AlbumSeed {
	title: string
	artist: string
	year: number | null
	genre: string | null
	trackCount: number
	cover: number | null
	trackTitles: string[]
}

const ALBUM_SEEDS: AlbumSeed[] = [
	{
		title: 'Quiet Harbour',
		artist: 'Hollow Tide',
		year: 2024,
		genre: 'Deep House',
		trackCount: 9,
		cover: 21,
		trackTitles: [
			'Paper Lanterns',
			'Salt Air',
			'Low Tide',
			'Harbour Lights',
			'Rope & Anchor',
			'Fog Horn',
			'Driftwood',
			'Night Ferry',
			'Morning Nets',
		],
	},
	{
		title: 'Signal Loss EP',
		artist: 'Nova Kline',
		year: 2025,
		genre: 'Melodic Techno',
		trackCount: 4,
		cover: 22,
		trackTitles: ['Afterglow Protocol', 'Dead Air', 'Carrier Wave', 'Static Bloom'],
	},
	{
		title: 'The Complete Dub Sessions (Deluxe Anniversary Edition)',
		artist: 'Marlo Vance & The Static Choir',
		year: 2024,
		genre: 'Dub Techno',
		trackCount: 12,
		cover: 23,
		trackTitles: [
			'Opening Echo',
			'Spring Reverb',
			'Long Delay',
			'Tape Loop',
			'Chamber Dub',
			'Sub Bass Sermon',
			'Return Trip',
			'Wide Room',
			'Soft Clip',
			'Dub Organ',
			'Fade Out',
			'Closing Echo',
		],
	},
	{
		title: 'Coastlines',
		artist: 'Saoirse',
		year: 2024,
		genre: 'Afro House',
		trackCount: 8,
		cover: 24,
		trackTitles: ['Tidewater', 'Sand Drums', 'Kelp', 'Shoreline', 'Saltmarsh', 'Lighthouse', 'Red Sail', 'Last Wave'],
	},
	{
		title: 'Poolside Sessions',
		artist: 'Sunset Regulars',
		year: 2023,
		genre: 'Nu Disco',
		trackCount: 6,
		cover: null,
		trackTitles: ['Golden Hour Edit', 'Sunbed', 'Chlorine', 'Lemon Soda', 'Cabana', 'Dusk Dip'],
	},
	{
		title: 'Night Trains',
		artist: 'Yuki Aoyama',
		year: 2025,
		genre: 'House',
		trackCount: 10,
		cover: 26,
		trackTitles: [
			'Tokyo Drift Sessions',
			'Last Yamanote',
			'Platform 9',
			'Neon Window',
			'Overnight',
			'Transfer',
			'Ticket Gate',
			'Rain Delay',
			'Terminus',
			'First Train',
		],
	},
	{
		title: 'Evenings',
		artist: 'Amber Field',
		year: null,
		genre: null,
		trackCount: 7,
		cover: null,
		trackTitles: ['Slow Burn', 'Porch Light', 'Kettle', 'Window Seat', 'Half Light', 'Blanket', 'Late Bus'],
	},
]

export const ALBUMS: PlayerAlbum[] = ALBUM_SEEDS.map((seed, index) => ({
	id: `alb-${String(index + 1).padStart(2, '0')}`,
	folder_path: `/Volumes/Harness/Albums/${seed.artist} - ${seed.title}`.slice(0, 120),
	title: seed.title,
	artist: seed.artist,
	year: seed.year,
	genre: seed.genre,
	artwork_path: seed.cover === null ? null : artworkDataUrl(seed.cover),
	track_count: seed.trackCount,
	total_duration_ms: seed.trackCount * 312_000,
	created_at: isoAgo(5 + index * 6),
}))

const KEYS = ['8A', '5A', '11A', '10B', '2A', '9A', '4B', '1A', '6A', '12A']

export function albumTracksFor(albumId: string): PlayerAlbumTrack[] {
	const index = ALBUMS.findIndex((album) => album.id === albumId)
	if (index < 0) return []
	const album = ALBUMS[index]
	const seed = ALBUM_SEEDS[index]
	return seed.trackTitles.map((title, position) => ({
		id: `${albumId}-t${String(position + 1).padStart(2, '0')}`,
		album_id: albumId,
		file_path: `${album.folder_path}/${String(position + 1).padStart(2, '0')} ${title}.flac`,
		track_number: position + 1,
		title,
		artist: album.artist,
		duration_ms: 240_000 + ((position * 37 + index * 11) % 11) * 15_000,
		format: 'flac',
		bitrate: 987,
		sample_rate: 48000,
		bpm: 108 + ((position * 5 + index * 3) % 28),
		key: KEYS[(position + index) % KEYS.length],
		energy: 3 + ((position + index) % 7),
		artwork_path: album.artwork_path ?? null,
	}))
}

// =============================================================================
// Recently opened standalone files ("Open With")
// =============================================================================

const RECENT_SEEDS: { title: string; artist: string; format: string; cover: number | null; inLibrary: boolean }[] = [
	{ title: 'Afterglow Protocol (Club Mix)', artist: 'Nova Kline', format: 'flac', cover: 31, inLibrary: true },
	{ title: 'Demo Master v7', artist: 'Anonymous Promo', format: 'wav', cover: null, inLibrary: false },
	{ title: 'Midnight Edit (Bootleg)', artist: 'Sunset Regulars', format: 'mp3', cover: 33, inLibrary: false },
	{ title: 'Untitled Bounce 2025-09-12', artist: 'Studio Sketches', format: 'aiff', cover: null, inLibrary: false },
]

export const RECENT_STANDALONE_TRACKS: StandaloneTrack[] = RECENT_SEEDS.map((seed, index) => ({
	id: `sa-${index + 1}`,
	file_path: `/Volumes/Harness/Inbox/${seed.title}.${seed.format}`,
	title: seed.title,
	artist: seed.artist,
	album: null,
	duration_ms: 300_000 + index * 21_000,
	format: seed.format,
	bitrate: seed.format === 'mp3' ? 320 : 1411,
	sample_rate: 44100,
	bpm: 120 + index * 3,
	key: KEYS[index],
	energy: 5 + index,
	artwork_path: seed.cover === null ? null : artworkDataUrl(seed.cover),
	is_in_library: seed.inLibrary,
	last_played_at: isoAgo(index + 1),
}))
