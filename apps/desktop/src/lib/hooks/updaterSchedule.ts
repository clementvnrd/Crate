import { derived } from 'svelte/store'
import { updaterStore } from '$lib/stores/updater'
import { isPlaying } from '$shared/stores/player'
import { isAnalyzing } from '$lib/stores/analysis'
import { isExporting } from '$lib/stores/export'
import { isSyncing } from '$lib/stores/sync'
import { isUpgrading } from '$shared/stores/upgrader'
import { isImportingSpotify, isResettingSpotifyHistory, isSyncingRekordbox } from '$shared/stores/stats'
import { isBackupBusy } from '$lib/stores/backup'
import { isApplyingOrganisation } from '$shared/stores/organisation'

/** The first automatic check waits until start-up is over. */
export const FIRST_CHECK_DELAY_MS = 30_000

/** Then Crate looks for a new version every 6 hours (one ~1 KB request). */
export const CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000

/**
 * Crate is busy when relaunching would cut the music or a long job: playback, analysis, USB
 * export or sync, Beatport upgrades, Spotify import or history reset, Rekordbox sync, a backup
 * being created or restored, files being moved by Organisation. While busy, an update is
 * installed for the next launch and never relaunches the app.
 */
export const appBusy = derived(
	[
		isPlaying,
		isAnalyzing,
		isExporting,
		isSyncing,
		isUpgrading,
		isImportingSpotify,
		isResettingSpotifyHistory,
		isSyncingRekordbox,
		isBackupBusy,
		isApplyingOrganisation,
	],
	(flags) => flags.some(Boolean)
)

/** Start the automatic update checks; returns the cleanup function. */
export function startUpdaterSchedule(): () => void {
	const disconnect = updaterStore.connectBusy(appBusy)
	const first = setTimeout(() => updaterStore.check(true).catch(() => {}), FIRST_CHECK_DELAY_MS)
	const interval = setInterval(() => updaterStore.check(true).catch(() => {}), CHECK_INTERVAL_MS)
	return () => {
		disconnect()
		clearTimeout(first)
		clearInterval(interval)
	}
}
