import type {
	BeatportArtist,
	BeatportArtistDetail,
	BeatportAuthState,
	BeatportChart,
	BeatportGenre,
	BeatportPlaylist,
	BeatportTrack,
	UpgradeMatch,
} from '$shared/types'
import { artworkDataUrl } from './artwork'
import { formatClock, isoAgo } from './reference'
import { TRACKS } from './library'

/** Opaque, non-JWT token: the app's expiry check only parses JWTs, so this one never "expires". */
export const BEATPORT_TOKEN = 'harness-beatport-token'

export const BEATPORT_AUTH_LOGGED_IN: BeatportAuthState = {
	is_authenticated: true,
	username: 'harness.dj',
	token: BEATPORT_TOKEN,
	refresh_token: 'harness-beatport-refresh',
	has_subscription: true,
	subscription_tier: 'Beatport Link',
}

export const BEATPORT_AUTH_LOGGED_OUT: BeatportAuthState = {
	is_authenticated: false,
	username: null,
	token: null,
	refresh_token: null,
	has_subscription: false,
	subscription_tier: null,
}

export const BEATPORT_GENRES: BeatportGenre[] = [
	{ id: 90, name: 'Techno (Peak Time / Driving)', slug: 'techno-peak-time-driving' },
	{ id: 6, name: 'Techno (Raw / Deep / Hypnotic)', slug: 'techno-raw-deep-hypnotic' },
	{ id: 5, name: 'House', slug: 'house' },
	{ id: 12, name: 'Deep House', slug: 'deep-house' },
	{ id: 11, name: 'Tech House', slug: 'tech-house' },
	{ id: 15, name: 'Progressive House', slug: 'progressive-house' },
	{ id: 92, name: 'Melodic House & Techno', slug: 'melodic-house-techno' },
	{ id: 89, name: 'Afro House', slug: 'afro-house' },
	{ id: 37, name: 'Indie Dance', slug: 'indie-dance' },
	{ id: 36, name: 'Organic House', slug: 'organic-house' },
]

const ARTISTS: BeatportArtist[] = [
	'Nova Kline',
	'Hollow Tide',
	'Ilse Brandt',
	'KOVA',
	'Tomasz Reyes',
	'Circuit Bloom',
	'Yuki Aoyama',
	'Saoirse',
	'Otto & Elise',
	'Mara Delacroix',
	'Bruno Castellan',
	'The Quiet Engine',
].map((name, index) => ({
	id: 1000 + index,
	name,
	slug: name.toLowerCase().replace(/[^a-z0-9]+/g, '-'),
	image_url: artworkDataUrl(40 + index),
}))

const TITLES = [
	'Afterglow Protocol',
	'Glass Cathedral',
	'Midnight Arithmetic',
	'Undertow',
	'Low Gravity (Extended Mix)',
	'Ghost Frequencies',
	'Tokyo Drift Sessions',
	'Tidewater',
	'Kaleidoscope (Club Edit)',
	'Everything You Ever Wanted To Hear In A Very Long Extended Dub Version (Club Mix Extended Remaster 2024)',
	'Velvet Static',
	'Paper Lanterns (Remix)',
	'Hollow Sun',
	'Night Ferry',
	'Copper Wire',
	'Slow Collapse',
	'Saltmarsh',
	'Transfer Station',
	'Red Sail',
	'Concrete Bloom',
	'Halogen',
	'Last Yamanote',
	'Porch Light',
	'Terminal Bloom',
]

const KEYS = ['8A', '5A', '11A', '10B', '2A', '9A', '4B', '1A', '6A', '12A', '7B', '3B']
const MIX_NAMES = ['Original Mix', 'Extended Mix', 'Club Mix', 'Radio Edit']

function buildTrack(index: number): BeatportTrack {
	const genre = BEATPORT_GENRES[index % BEATPORT_GENRES.length]
	const durationMs = 330_000 + ((index * 53) % 9) * 20_000
	const artist = ARTISTS[index % ARTISTS.length]
	const second = index % 4 === 3 ? [ARTISTS[(index + 5) % ARTISTS.length]] : []
	const title = TITLES[index % TITLES.length]
	return {
		id: 7_000_000 + index,
		title,
		mix_name: MIX_NAMES[index % MIX_NAMES.length],
		artists: [artist, ...second],
		remixers: index % 5 === 4 ? [ARTISTS[(index + 2) % ARTISTS.length]] : null,
		genre: genre.name,
		genre_id: genre.id,
		release_name: `${title.split(' (')[0]} EP`,
		release_date: isoAgo(index * 3 + 2).slice(0, 10),
		duration_ms: durationMs,
		duration_formatted: formatClock(durationMs),
		key: KEYS[index % KEYS.length],
		bpm: 118 + ((index * 7) % 22),
		artwork_url: artworkDataUrl(50 + index),
		preview_url: null,
		waveform_url: null,
		is_favorite: index % 6 === 0,
		in_cart: false,
		beatport_url: `https://www.beatport.com/track/harness/${7_000_000 + index}`,
	}
}

export const BEATPORT_TRACKS: BeatportTrack[] = Array.from({ length: TITLES.length }, (_, index) => buildTrack(index))

export function beatportTracksForGenre(genreId: number | null): BeatportTrack[] {
	if (genreId === null) return BEATPORT_TRACKS
	const tracks = BEATPORT_TRACKS.filter((track) => track.genre_id === genreId)
	return tracks.length > 0 ? tracks : BEATPORT_TRACKS.slice(0, 8)
}

