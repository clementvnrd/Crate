import type { DiscoveryRelease, DiscoverySourceType, DiscoveryTrack, FollowedSource, Tag } from '$shared/types'
import { artworkDataUrl } from './artwork'
import { isoAgo } from './reference'
import { TAG_CATEGORIES } from './library'

// Discovery: releases found on Bandcamp, SoundCloud, YouTube and Discogs, and the artists and labels followed.

interface ReleaseSeed {
	source: DiscoverySourceType
	artist: string | null
	title: string | null
	label: string | null
	releaseDate: string | null
	cover: number | null
	isNew: boolean
	tags: string[]
	tracks: string[]
}

const ALL_TAGS = new Map(TAG_CATEGORIES.flatMap((category) => category.tags).map((tag) => [tag.id, tag]))

const SEEDS: ReleaseSeed[] = [
	{
		source: 'bandcamp',
		artist: 'Nova Kline',
		title: 'Signal Loss EP',
		label: 'Low Orbit',
		releaseDate: '2026-09-12',
		cover: 101,
		isNew: true,
		tags: ['euphoric', 'peak'],
		tracks: ['Afterglow Protocol', 'Dead Air', 'Carrier Wave', 'Static Bloom'],
	},
	{
		source: 'bandcamp',
		artist: 'Hollow Tide',
		title: 'Quiet Harbour',
		label: 'Tidewater Records',
		releaseDate: '2026-08-30',
		cover: 102,
		isNew: true,
		tags: ['melancholic', 'deep'],
		tracks: ['Paper Lanterns', 'Salt Air', 'Low Tide'],
	},
	{
		source: 'soundcloud',
		artist: 'KOVA',
		title: 'Undertow (Bootleg Free Download)',
		label: null,
		releaseDate: '2026-09-02',
		cover: 103,
		isNew: false,
		tags: ['dark'],
		tracks: ['Undertow (Bootleg)'],
	},
	{
		source: 'youtube',
		artist: 'Marlo Vance & The Static Choir',
		title: 'Everything You Ever Wanted To Hear In A Very Long Extended Dub Version (Club Mix Extended Remaster 2024)',
		label: 'Basement Ritual',
		releaseDate: '2024-11-08',
		cover: null,
		isNew: false,
		tags: ['hypnotic'],
		tracks: ['Opening Echo', 'Spring Reverb'],
	},
	{
		source: 'discogs',
		artist: 'The Velvet Machine',
		title: 'Cold Wave Revival',
		label: 'Rive Gauche',
		releaseDate: '1989-03-01',
		cover: 105,
		isNew: false,
		tags: [],
		tracks: ['Paris 1989', 'Night Bus', 'Glass Hour', 'Rooftop'],
	},
	{
		source: 'bandcamp',
		artist: null,
		title: null,
		label: 'Concrete Bloom',
		releaseDate: null,
		cover: null,
		isNew: true,
		tags: [],
		tracks: [],
	},
]

function buildRelease(seed: ReleaseSeed, index: number): DiscoveryRelease {
	const id = `rel-${String(index + 1).padStart(2, '0')}`
	const tags = seed.tags.map((slug) => ALL_TAGS.get(`tag-${slug}`)).filter((tag): tag is Tag => tag !== undefined)
	const tracks: DiscoveryTrack[] = seed.tracks.map((name, position) => ({
		id: `${id}-t${position + 1}`,
		release_id: id,
		name,
		position: position + 1,
		duration_ms: 200_000 + ((position * 41 + index * 17) % 9) * 20_000,
		video_id: seed.source === 'youtube' ? `harness${position}` : null,
		is_liked: position === 0 && index % 2 === 0,
	}))
	return {
		id,
		url: `https://example.invalid/${seed.source}/${id}`,
		source_type: seed.source,
		artist: seed.artist,
		title: seed.title,
		label: seed.label,
		release_date: seed.releaseDate,
		artwork_url: seed.cover === null ? null : artworkDataUrl(seed.cover),
		artwork_path: null,
		notes: index === 0 ? 'Sent by a friend, check the second track in a long set.' : null,
		parent_url: null,
		source_page_url: null,
		date_added: isoAgo(index * 2 + 1),
		date_modified: isoAgo(index * 2),
		is_new: seed.isNew,
		surfaced_at: seed.isNew ? isoAgo(index) : null,
		source_ids: [],
		tracks,
		tags,
	}
}

export const DISCOVERY_RELEASES: DiscoveryRelease[] = SEEDS.map(buildRelease)

export const FOLLOWED_SOURCES: FollowedSource[] = [
	{ name: 'Low Orbit', type: 'label', newCount: 2, health: 'ok' },
	{ name: 'Hollow Tide', type: 'artist', newCount: 0, health: 'ok' },
	{ name: 'Concrete Bloom', type: 'label', newCount: 1, health: 'error' },
].map((seed, index) => ({
	id: `follow-${index + 1}`,
	url: `https://example.invalid/bandcamp/follow-${index + 1}`,
	sourceType: 'bandcamp',
	followType: seed.type as 'artist' | 'label',
	name: seed.name,
	artworkUrl: artworkDataUrl(110 + index),
	artworkPath: null,
	enabled: true,
	dateAdded: isoAgo(40 - index),
	dateModified: isoAgo(3),
	lastCheckedAt: isoAgo(0, 5),
	health: seed.health,
	lastError: seed.health === 'error' ? 'The page could not be reached (timeout).' : null,
	newCount: seed.newCount,
	lastReleaseAt: isoAgo(6 + index),
}))
