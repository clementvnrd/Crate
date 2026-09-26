import { writable, derived } from 'svelte/store'
import type { StandaloneTrack } from '$shared/types'
import * as standaloneApi from '$shared/api/standalone'
import { toErrorMessage } from '$shared/utils/errors'

interface RecentTracksState {
	tracks: StandaloneTrack[]
	loading: boolean
	error: string | null
}

const initialState: RecentTracksState = {
	tracks: [],
	loading: false,
	error: null,
}

function createRecentTracksStore() {
	const { subscribe, set, update } = writable<RecentTracksState>(initialState)

	return {
		subscribe,

		/**
		 * Load recent standalone tracks from SQLite
		 */
		async load(limit?: number) {
			update((s) => ({ ...s, loading: true, error: null }))
			try {
				const tracks = await standaloneApi.getRecentStandaloneTracks(limit)
				update((s) => ({ ...s, tracks, loading: false }))
			} catch (err) {
				const error = toErrorMessage(err, 'Failed to load recent tracks')
				update((s) => ({ ...s, error, loading: false }))
			}
		},

		/**
		 * Add a track to recent history (persisted in DB and state)
		 */
		async addTrack(track: StandaloneTrack) {
			// STRICT GOLDEN RULE: If already in library, do not add to recent standalone tracks!
			if (track.is_in_library) {
				return
			}

			try {
				await standaloneApi.addRecentStandaloneTrack(track)
				update((s) => {
					const existingIndex = s.tracks.findIndex((t) => t.file_path === track.file_path)
					let updated: StandaloneTrack[]
					if (existingIndex >= 0) {
						updated = [
							{ ...s.tracks[existingIndex], ...track, last_played_at: new Date().toISOString() },
							...s.tracks.filter((_, i) => i !== existingIndex),
						]
					} else {
						updated = [{ ...track, last_played_at: new Date().toISOString() }, ...s.tracks]
					}
					return { ...s, tracks: updated }
				})
			} catch (err) {
				console.error('Failed to save recent track:', err)
			}
		},

		/**
		 * Remove a single track from recent history
		 */
		async removeTrack(id: string) {
			try {
				await standaloneApi.removeRecentStandaloneTrack(id)
				update((s) => ({
					...s,
					tracks: s.tracks.filter((t) => t.id !== id),
				}))
			} catch (err) {
				console.error('Failed to remove recent track:', err)
			}
		},

		/**
		 * Clear all recent standalone tracks history
		 */
		async clear() {
			try {
				await standaloneApi.clearRecentStandaloneTracks()
				update((s) => ({ ...s, tracks: [] }))
			} catch (err) {
				console.error('Failed to clear recent tracks:', err)
			}
		},
	}
}

export const recentTracksStore = createRecentTracksStore()

export const recentStandaloneTracks = derived(recentTracksStore, ($state) => $state.tracks)
export const recentTracksLoading = derived(recentTracksStore, ($state) => $state.loading)