export const BEATPORT_CHARTS: BeatportChart[] = [
	{ title: 'Peak Time Techno Top 100', subtitle: 'Weekly', genre_name: 'Techno', tracks_count: 100 },
	{ title: 'Melodic House Essentials', subtitle: 'Curated', genre_name: 'Melodic House & Techno', tracks_count: 40 },
	{ title: 'Deep House Selects', subtitle: 'Weekly', genre_name: 'Deep House', tracks_count: 50 },
	{ title: 'Afro House Rising', subtitle: 'New', genre_name: 'Afro House', tracks_count: 30 },
	{
		title: 'Indie Dance Fresh Finds with a Very Long Chart Title That Should Truncate',
		subtitle: 'Curated',
		genre_name: 'Indie Dance',
		tracks_count: 25,
	},
	{ title: 'Progressive Sunrise', subtitle: 'Weekly', genre_name: 'Progressive House', tracks_count: 60 },
].map((chart, index) => ({
	id: 500 + index,
	title: chart.title,
	subtitle: chart.subtitle,
	description: null,
	image_url: artworkDataUrl(70 + index),
	genre_name: chart.genre_name,
	tracks_count: chart.tracks_count,
}))

export const BEATPORT_PLAYLISTS: BeatportPlaylist[] = [
	{ id: 9001, name: 'Warm-up ideas', track_count: 18, image_url: artworkDataUrl(81), is_public: false },
	{ id: 9002, name: 'Sunday afternoon', track_count: 32, image_url: null, is_public: true },
	{ id: 9003, name: 'To check before Friday', track_count: 7, image_url: artworkDataUrl(83), is_public: false },
]

export const BEATPORT_FAVORITES: BeatportTrack[] = BEATPORT_TRACKS.filter((_, index) => index % 4 === 1).map(
	(track) => ({
		...track,
		is_favorite: true,
	})
)

export const BEATPORT_PURCHASES: BeatportTrack[] = BEATPORT_TRACKS.slice(2, 8)

export function beatportArtistDetail(artistId: number): BeatportArtistDetail {
	const artist = ARTISTS.find((entry) => entry.id === artistId) ?? ARTISTS[0]
	return {
		id: artist.id,
		name: artist.name,
		slug: artist.slug,
		image_url: artist.image_url,
		biography: 'Producer and DJ known for long, patient sets that move between deep and driving techno.',
		genres: ['Techno', 'Melodic House & Techno'],
		tracks_count: 48,
	}
}

export function beatportTracksForArtist(artistId: number): BeatportTrack[] {
	const tracks = BEATPORT_TRACKS.filter((track) => track.artists.some((artist) => artist.id === artistId))
	return tracks.length > 0 ? tracks : BEATPORT_TRACKS.slice(0, 4)
}

export function searchBeatport(query: string): { tracks: BeatportTrack[]; artists: BeatportArtist[] } {
	const needle = query.trim().toLowerCase()
	if (!needle) return { tracks: [], artists: [] }
	return {
		tracks: BEATPORT_TRACKS.filter(
			(track) =>
				track.title.toLowerCase().includes(needle) ||
				track.artists.some((artist) => artist.name.toLowerCase().includes(needle))
		),
		artists: ARTISTS.filter((artist) => artist.name.toLowerCase().includes(needle)),
	}
}

// =============================================================================
// Beatport Quality Upgrader: MP3s of the library that Beatport sells as FLAC
// =============================================================================

const UPGRADE_CANDIDATES: { trackId: string; beatportIndex: number; confidence: number }[] = [
	{ trackId: 'trk-02', beatportIndex: 11, confidence: 97 },
	{ trackId: 'trk-05', beatportIndex: 17, confidence: 91 },
	{ trackId: 'trk-12', beatportIndex: 10, confidence: 78 },
]

export const UPGRADE_MATCHES: UpgradeMatch[] = UPGRADE_CANDIDATES.map(({ trackId, beatportIndex, confidence }) => {
	const track = TRACKS.find((entry) => entry.id === trackId)!
	return {
		track_id: track.id,
		file_path: track.file_path,
		current_format: track.format,
		current_bitrate: track.bitrate,
		current_sample_rate: track.sample_rate,
		current_duration_ms: track.duration_ms,
		current_bpm: track.bpm,
		current_key: track.key,
		current_energy: track.energy,
		current_artwork_path: track.artwork_path,
		current_file_size_bytes: Math.round((track.duration_ms / 1000) * 40_000),
		title: track.title ?? '',
		artist: track.artist ?? '',
		album: track.album,
		// The Beatport release of the same track: same title and artist, sold as FLAC.
		beatport_track: {
			...BEATPORT_TRACKS[beatportIndex],
			title: track.title ?? '',
			artists: [{ id: 3000 + beatportIndex, name: track.artist ?? '' }],
			release_name: `${track.title ?? ''} EP`,
			duration_ms: track.duration_ms + 45_000,
			duration_formatted: formatClock(track.duration_ms + 45_000),
			bpm: track.bpm,
			key: track.key,
		},
		confidence_score: confidence,
		// Weights of the backend scoring: title 40, artist 30, duration 15, BPM 10, key 5 (total 100).
		score_breakdown: {
			title_score: Math.round((40 * confidence) / 100),
			artist_score: Math.round((30 * confidence) / 100),
			duration_score: Math.round((15 * confidence) / 100),
			bpm_score: Math.round((10 * confidence) / 100),
			key_score: Math.round((5 * confidence) / 100),
		},
		alternative_candidates: [BEATPORT_TRACKS[(beatportIndex + 3) % BEATPORT_TRACKS.length]],
	}
})
