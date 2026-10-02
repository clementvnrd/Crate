import { describe, it, expect, beforeEach, vi } from 'vitest'
import { get } from 'svelte/store'
import { setModeStore, setModeTrackCount, setModeIsEmpty } from './setMode'
import type { SetAnalysis } from '../types'
import * as libraryApi from '../api/library'
import * as exportApi from '../api/export'
import { save } from '@tauri-apps/plugin-dialog'

vi.mock('../api/library', () => ({
	analyzeSet: vi.fn(),
	suggestSetOrder: vi.fn(),
}))

vi.mock('../api/export', () => ({
	exportSetRekordboxXml: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
	save: vi.fn(),
}))

function entry(trackId: string, position: number, harmonic: SetAnalysis['entries'][number]['from_previous']) {
	return {
		position,
		track_id: trackId,
		title: `Track ${trackId}`,
		artist: 'Artist',
		key: '8A',
		bpm: 124,
		energy: 5,
		duration_ms: 200_000,
		from_previous: harmonic,
	}
}

function analysisFor(trackIds: string[]): SetAnalysis {
	return {
		entries: trackIds.map((id, i) =>
			entry(
				id,
				i + 1,
				i === 0 ? null : { harmonic: 'same', bpm_delta_percent: 0, energy_delta: 0, energy_jump: false, bridges: [] }
			)
		),
		total_duration_ms: trackIds.length * 200_000,
		harmonic_transitions: Math.max(0, trackIds.length - 1),
		clashing_transitions: 0,
		unknown_transitions: 0,
		energy_jumps: 0,
		bpm_min: 124,
		bpm_max: 124,
	}
}

