import { writable, derived, get } from 'svelte/store'
import { save } from '@tauri-apps/plugin-dialog'
import type { SetAnalysis } from '../types'
import { analyzeSet, suggestSetOrder } from '../api/library'
import { exportSetRekordboxXml } from '../api/export'
import { toastStore } from './toast'
import { toErrorMessage } from '../utils/errors'
import { withNativeDialog } from '../utils/dom'
import { translate } from '../i18n'

const EMPTY_ANALYSIS: SetAnalysis = {
	entries: [],
	total_duration_ms: 0,
	harmonic_transitions: 0,
	clashing_transitions: 0,
	unknown_transitions: 0,
	energy_jumps: 0,
	bpm_min: null,
	bpm_max: null,
}

export interface SetModeState {
	open: boolean
	/** The set's track ids, in play order. */
	trackIds: string[]
	setName: string
	analysis: SetAnalysis | null
	/** True only while there is no analysis to show yet (first load after opening). */
	loading: boolean
	/** True while a reorder/auto-order/removal is recomputing the check; the previous analysis
	 * stays on screen instead of being replaced by a spinner. */
	refreshing: boolean
	error: string | null
	exporting: boolean
}

const initialState: SetModeState = {
	open: false,
	trackIds: [],
	setName: '',
	analysis: null,
	loading: false,
	refreshing: false,
	error: null,
	exporting: false,
}

function createSetModeStore() {
	const { subscribe, set, update } = writable<SetModeState>(initialState)

	// Invalidates any in-flight analysis when the order changes again, or the modal closes, before
	// that older response can land.
	let requestId = 0

	async function runAnalysis() {
		const state = get(setModeStore)
		const current = ++requestId

		if (state.trackIds.length === 0) {
			update((s) => ({ ...s, analysis: EMPTY_ANALYSIS, loading: false, refreshing: false, error: null }))
			return
		}

		const first = state.analysis === null
		update((s) => ({ ...s, loading: first, refreshing: !first, error: null }))
		try {
			const result = await analyzeSet(state.trackIds)
			if (current !== requestId) return
			update((s) => ({ ...s, analysis: result, loading: false, refreshing: false }))
		} catch (error) {
			if (current !== requestId) return
			const message = toErrorMessage(error, get(translate)('setMode.toast.analyzeFailed'))
			update((s) => ({ ...s, loading: false, refreshing: false, error: message }))
		}
	}

	return {
		subscribe,

		/** Opens Set mode seeded with this selection, in the order it was given, and starts checking it. */
		open(trackIds: string[]) {
			set({
				...initialState,
				open: true,
				trackIds: [...trackIds],
				setName: get(translate)('setMode.defaultName'),
			})
			runAnalysis()
		},

		close() {
			requestId++
			set(initialState)
		},

		setSetName(name: string) {
			update((s) => ({ ...s, setName: name }))
		},

		moveUp(index: number) {
			if (index <= 0) return
			update((s) => {
				const trackIds = [...s.trackIds]
				;[trackIds[index - 1], trackIds[index]] = [trackIds[index], trackIds[index - 1]]
				return { ...s, trackIds }
			})
			runAnalysis()
		},

		moveDown(index: number) {
			update((s) => {
				if (index >= s.trackIds.length - 1) return s
				const trackIds = [...s.trackIds]
				;[trackIds[index], trackIds[index + 1]] = [trackIds[index + 1], trackIds[index]]
				return { ...s, trackIds }
			})
			runAnalysis()
		},

		removeTrack(trackId: string) {
			update((s) => ({ ...s, trackIds: s.trackIds.filter((id) => id !== trackId) }))
			runAnalysis()
		},

		/** Replaces the order with `suggestSetOrder`'s proposal, then re-checks it. */
		async applySuggestedOrder() {
			const state = get(setModeStore)
			if (state.trackIds.length < 2) return
			update((s) => ({ ...s, refreshing: true, error: null }))
			try {
				const order = await suggestSetOrder(state.trackIds)
				update((s) => ({ ...s, trackIds: order }))
				await runAnalysis()
			} catch (error) {
				const message = toErrorMessage(error, get(translate)('setMode.toast.suggestFailed'))
				update((s) => ({ ...s, refreshing: false }))
				toastStore.error(message)
			}
		},

		/**
		 * Opens the native save dialog and exports this set, in its current order, to a single
		 * Rekordbox XML playlist named after it. Resolves to whether the export went through (a
		 * cancelled dialog is not an error).
		 */
		async exportXml(): Promise<boolean> {
			const state = get(setModeStore)
			if (state.trackIds.length === 0) return false

			const defaultName = (state.setName || get(translate)('setMode.defaultName')).trim()
			const path = await withNativeDialog(() =>
				save({
					defaultPath: `${defaultName}.xml`,
					filters: [{ name: 'Pioneer Rekordbox XML', extensions: ['xml'] }],
				})
			)
			if (!path) return false

			update((s) => ({ ...s, exporting: true }))
			try {
				const count = await exportSetRekordboxXml(path, state.trackIds, defaultName)
				toastStore.success(get(translate)('setMode.toast.exportSuccess', { values: { count } }))
				update((s) => ({ ...s, exporting: false }))
				return true
			} catch (error) {
				const message = toErrorMessage(error, get(translate)('common.unknownError'))
				toastStore.error(get(translate)('setMode.toast.exportFailed', { values: { error: message } }))
				update((s) => ({ ...s, exporting: false }))
				return false
			}
		},

		reset() {
			requestId++
			set(initialState)
		},
	}
}

export const setModeStore = createSetModeStore()

export const setModeTrackCount = derived(setModeStore, ($s) => $s.trackIds.length)
export const setModeIsEmpty = derived(setModeStore, ($s) => $s.trackIds.length === 0)
