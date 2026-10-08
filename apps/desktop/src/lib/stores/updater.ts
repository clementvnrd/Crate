import { writable, derived, get, type Readable } from 'svelte/store'
import type { Update } from '@tauri-apps/plugin-updater'
import { checkForUpdate, relaunch } from '$shared/api/updater'
import { appVersion, isDev } from '$lib/stores/app'
import { toastStore } from '$shared/stores/toast'
import { translate } from '$shared/i18n'
import { toErrorMessage } from '$shared/utils/errors'

// =============================================================================
// Types
// =============================================================================

/**
 * - `available`: a newer version exists; nothing is downloaded yet.
 * - `downloading` / `installing`: "Update now" is running (download, signature check, install).
 * - `scheduled`: installed in the background while Crate was busy; it opens at the next launch.
 * - `error`: `errorPhase` says which step failed, so "Retry" can resume from there.
 */
export type UpdaterStatus =
	| 'idle'
	| 'checking'
	| 'upToDate'
	| 'available'
	| 'downloading'
	| 'installing'
	| 'scheduled'
	| 'error'

export type UpdaterErrorPhase = 'check' | 'download' | 'install'

export interface UpdaterState {
	status: UpdaterStatus
	update: Update | null
	version: string | null
	body: string | null
	/** Download progress in percent, or `null` while the size is unknown. */
	progress: number | null
	error: string | null
	errorPhase: UpdaterErrorPhase | null
	/** The banner was hidden ("Later", "Skip this version"); the update itself is kept. */
	dismissed: boolean
	/** Epoch milliseconds of the last completed check. */
	lastChecked: number | null
	/** Audio is playing or a long job runs: never relaunch, install for the next launch instead. */
	busy: boolean
}

// =============================================================================
// Persistence (per-device conveniences: losing them only shows the banner again)
// =============================================================================

const STORAGE_KEYS = {
	snoozedUntil: 'crate.updater.snoozedUntil',
	skippedVersion: 'crate.updater.skippedVersion',
	lastChecked: 'crate.updater.lastChecked',
} as const

export const SNOOZE_MS = 24 * 60 * 60 * 1000

function readStored(key: string): string | null {
	try {
		return localStorage.getItem(key)
	} catch {
		return null
	}
}

function writeStored(key: string, value: string | null): void {
	try {
		if (value === null) localStorage.removeItem(key)
		else localStorage.setItem(key, value)
	} catch {
		// Storage unavailable: the banner simply comes back next time.
	}
}

function storedNumber(key: string): number | null {
	const value = Number(readStored(key))
	return Number.isFinite(value) && value > 0 ? value : null
}

// =============================================================================
// State
// =============================================================================

function initialState(): UpdaterState {
	return {
		status: 'idle',
		update: null,
		version: null,
		body: null,
		progress: 0,
		error: null,
		errorPhase: null,
		dismissed: false,
		lastChecked: storedNumber(STORAGE_KEYS.lastChecked),
		busy: false,
	}
}

/** States in which a new check must not run: it would reset an update in progress. */
const IN_FLIGHT: UpdaterStatus[] = ['checking', 'downloading', 'installing', 'scheduled']

// =============================================================================
// Store
// =============================================================================

