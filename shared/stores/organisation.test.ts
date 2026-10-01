import { describe, it, expect, beforeEach, vi } from 'vitest'
import { get } from 'svelte/store'
import {
	organisationStore,
	organisationPlan,
	isPlanningOrganisation,
	organisationPlanError,
	isApplyingOrganisation,
	organisationBatches,
	undoingOrganisationBatchId,
} from './organisation'
import type { OrganisationPlan, OrganisationResult, OrganisationRule, OrganisationBatch } from '../types'
import * as libraryApi from '../api/library'

vi.mock('../api/library', () => ({
	planOrganisation: vi.fn(),
	applyOrganisation: vi.fn(),
	undoOrganisation: vi.fn(),
	getOrganisationBatches: vi.fn(),
}))

const rule: OrganisationRule = {
	destination_root: '/Volumes/Music/Library',
	template: '{artist}/{album}/{artist} - {title}',
}

const mockPlan: OrganisationPlan = {
	id: 'plan-1',
	rule,
	moves: [
		{
			track_id: 't1',
			title: 'One More Time',
			artist: 'Daft Punk',
			from: '/old/one.mp3',
			to: '/Volumes/Music/Library/Daft Punk/Discovery/Daft Punk - One More Time.mp3',
			status: 'move',
		},
		{
			track_id: 't2',
			title: 'Taken',
			artist: 'Someone',
			from: '/old/taken.mp3',
			to: '/Volumes/Music/Library/Someone/Album/Someone - Taken.mp3',
			status: 'target_exists',
		},
	],
	to_move: 1,
	already_in_place: 0,
	blocked: 1,
	breaks_external_paths: true,
}

describe('organisationStore', () => {
	beforeEach(() => {
		vi.clearAllMocks()
		organisationStore.reset()
	})

	it('initializes with no plan and no batches', () => {
		expect(get(organisationPlan)).toBeNull()
		expect(get(isPlanningOrganisation)).toBe(false)
		expect(get(organisationBatches)).toEqual([])
	})

	it('computes a dry run plan without moving anything', async () => {
		vi.mocked(libraryApi.planOrganisation).mockResolvedValueOnce(mockPlan)

		const result = await organisationStore.planDryRun(rule)

		expect(libraryApi.planOrganisation).toHaveBeenCalledWith(rule)
		expect(result).toEqual(mockPlan)
		expect(get(organisationPlan)).toEqual(mockPlan)
		expect(get(isPlanningOrganisation)).toBe(false)
		expect(libraryApi.applyOrganisation).not.toHaveBeenCalled()
	})

	it('surfaces a refused rule (e.g. an invalid template) as planError, not a thrown exception', async () => {
		vi.mocked(libraryApi.planOrganisation).mockRejectedValueOnce('invalid template: it must contain {title}')

		const result = await organisationStore.planDryRun(rule)

		expect(result).toBeNull()
		expect(get(organisationPlan)).toBeNull()
		expect(get(organisationPlanError)).toBe('invalid template: it must contain {title}')
	})

	it('does nothing when apply is called with no previewed plan', async () => {
		const result = await organisationStore.apply(rule, true)
		expect(result).toBeNull()
		expect(libraryApi.applyOrganisation).not.toHaveBeenCalled()
	})

	it('applies exactly the previewed plan id, then refreshes the preview and the batch list', async () => {
		vi.mocked(libraryApi.planOrganisation).mockResolvedValueOnce(mockPlan)
		await organisationStore.planDryRun(rule)

		const applyResult: OrganisationResult = { batch_id: 'batch-1', moved: 1, failed: [] }
		vi.mocked(libraryApi.applyOrganisation).mockResolvedValueOnce(applyResult)
		vi.mocked(libraryApi.planOrganisation).mockResolvedValueOnce({
			...mockPlan,
			to_move: 0,
			already_in_place: 1,
		})
		vi.mocked(libraryApi.getOrganisationBatches).mockResolvedValueOnce([
			{ batch_id: 'batch-1', moved_at: '2026-10-01T00:00:00Z', files: 1, undone: 0 },
		])

		const result = await organisationStore.apply(rule, true)

		expect(libraryApi.applyOrganisation).toHaveBeenCalledWith(rule, 'plan-1', true)
		expect(result).toEqual(applyResult)
		expect(get(isApplyingOrganisation)).toBe(false)
		expect(get(organisationPlan)?.to_move).toBe(0)
		expect(get(organisationBatches)).toHaveLength(1)
	})

	it('reports a partially failed apply without losing the batch id', async () => {
		vi.mocked(libraryApi.planOrganisation).mockResolvedValueOnce(mockPlan)
		await organisationStore.planDryRun(rule)

		const applyResult: OrganisationResult = {
			batch_id: 'batch-2',
			moved: 0,
			failed: [{ track_id: 't1', reason: 'the file is no longer there' }],
		}
		vi.mocked(libraryApi.applyOrganisation).mockResolvedValueOnce(applyResult)
		vi.mocked(libraryApi.planOrganisation).mockResolvedValueOnce(mockPlan)
		vi.mocked(libraryApi.getOrganisationBatches).mockResolvedValueOnce([])

		const result = await organisationStore.apply(rule, true)
		expect(result?.failed).toHaveLength(1)
	})

	it('undoes a batch and reloads the batch list', async () => {
		const undoResult: OrganisationResult = { batch_id: 'batch-1', moved: 1, failed: [] }
		vi.mocked(libraryApi.undoOrganisation).mockResolvedValueOnce(undoResult)
		const refreshedBatches: OrganisationBatch[] = [
			{ batch_id: 'batch-1', moved_at: '2026-10-01T00:00:00Z', files: 1, undone: 1 },
		]
		vi.mocked(libraryApi.getOrganisationBatches).mockResolvedValueOnce(refreshedBatches)

		const result = await organisationStore.undo('batch-1')

		expect(libraryApi.undoOrganisation).toHaveBeenCalledWith('batch-1')
		expect(result).toEqual(undoResult)
		expect(get(undoingOrganisationBatchId)).toBeNull()
		expect(get(organisationBatches)).toEqual(refreshedBatches)
	})

	it('loads past batches, latest first, as the backend returns them', async () => {
		const batches: OrganisationBatch[] = [
			{ batch_id: 'batch-2', moved_at: '2026-10-01T00:00:00Z', files: 3, undone: 0 },
			{ batch_id: 'batch-1', moved_at: '2026-09-30T00:00:00Z', files: 2, undone: 2 },
		]
		vi.mocked(libraryApi.getOrganisationBatches).mockResolvedValueOnce(batches)

		const result = await organisationStore.loadBatches()
		expect(result).toEqual(batches)
		expect(get(organisationBatches)).toEqual(batches)
	})

	it('clears the plan, e.g. when the rule changes after a preview', async () => {
		vi.mocked(libraryApi.planOrganisation).mockResolvedValueOnce(mockPlan)
		await organisationStore.planDryRun(rule)
		expect(get(organisationPlan)).not.toBeNull()

		organisationStore.clearPlan()
		expect(get(organisationPlan)).toBeNull()
	})
})
