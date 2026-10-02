import { writable, get } from 'svelte/store'
import type { DiscrepancyReport } from '../types'
import { getDiscrepancyReport } from '../api/library'
import { toErrorMessage } from '../utils/errors'
import { translate } from '../i18n'

export interface DiscrepancyReportState {
	report: DiscrepancyReport | null
	loading: boolean
	error: string | null
	/** The Rekordbox export path the last successful load used, if any (kept so Retry repeats it). */
	rekordboxXmlPath: string | null
}

const initialState: DiscrepancyReportState = {
	report: null,
	loading: false,
	error: null,
	rekordboxXmlPath: null,
}

function createDiscrepancyReportStore() {
	const { subscribe, update, set } = writable<DiscrepancyReportState>(initialState)

	return {
		subscribe,

		/**
		 * Builds (or rebuilds) the report. Pass a Rekordbox XML export path to include the Rekordbox
		 * comparison; omit it to keep (or go back to) just Mixed In Key and the missing files. Read only:
		 * nothing is changed or deleted by loading this report.
		 */
		async load(rekordboxXmlPath?: string) {
			update((s) => ({ ...s, loading: true, error: null }))
			try {
				const report = await getDiscrepancyReport(rekordboxXmlPath)
				update((s) => ({ ...s, report, loading: false, rekordboxXmlPath: rekordboxXmlPath ?? null }))
			} catch (err) {
				const message = toErrorMessage(err, get(translate)('discrepancy.loadFailed'))
				update((s) => ({ ...s, loading: false, error: message }))
			}
		},

		/** Reruns the last load (same Rekordbox path, if one was given). */
		async retry() {
			const current = get({ subscribe })
			await this.load(current.rekordboxXmlPath ?? undefined)
		},

		reset() {
			set(initialState)
		},
	}
}

export const discrepancyReportStore = createDiscrepancyReportStore()
