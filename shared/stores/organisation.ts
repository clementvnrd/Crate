import { writable, derived, get } from 'svelte/store'
import type { OrganisationBatch, OrganisationPlan, OrganisationResult, OrganisationRule } from '../types'
import * as libraryApi from '../api/library'
import { toastStore } from './toast'
import { toErrorMessage } from '../utils/errors'
import { translate } from '../i18n'

// Assisted physical organisation (CRA-130): a naming/folder rule applied to the real files on disk.
// The hard rule behind this store (register defect C6, never move a file without a preview): the only
// way to reach `apply()` is to already hold the `OrganisationPlan` that `planDryRun()` returned, and
// `apply()` sends that plan's `id` straight back to the backend, which refuses to run a different plan.

export interface OrganisationState {
	plan: OrganisationPlan | null
	planning: boolean
	planError: string | null
	applying: boolean
	applyResult: OrganisationResult | null
	batches: OrganisationBatch[]
	loadingBatches: boolean
	undoingBatchId: string | null
}

const initialState: OrganisationState = {
	plan: null,
	planning: false,
	planError: null,
	applying: false,
	applyResult: null,
	batches: [],
	loadingBatches: false,
	undoingBatchId: null,
}

function createOrganisationStore() {
	const { subscribe, set, update } = writable<OrganisationState>(initialState)

	return {
		subscribe,

		/** Computes the dry run: every planned move, touching nothing on disk or in the database. */
		async planDryRun(rule: OrganisationRule): Promise<OrganisationPlan | null> {
			update((s) => ({ ...s, planning: true, planError: null, applyResult: null }))
			try {
				const plan = await libraryApi.planOrganisation(rule)
				update((s) => ({ ...s, plan, planning: false }))
				return plan
			} catch (error) {
				const message = toErrorMessage(error, get(translate)('organisation.toast.planFailed'))
				update((s) => ({ ...s, plan: null, planning: false, planError: message }))
				return null
			}
		},

		/**
		 * Runs the previewed plan. Refuses (at the backend) if the library or the destination changed
		 * since the preview, or if `understandsExternalTools` is not explicitly true.
		 */
		async apply(rule: OrganisationRule, understandsExternalTools: boolean): Promise<OrganisationResult | null> {
			const state = get(organisationStore)
			if (!state.plan) return null
			update((s) => ({ ...s, applying: true }))
			try {
				const result = await libraryApi.applyOrganisation(rule, state.plan.id, understandsExternalTools)
				update((s) => ({ ...s, applying: false, applyResult: result }))
				if (result.moved > 0) {
					toastStore.success(get(translate)('organisation.toast.applied', { values: { count: result.moved } }))
				}
				if (result.failed.length > 0) {
					toastStore.error(
						get(translate)('organisation.toast.applyPartialFailure', { values: { count: result.failed.length } })
					)
				}
				// The library changed: a fresh preview reflects what is left to move.
				await this.planDryRun(rule)
				await this.loadBatches()
				return result
			} catch (error) {
				const message = toErrorMessage(error, get(translate)('organisation.toast.applyFailed'))
				toastStore.error(message)
				update((s) => ({ ...s, applying: false }))
				return null
			}
		},

		/** Puts a batch's files back where they were. */
		async undo(batchId: string): Promise<OrganisationResult | null> {
			update((s) => ({ ...s, undoingBatchId: batchId }))
			try {
				const result = await libraryApi.undoOrganisation(batchId)
				if (result.moved > 0) {
					toastStore.success(get(translate)('organisation.toast.undone', { values: { count: result.moved } }))
				}
				if (result.failed.length > 0) {
					toastStore.error(
						get(translate)('organisation.toast.undoPartialFailure', { values: { count: result.failed.length } })
					)
				}
				await this.loadBatches()
				update((s) => ({ ...s, undoingBatchId: null }))
				return result
			} catch (error) {
				const message = toErrorMessage(error, get(translate)('organisation.toast.undoFailed'))
				toastStore.error(message)
				update((s) => ({ ...s, undoingBatchId: null }))
				return null
			}
		},

		/** The batches that were applied, latest first. */
		async loadBatches(limit?: number): Promise<OrganisationBatch[]> {
			update((s) => ({ ...s, loadingBatches: true }))
			try {
				const batches = await libraryApi.getOrganisationBatches(limit)
				update((s) => ({ ...s, batches, loadingBatches: false }))
				return batches
			} catch (error) {
				console.warn('Failed to load organisation batches:', error)
				update((s) => ({ ...s, loadingBatches: false }))
				return []
			}
		},

		/** Clears the previewed plan, e.g. when the rule changes and the preview no longer applies. */
		clearPlan() {
			update((s) => ({ ...s, plan: null, planError: null, applyResult: null }))
		},

		reset() {
			set(initialState)
		},
	}
}

export const organisationStore = createOrganisationStore()

export const organisationPlan = derived(organisationStore, ($s) => $s.plan)
export const isPlanningOrganisation = derived(organisationStore, ($s) => $s.planning)
export const organisationPlanError = derived(organisationStore, ($s) => $s.planError)
export const isApplyingOrganisation = derived(organisationStore, ($s) => $s.applying)
export const organisationApplyResult = derived(organisationStore, ($s) => $s.applyResult)
export const organisationBatches = derived(organisationStore, ($s) => $s.batches)
export const isLoadingOrganisationBatches = derived(organisationStore, ($s) => $s.loadingBatches)
export const undoingOrganisationBatchId = derived(organisationStore, ($s) => $s.undoingBatchId)