function createUpdaterStore() {
	const { subscribe, set, update } = writable<UpdaterState>(initialState())
	const read = () => get({ subscribe })

	/** Versions hidden with the banner's close button, until Crate quits. */
	const hiddenForSession = new Set<string>()
	let checkInFlight = false

	function fail(phase: UpdaterErrorPhase, error: unknown) {
		update((s) => ({ ...s, status: 'error', errorPhase: phase, error: toErrorMessage(error, 'Unknown error') }))
	}

	/** "Later" (24 h), "Skip this version" or the close button still apply to this version. */
	function isHidden(version: string, now: number): boolean {
		return (
			(storedNumber(STORAGE_KEYS.snoozedUntil) ?? 0) > now ||
			readStored(STORAGE_KEYS.skippedVersion) === version ||
			hiddenForSession.has(version)
		)
	}

	/** Free the native resource behind an offer that is replaced or withdrawn. */
	function release(offer: Update | null) {
		if (!offer) return
		void Promise.resolve()
			.then(() => offer.close?.())
			.catch(() => {})
	}

	/** Relaunch into the installed version; if that fails, keep "Restart now" on offer. */
	async function relaunchOrSchedule() {
		try {
			await relaunch()
		} catch (error) {
			console.warn('Relaunch after update failed:', error)
			update((s) => ({ ...s, status: 'scheduled', dismissed: false }))
		}
	}

	/**
	 * Download the update with progress. Returns false (state set to `error`) on failure. The
	 * banner is shown again even if it was hidden: the user asked for this update, and the banner
	 * is where its progress and any failure (with Retry) appear — no toast on top of it.
	 */
	async function download(pending: Update): Promise<boolean> {
		update((s) => ({ ...s, status: 'downloading', progress: 0, error: null, errorPhase: null, dismissed: false }))
		let contentLength = 0
		let downloaded = 0
		try {
			await pending.download((event) => {
				switch (event.event) {
					case 'Started':
						contentLength = event.data.contentLength ?? 0
						if (contentLength === 0) update((s) => ({ ...s, progress: null }))
						break
					case 'Progress':
						downloaded += event.data.chunkLength
						if (contentLength > 0) {
							const progress = Math.min(100, Math.round((downloaded / contentLength) * 100))
							update((s) => (s.progress === progress ? s : { ...s, progress }))
						}
						break
					case 'Finished':
						update((s) => ({ ...s, progress: 100 }))
						break
				}
			})
			return true
		} catch (error) {
			fail('download', error)
			return false
		}
	}

	/** Verify and install the downloaded bundle (macOS swaps the app on disk; the running copy keeps working). */
	async function install(pending: Update): Promise<boolean> {
		update((s) => ({ ...s, status: 'installing' }))
		try {
			await pending.install()
			return true
		} catch (error) {
			fail('install', error)
			return false
		}
	}

	const store = {
		subscribe,

		/**
		 * Look for a newer version. A silent (automatic) check never runs in development builds,
		 * never interrupts an update in progress or a failed one waiting for "Retry", respects
		 * "Later", "Skip this version" and the close button, and never shows a toast: being offline
		 * at a gig is not an error worth reporting. It keeps running while an update is offered, so
		 * the banner comes back when "Later" expires and a newer version replaces a skipped one.
		 * A failed check never drops an update already offered.
		 */
		async check(silent = false) {
			if (silent && get(isDev)) return
			const previous = read()
			if (checkInFlight || IN_FLIGHT.includes(previous.status)) return
			if (silent && previous.status === 'error' && previous.errorPhase !== 'check') return

			const holdsOffer = previous.update !== null && (previous.status === 'available' || previous.status === 'error')
			checkInFlight = true
			// While an update is offered the banner stays as it is during the check.
			if (!holdsOffer) update((s) => ({ ...s, status: 'checking', error: null, errorPhase: null }))

			try {
				const result = await checkForUpdate()
				const now = Date.now()
				writeStored(STORAGE_KEYS.lastChecked, String(now))

				if (!result || result.version === get(appVersion)) {
					release(previous.update)
					release(result)
					update((s) => ({ ...s, status: 'upToDate', update: null, error: null, errorPhase: null, lastChecked: now }))
					return
				}

				// The same version again: keep the offer already held (and its download state).
				const offer = previous.update?.version === result.version ? previous.update : result
				release(offer === result ? previous.update : result)
				update((s) => ({
					...s,
					status: 'available',
					update: offer,
					version: offer.version,
					body: offer.body ?? null,
					progress: 0,
					error: null,
					errorPhase: null,
					lastChecked: now,
					dismissed: silent ? isHidden(offer.version, now) : false,
				}))
			} catch (error) {
				if (!holdsOffer) fail('check', error)
				if (!silent) toastStore.error(get(translate)('errors.updateCheckFailed'))
			} finally {
				checkInFlight = false
			}
		},

		/**
		 * "Update now": download, verify, install, then relaunch — unless Crate is busy (audio
		 * playing, analysis, export, upgrade, sync), in which case it installs for the next launch.
		 */
		async updateNow() {
			const current = read()
			if (!current.update || IN_FLIGHT.includes(current.status)) return
			if (current.busy) {
				await store.installOnQuit()
				return
			}
			const pending = current.update
			if (!(await download(pending))) return
			if (!(await install(pending))) return
			if (read().busy) {
				// Playback started during the download: do not cut it.
				update((s) => ({ ...s, status: 'scheduled' }))
				return
			}
			await relaunchOrSchedule()
		},

		/** "Install when I quit": download and install now, never relaunch; the new version opens next time. */
		async installOnQuit() {
			const current = read()
			if (!current.update || IN_FLIGHT.includes(current.status)) return
			const pending = current.update
			if (!(await download(pending))) return
			if (!(await install(pending))) return
			update((s) => ({ ...s, status: 'scheduled', dismissed: false }))
		},

		/** Relaunch into an update installed for the next launch. */
		async restartNow() {
			if (read().status !== 'scheduled') return
			await relaunchOrSchedule()
		},

		/** Resume from the step that failed. */
		async retry() {
			const current = read()
			if (current.status !== 'error') return
			if (current.errorPhase !== 'check' && current.update) {
				update((s) => ({ ...s, status: 'available', error: null, errorPhase: null }))
				await store.updateNow()
			} else {
				await store.check(false)
			}
		},

		/** "Later": hide the banner and stay quiet for 24 hours. */
		later() {
			writeStored(STORAGE_KEYS.snoozedUntil, String(Date.now() + SNOOZE_MS))
			update((s) => ({ ...s, dismissed: true }))
		},

		/** "Skip this version": never offer this version again (a newer one is offered normally). */
		skipVersion() {
			const version = read().version
			if (version) writeStored(STORAGE_KEYS.skippedVersion, version)
			update((s) => ({ ...s, dismissed: true }))
		},

		/** Hide the banner for this session only (automatic checks keep it hidden until Crate quits). */
		dismiss() {
			const version = read().version
			if (version) hiddenForSession.add(version)
			update((s) => ({ ...s, dismissed: true }))
		},

		/**
		 * Follow whether Crate is busy. The layout passes a store combining playback and the long
		 * jobs; returns the unsubscribe function.
		 */
		connectBusy(source: Readable<boolean>) {
			return source.subscribe((busy) => update((s) => (s.busy === busy ? s : { ...s, busy })))
		},

		/** Back to the initial state; whether Crate is busy is kept (its source does not emit again). */
		reset() {
			hiddenForSession.clear()
			checkInFlight = false
			set({ ...initialState(), busy: read().busy })
		},
	}

	return store
}

export const updaterStore = createUpdaterStore()

// =============================================================================
// Derived Stores
// =============================================================================

export const updateStatus = derived(updaterStore, ($s) => $s.status)

/** A newer version is known and the banner was not hidden. */
export const updateAvailable = derived(updaterStore, ($s) => $s.status === 'available' && !$s.dismissed)

/**
 * The update banner is shown while an update is offered, running, scheduled, or failed during
 * download/install. A failed *check* stays out of the banner (Settings → About shows it).
 */
export const updateBannerVisible = derived(updaterStore, ($s) => {
	if ($s.dismissed) return false
	if ($s.status === 'error') return $s.errorPhase !== 'check' && $s.update !== null
	return ['available', 'downloading', 'installing', 'scheduled'].includes($s.status)
})
