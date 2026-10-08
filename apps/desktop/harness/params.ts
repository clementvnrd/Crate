import type { HarnessParams, HarnessUpdate } from './types'

const UPDATE_VALUES: HarnessUpdate[] = ['available', 'download-fails', 'install-fails', 'check-fails']

/** Read the harness parameters of the current page URL. Unknown or missing values fall back to the defaults. */
export function readParams(search: string = window.location.search): HarnessParams {
	const query = new URLSearchParams(search)
	const latency = Number(query.get('latency'))
	return {
		beatportLoggedIn: query.get('beatport') !== 'out',
		libraryEmpty: query.get('library') === 'empty',
		onboarding: query.get('onboarding') === '1',
		dev: query.get('dev') === '1',
		latencyMs: Number.isFinite(latency) && latency > 0 ? Math.min(latency, 10_000) : 0,
		playingTrackId: query.get('playing') || null,
		discrepancyReportFails: query.get('discrepancyReport') === 'fail',
		missingTrackIds: (query.get('missing') ?? '')
			.split(',')
			.map((id) => id.trim())
			.filter((id) => id.length > 0),
		update: UPDATE_VALUES.find((value) => value === query.get('update')) ?? 'none',
	}
}
