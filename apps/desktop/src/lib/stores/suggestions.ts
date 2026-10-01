import { writable, derived } from 'svelte/store'
import type { NextTrackSuggestion } from '$shared/types'
import * as libraryApi from '$shared/api/library'
import { toErrorMessage } from '$shared/utils/errors'

interface SuggestionsState {
	suggestions: NextTrackSuggestion[]
	loading: boolean
	error: string | null
}

const initialState: SuggestionsState = {
	suggestions: [],
	loading: false,
	error: null,
}

function createSuggestionsStore() {
	const { subscribe, set, update } = writable<SuggestionsState>(initialState)

	// Bumped on every call: a response for a track the user has since moved away from must not
	// overwrite a newer (or empty) request's result.
	let requestId = 0

	return {
		subscribe,

		/**
		 * Loads next-track suggestions for the library track `trackId` (the source: what the owner
		 * actually played after it in Rekordbox, filled out with harmonically and rhythmically
		 * compatible tracks).
		 */
		async load(trackId: string) {
			const id = ++requestId
			update((s) => ({ ...s, loading: true, error: null }))
			try {
				const suggestions = await libraryApi.suggestNextTracks(trackId)
				if (id !== requestId) return
				update((s) => ({ ...s, suggestions, loading: false }))
			} catch (err) {
				if (id !== requestId) return
				const error = toErrorMessage(err, 'Failed to load next-track suggestions')
				update((s) => ({ ...s, suggestions: [], loading: false, error }))
			}
		},

		/** Drops a suggestion that failed to play, e.g. it was removed from the library meanwhile. */
		removeSuggestion(trackId: string) {
			update((s) => ({ ...s, suggestions: s.suggestions.filter((entry) => entry.track.id !== trackId) }))
		},

		/** Resets to the empty state, e.g. when there is no longer an eligible track to suggest from. */
		clear() {
			requestId++
			set(initialState)
		},
	}
}

export const suggestionsStore = createSuggestionsStore()

export const nextTrackSuggestions = derived(suggestionsStore, ($s) => $s.suggestions)
export const suggestionsLoading = derived(suggestionsStore, ($s) => $s.loading)
export const suggestionsError = derived(suggestionsStore, ($s) => $s.error)
