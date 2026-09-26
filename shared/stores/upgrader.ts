import { writable, derived, get } from 'svelte/store'
import { listen } from '@tauri-apps/api/event'
import type { UpgradeMatch, UpgradeScanResult, UpgradeCountInfo, UpgradeReplacementResult, UpgradeProgress } from '../types'
import * as upgraderApi from '../api/upgrader'
import { toastStore } from './toast'

export interface UpgraderState {
	matches: UpgradeMatch[]
	totalScanned: number
	totalEligibleMp3s: number
	potentialUpgradesCount: number
	loading: boolean
	upgrading: boolean
	progress: UpgradeProgress | null
	error: string | null
	selectedMatchTrackIds: Set<string>
}

const initialState: UpgraderState = {
	matches: [],
	totalScanned: 0,
	totalEligibleMp3s: 0,
	potentialUpgradesCount: 0,
	loading: false,
	upgrading: false,
	progress: null,
	error: null,
	selectedMatchTrackIds: new Set(),
}

let isFetchingCount = false

function createUpgraderStore() {
	const { subscribe, set, update } = writable<UpgraderState>(initialState)

	return {
		subscribe,

		/**
		 * Scan and load all upgrade matches, pre-selecting all matches by default
		 */
		async load() {
			update((s) => ({ ...s, loading: true, error: null }))
			try {
				const result = await upgraderApi.getUpgradeMatches()
				const preselected = new Set<string>()
				for (const m of result.matches) {
					preselected.add(m.track_id)
				}

				update((s) => ({
					...s,
					matches: result.matches,
					totalScanned: result.total_scanned,
					totalEligibleMp3s: result.total_eligible_mp3s,
					potentialUpgradesCount: result.potential_upgrades_count,
					loading: false,
					selectedMatchTrackIds: preselected,
				}))
				return result
			} catch (error) {
				const errorMsg = error instanceof Error ? error.message : "Erreur lors du scan d'upgrade"
				update((s) => ({ ...s, loading: false, error: errorMsg }))
				toastStore.error(errorMsg)
				return null
			}
		},

		/**
		 * Fetch quick upgrade count without loading full data (deduplicated)
		 */
		async loadCount() {
			if (isFetchingCount) return null
			isFetchingCount = true
			try {
				const countInfo = await upgraderApi.getUpgradeCount()
				update((s) => ({
					...s,
					potentialUpgradesCount: countInfo.match_count,
					totalEligibleMp3s: countInfo.eligible_mp3_count,
				}))
				return countInfo
			} catch (error) {
				console.warn('Failed to get upgrader count:', error)
				return null
			} finally {
				isFetchingCount = false
			}
		},

		/**
		 * Toggle selection of a match for replacement
		 */
		toggleMatchSelection(trackId: string) {
			update((s) => {
				const next = new Set(s.selectedMatchTrackIds)
				if (next.has(trackId)) {
					next.delete(trackId)
				} else {
					next.add(trackId)
				}
				return { ...s, selectedMatchTrackIds: next }
			})
		},

		/**
		 * Select all matches
		 */
		selectAll() {
			update((s) => {
				const next = new Set<string>()
				for (const m of s.matches) {
					next.add(m.track_id)
				}
				return { ...s, selectedMatchTrackIds: next }
			})
		},

		/**
		 * Deselect all matches
		 */
		deselectAll() {
			update((s) => ({ ...s, selectedMatchTrackIds: new Set() }))
		},

		/**
		 * Ignore an upgrade match
		 */
		async ignoreMatch(match: UpgradeMatch) {
			try {
				await upgraderApi.ignoreUpgradeMatch(match.track_id, match.beatport_track.id.toString())
				update((s) => {
					const remaining = s.matches.filter((m) => m.track_id !== match.track_id)
					const nextSelected = new Set(s.selectedMatchTrackIds)
					nextSelected.delete(match.track_id)
					return {
						...s,
						matches: remaining,
						potentialUpgradesCount: remaining.length,
						selectedMatchTrackIds: nextSelected,
					}
				})
				toastStore.success('Morceau ignoré pour les futures améliorations')
			} catch (error) {
				const errorMsg = error instanceof Error ? error.message : "Erreur lors de l'ignorance du morceau"
				toastStore.error(errorMsg)
			}
		},

		/**
		 * Execute upgrade replacement for all selected matches
		 */
		async executeSelected(onUpgraded?: () => Promise<void> | void): Promise<UpgradeReplacementResult | null> {
			const state = get(upgraderStore)
			const selectedMatches = state.matches.filter((m) => state.selectedMatchTrackIds.has(m.track_id))
			if (selectedMatches.length === 0) return null

			update((s) => ({ ...s, upgrading: true, progress: null }))
			const unlistenProgress = await listen<UpgradeProgress>('upgrade-progress', (event) => {
				update((s) => ({ ...s, progress: event.payload }))
			}).catch(() => null)
			try {
				const result = await upgraderApi.executeUpgradeReplacements(selectedMatches)
				if (onUpgraded) {
					await onUpgraded()
				}
				if (result.success_count > 0) {
					toastStore.success(
						result.success_count === 1
							? '1 morceau mis à niveau en FLAC Lossless !'
							: `${result.success_count} morceaux mis à niveau en FLAC Lossless !`
					)
				}
				if (result.failed_count > 0) {
					toastStore.error(`${result.failed_count} mise(s) à niveau ont échoué.`)
				}
				await this.load()
				return result
			} catch (error) {
				const errorMsg = error instanceof Error ? error.message : 'Erreur lors de la mise à niveau'
				toastStore.error(errorMsg)
				return null
			} finally {
				unlistenProgress?.()
				update((s) => ({ ...s, upgrading: false, progress: null }))
			}
		},

		/**
		 * Reset store
		 */
		reset() {
			set(initialState)
		},
	}
}

export const upgraderStore = createUpgraderStore()

export const upgraderMatches = derived(upgraderStore, ($s) => $s.matches)
export const upgraderMatchCount = derived(upgraderStore, ($s) => $s.potentialUpgradesCount)
export const upgraderEligibleCount = derived(upgraderStore, ($s) => $s.totalEligibleMp3s)
export const selectedUpgradeCount = derived(upgraderStore, ($s) => $s.selectedMatchTrackIds.size)
export const isUpgraderLoading = derived(upgraderStore, ($s) => $s.loading)
export const isUpgrading = derived(upgraderStore, ($s) => $s.upgrading)
export const upgradeProgress = derived(upgraderStore, ($s) => $s.progress)
