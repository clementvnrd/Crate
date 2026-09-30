import type { DiscoveryFilter, DiscoveryRelease } from '$shared/types'
import type { HandlerMap } from '../types'
import type { HarnessState } from '../state'
import { DISCOVERY_RELEASES, FOLLOWED_SOURCES } from '../fixtures/discovery'
import { REFERENCE_NOW } from '../fixtures/reference'

function matchesSearch(release: DiscoveryRelease, search: string): boolean {
	const needle = search.trim().toLowerCase()
	if (!needle) return true
	return [release.artist, release.title, release.label].some((field) => field?.toLowerCase().includes(needle))
}

export function discoveryHandlers(state: HarnessState): HandlerMap {
	const releases = state.params.libraryEmpty ? [] : structuredClone(DISCOVERY_RELEASES)
	return {
		get_discovery_releases: ({ filter }) => {
			const search = (filter as DiscoveryFilter | null)?.search
			return search ? releases.filter((release) => matchesSearch(release, search)) : releases
		},
		get_discovery_release: ({ id }) => {
			const release = releases.find((entry) => entry.id === id)
			if (!release) throw `Release not found: ${String(id)}`
			return release
		},
		toggle_discovery_track_liked: ({ trackId }) => {
			const track = releases.flatMap((release) => release.tracks).find((entry) => entry.id === trackId)
			if (!track) throw `Discovery track not found: ${String(trackId)}`
			track.is_liked = !track.is_liked
			return track.is_liked
		},
		get_followed_sources: () => (state.params.libraryEmpty ? [] : FOLLOWED_SOURCES),
		// "Check now" in the Following window: nothing new.
		check_all_followed_sources: () => ({ totalNew: 0, bySource: [], releaseIds: [], checkedAt: REFERENCE_NOW }),
		get_discovery_audio_cache_size: () => 48_000_000,
	}
}