describe('setModeStore', () => {
	beforeEach(() => {
		vi.clearAllMocks()
		setModeStore.reset()
	})

	it('opens with the given selection, in order, and checks it', async () => {
		vi.mocked(libraryApi.analyzeSet).mockResolvedValueOnce(analysisFor(['t1', 't2']))

		setModeStore.open(['t1', 't2'])
		expect(get(setModeStore).trackIds).toEqual(['t1', 't2'])
		expect(get(setModeStore).loading).toBe(true)

		await vi.waitFor(() => expect(get(setModeStore).loading).toBe(false))
		expect(libraryApi.analyzeSet).toHaveBeenCalledWith(['t1', 't2'])
		expect(get(setModeStore).analysis?.entries.map((e) => e.track_id)).toEqual(['t1', 't2'])
		expect(get(setModeTrackCount)).toBe(2)
		expect(get(setModeIsEmpty)).toBe(false)
	})

	it('shows the empty state without calling the backend when the selection is empty', async () => {
		setModeStore.open([])
		await vi.waitFor(() => expect(get(setModeStore).analysis).not.toBeNull())
		expect(libraryApi.analyzeSet).not.toHaveBeenCalled()
		expect(get(setModeStore).analysis?.entries).toEqual([])
		expect(get(setModeIsEmpty)).toBe(true)
	})

	it('moveUp and moveDown reorder the set and re-check it', async () => {
		vi.mocked(libraryApi.analyzeSet).mockResolvedValue(analysisFor(['t1', 't2', 't3']))
		setModeStore.open(['t1', 't2', 't3'])
		await vi.waitFor(() => expect(get(setModeStore).loading).toBe(false))

		vi.mocked(libraryApi.analyzeSet).mockResolvedValueOnce(analysisFor(['t2', 't1', 't3']))
		setModeStore.moveUp(1)
		expect(get(setModeStore).trackIds).toEqual(['t2', 't1', 't3'])
		await vi.waitFor(() => expect(libraryApi.analyzeSet).toHaveBeenLastCalledWith(['t2', 't1', 't3']))

		vi.mocked(libraryApi.analyzeSet).mockResolvedValueOnce(analysisFor(['t2', 't3', 't1']))
		setModeStore.moveDown(1)
		expect(get(setModeStore).trackIds).toEqual(['t2', 't3', 't1'])
		await vi.waitFor(() => expect(libraryApi.analyzeSet).toHaveBeenLastCalledWith(['t2', 't3', 't1']))

		// A boundary move is a no-op.
		setModeStore.moveUp(0)
		expect(get(setModeStore).trackIds).toEqual(['t2', 't3', 't1'])
	})

	it('applySuggestedOrder replaces the order with suggestSetOrder and re-checks it', async () => {
		vi.mocked(libraryApi.analyzeSet).mockResolvedValue(analysisFor(['t1', 't2', 't3']))
		setModeStore.open(['t1', 't2', 't3'])
		await vi.waitFor(() => expect(get(setModeStore).loading).toBe(false))

		vi.mocked(libraryApi.suggestSetOrder).mockResolvedValueOnce(['t3', 't1', 't2'])
		vi.mocked(libraryApi.analyzeSet).mockResolvedValueOnce(analysisFor(['t3', 't1', 't2']))

		await setModeStore.applySuggestedOrder()

		expect(libraryApi.suggestSetOrder).toHaveBeenCalledWith(['t1', 't2', 't3'])
		expect(get(setModeStore).trackIds).toEqual(['t3', 't1', 't2'])
		expect(get(setModeStore).refreshing).toBe(false)
	})

	it('removeTrack drops a track and re-checks the remaining set', async () => {
		vi.mocked(libraryApi.analyzeSet).mockResolvedValue(analysisFor(['t1', 't2']))
		setModeStore.open(['t1', 't2'])
		await vi.waitFor(() => expect(get(setModeStore).loading).toBe(false))

		vi.mocked(libraryApi.analyzeSet).mockResolvedValueOnce(analysisFor(['t2']))
		setModeStore.removeTrack('t1')
		expect(get(setModeStore).trackIds).toEqual(['t2'])
		await vi.waitFor(() => expect(libraryApi.analyzeSet).toHaveBeenLastCalledWith(['t2']))
	})

	it('exportXml does nothing when the native save dialog is cancelled', async () => {
		vi.mocked(libraryApi.analyzeSet).mockResolvedValue(analysisFor(['t1']))
		setModeStore.open(['t1'])
		await vi.waitFor(() => expect(get(setModeStore).loading).toBe(false))

		vi.mocked(save).mockResolvedValueOnce(null)
		const result = await setModeStore.exportXml()

		expect(result).toBe(false)
		expect(exportApi.exportSetRekordboxXml).not.toHaveBeenCalled()
	})

	it('exportXml exports the current order under the set name once a path is chosen', async () => {
		vi.mocked(libraryApi.analyzeSet).mockResolvedValue(analysisFor(['t1', 't2']))
		setModeStore.open(['t1', 't2'])
		await vi.waitFor(() => expect(get(setModeStore).loading).toBe(false))
		setModeStore.setSetName('Friday warehouse')

		vi.mocked(save).mockResolvedValueOnce('/tmp/set.xml')
		vi.mocked(exportApi.exportSetRekordboxXml).mockResolvedValueOnce(2)

		const result = await setModeStore.exportXml()

		expect(result).toBe(true)
		expect(exportApi.exportSetRekordboxXml).toHaveBeenCalledWith('/tmp/set.xml', ['t1', 't2'], 'Friday warehouse')
		expect(get(setModeStore).exporting).toBe(false)
	})

	it('exportXml reports a failure and resets the exporting flag', async () => {
		vi.mocked(libraryApi.analyzeSet).mockResolvedValue(analysisFor(['t1']))
		setModeStore.open(['t1'])
		await vi.waitFor(() => expect(get(setModeStore).loading).toBe(false))

		vi.mocked(save).mockResolvedValueOnce('/tmp/set.xml')
		vi.mocked(exportApi.exportSetRekordboxXml).mockRejectedValueOnce(new Error('disk full'))

		const result = await setModeStore.exportXml()

		expect(result).toBe(false)
		expect(get(setModeStore).exporting).toBe(false)
	})

	it('close resets the store', async () => {
		vi.mocked(libraryApi.analyzeSet).mockResolvedValue(analysisFor(['t1']))
		setModeStore.open(['t1'])
		await vi.waitFor(() => expect(get(setModeStore).loading).toBe(false))

		setModeStore.close()
		expect(get(setModeStore).open).toBe(false)
		expect(get(setModeStore).trackIds).toEqual([])
		expect(get(setModeStore).analysis).toBeNull()
	})
})
