import { writable, derived, get } from 'svelte/store'
import type { DuplicateGroup, DuplicateScanResult, DuplicateCountInfo } from '../types'
import * as duplicateApi from '../api/duplicate'
import * as libraryApi from '../api/library'
import { toastStore } from './toast'
import { toErrorMessage } from '../utils/errors'

export interface DuplicateState {
	groups: DuplicateGroup[]
	totalDuplicateTracks: number
	totalGroups: number
	totalReclaimableBytes: number
	loading: boolean
	error: string | null
	selectedTrackIdsToDelete: Set<string>
}

const initialState: DuplicateState = {
	groups: [],
	totalDuplicateTracks: 0,
	totalGroups: 0,
	totalReclaimableBytes: 0,
	loading: false,
	error: null,
	selectedTrackIdsToDelete: new Set(),
}

function createDuplicateStore() {
	const { subscribe, set, update } = writable<DuplicateState>(initialState)

	return {
		subscribe,

		/**
		 * Scan and load all duplicate groups with default delete pre-selection
		 */
		async load() {
			update((s) => ({ ...s, loading: true, error: null }))
			try {
				const result = await duplicateApi.getDuplicateGroups()
				// Pre-select all tracks that are not recommended to keep
				const preselected = new Set<string>()
				for (const group of result.groups) {
					for (const track of group.tracks) {
						if (!track.recommended_keep) {
							preselected.add(track.id)
						}
					}
				}

				update((s) => ({
					...s,
					groups: result.groups,
					totalDuplicateTracks: result.total_duplicate_tracks,
					totalGroups: result.total_groups,
					totalReclaimableBytes: result.total_reclaimable_bytes,
					loading: false,
					selectedTrackIdsToDelete: preselected,
				}))
				return result
			} catch (error) {
				const errorMsg = toErrorMessage(error, 'Erreur lors du scan des doublons')
				update((s) => ({ ...s, loading: false, error: errorMsg }))
				toastStore.error(errorMsg)
				return null
			}
		},

		/**
		 * Quickly fetch duplicate count summary without loading heavy metadata
		 */
		async loadCount() {
			try {
				const countInfo = await duplicateApi.getDuplicateCount()
				update((s) => ({
					...s,
					totalGroups: countInfo.group_count,
					totalDuplicateTracks: countInfo.track_count,
					totalReclaimableBytes: countInfo.reclaimable_bytes,
				}))
				return countInfo
			} catch (error) {
				console.warn('Failed to get duplicate count:', error)
				return null
			}
		},

		/**
		 * Toggle selection of a track for deletion
		 */
		toggleTrackSelection(trackId: string) {
			update((s) => {
				const next = new Set(s.selectedTrackIdsToDelete)
				if (next.has(trackId)) {
					next.delete(trackId)
				} else {
					next.add(trackId)
				}
				return { ...s, selectedTrackIdsToDelete: next }
			})
		},

		/**
		 * Select all non-recommended duplicates in a group
		 */
		selectGroupDuplicates(group: DuplicateGroup) {
			update((s) => {
				const next = new Set(s.selectedTrackIdsToDelete)
				for (const track of group.tracks) {
					if (!track.recommended_keep) {
						next.add(track.id)
					}
				}
				return { ...s, selectedTrackIdsToDelete: next }
			})
		},

		/**
		 * Deselect all tracks in a group
		 */
		deselectGroup(group: DuplicateGroup) {
			update((s) => {
				const next = new Set(s.selectedTrackIdsToDelete)
				for (const track of group.tracks) {
					next.delete(track.id)
				}
				return { ...s, selectedTrackIdsToDelete: next }
			})
		},

		/**
		 * Select all non-recommended duplicates across all groups
		 */
		selectAllDuplicates() {
			update((s) => {
				const next = new Set<string>()
				for (const group of s.groups) {
					for (const track of group.tracks) {
						if (!track.recommended_keep) {
							next.add(track.id)
						}
					}
				}
				return { ...s, selectedTrackIdsToDelete: next }
			})
		},

		/**
		 * Clear all deletion selections
		 */
		deselectAll() {
			update((s) => ({ ...s, selectedTrackIdsToDelete: new Set() }))
		},

		/**
		 * Ignore a group so its tracks won't be flagged as duplicates
		 */
		async ignoreGroup(group: DuplicateGroup) {
			try {
				const trackIds = group.tracks.map((t) => t.id)
				await duplicateApi.ignoreDuplicateGroup(trackIds)

				update((s) => {
					const remainingGroups = s.groups.filter((g) => g.id !== group.id)
					let reclaimable = 0
					let dupTracks = 0
					for (const g of remainingGroups) {
						reclaimable += g.reclaimable_bytes
						dupTracks += g.tracks.length - 1
					}

					const nextSelected = new Set(s.selectedTrackIdsToDelete)
					for (const id of trackIds) {
						nextSelected.delete(id)
					}

					return {
						...s,
						groups: remainingGroups,
						totalGroups: remainingGroups.length,
						totalDuplicateTracks: dupTracks,
						totalReclaimableBytes: reclaimable,
						selectedTrackIdsToDelete: nextSelected,
					}
				})
				toastStore.success('Groupe de doublons ignoré')
			} catch (error) {
				const errorMsg = toErrorMessage(error, "Erreur lors de l'ignorance du groupe")
				toastStore.error(errorMsg)
			}
		},

		/**
		 * Delete all selected tracks (removes from Crate DB and moves files to macOS Trash)
		 */
		async deleteSelected(onDeleted?: (ids: string[]) => Promise<void> | void) {
			const state = get(duplicateStore)
			const trackIds = Array.from(state.selectedTrackIdsToDelete)
			if (trackIds.length === 0) return

			update((s) => ({ ...s, loading: true }))
			try {
				await libraryApi.deleteTracksAndFiles(trackIds)
				if (onDeleted) {
					await onDeleted(trackIds)
				}
				const count = trackIds.length
				toastStore.success(
					count === 1 ? '1 doublon supprimé (mis à la corbeille)' : `${count} doublons supprimés (mis à la corbeille)`
				)
				await this.load()
			} catch (error) {
				const errorMsg = toErrorMessage(error, 'Erreur lors de la suppression des doublons')
				update((s) => ({ ...s, loading: false }))
				toastStore.error(errorMsg)
			}
		},

		/**
		 * Reset store state
		 */
		reset() {
			set(initialState)
		},
	}
}

export const duplicateStore = createDuplicateStore()

export const duplicateGroups = derived(duplicateStore, ($s) => $s.groups)
export const duplicateGroupCount = derived(duplicateStore, ($s) => $s.totalGroups)
export const duplicateTrackCount = derived(duplicateStore, ($s) => $s.totalDuplicateTracks)
export const duplicateTotalReclaimable = derived(duplicateStore, ($s) => $s.totalReclaimableBytes)
export const isDuplicateLoading = derived(duplicateStore, ($s) => $s.loading)
export const selectedDuplicateCount = derived(duplicateStore, ($s) => $s.selectedTrackIdsToDelete.size)

export const selectedDuplicateReclaimableBytes = derived(duplicateStore, ($s) => {
	let bytes = 0
	for (const group of $s.groups) {
		for (const track of group.tracks) {
			if ($s.selectedTrackIdsToDelete.has(track.id)) {
				bytes += track.file_size_bytes
			}
		}
	}
	return bytes
})
