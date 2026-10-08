// Shared types of the browser harness (fake Tauri backend).

/** Parameters of an `invoke` call, normalised to a plain object (camelCase keys, as the app sends them). */
export type Args = Record<string, unknown>

/** A fake backend command: receives the arguments, returns (or resolves with) what the real command resolves with. */
export type Handler = (args: Args) => unknown

/** Commands by name. A handler may throw a string to simulate a backend rejection. */
export type HandlerMap = Record<string, Handler>

/** Parameters read from the page URL (`?theme=light&lang=fr&beatport=out…`). See README.md. */
export interface HarnessParams {
	/** `?beatport=out` starts logged out of Beatport. */
	beatportLoggedIn: boolean
	/** `?library=empty` starts with an empty library (empty states). */
	libraryEmpty: boolean
	/** `?onboarding=1` shows the onboarding wizard (settings report it as not completed). */
	onboarding: boolean
	/** `?dev=1` reports a development build (DEV badge, developer tools button). */
	dev: boolean
	/** `?latency=300` delays every command by that many milliseconds (loading states). */
	latencyMs: number
	/** `?playing=trk-03` restores that library track in the player bar, paused. */
	playingTrackId: string | null
	/** `?discrepancyReport=fail` makes `get_discrepancy_report` reject (the report's error state). */
	discrepancyReportFails: boolean
	/**
	 * `?update=available` makes the update check find Crate 0.4.0 (download and install succeed); `download-fails`
	 * and `install-fails` make that step reject, `check-fails` makes the check itself reject (offline). Default
	 * `none`: never an update.
	 */
	update: HarnessUpdate
}

export type HarnessUpdate = 'none' | 'available' | 'download-fails' | 'install-fails' | 'check-fails'

/** One recorded `invoke` call, kept in `window.__harness.calls` (most recent 300). */
export interface CallRecord {
	command: string
	args: Args
	mocked: boolean
}

/** What the harness exposes to tests and to the devtools console as `window.__harness`. */
export interface HarnessApi {
	params: HarnessParams
	/** Names of the commands the app called that have no precise handler (answered by a pattern default). */
	unmocked: Set<string>
	/** Recent calls, oldest first. */
	calls: CallRecord[]
	/** Emit a backend event to the app's `listen()` handlers, e.g. `emit('devices-changed', [])`. */
	emit: (event: string, payload?: unknown) => Promise<void>
}

declare global {
	interface Window {
		__harness?: HarnessApi
	}
}
