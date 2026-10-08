import { describe, it, expect, beforeEach, vi } from 'vitest'
import { get, writable } from 'svelte/store'
import type { Update } from '@tauri-apps/plugin-updater'
import { SNOOZE_MS, updaterStore, updateAvailable, updateBannerVisible, updateStatus } from './updater'
import * as updaterApi from '$shared/api/updater'
import { appVersion, isDev } from '$lib/stores/app'
import { toastStore } from '$shared/stores/toast'

vi.mock('$shared/api/updater', () => ({
	checkForUpdate: vi.fn(),
	relaunch: vi.fn(),
}))

vi.mock('$lib/stores/app', async () => {
	const { writable } = await import('svelte/store')
	return { appVersion: writable('0.3.0-staging.1'), isDev: writable(false) }
})

vi.mock('$shared/stores/toast', () => ({
	toastStore: { error: vi.fn(), success: vi.fn(), info: vi.fn() },
}))

vi.mock('$shared/i18n', async () => {
	const { readable } = await import('svelte/store')
	return { translate: readable((key: string) => key) }
})

type DownloadEvent = Parameters<NonNullable<Parameters<Update['download']>[0]>>[0]

/** A fake `Update` whose download emits real progress events. */
function fakeUpdate(version = '0.3.0-staging.2', options: { failDownload?: boolean; failInstall?: boolean } = {}) {
	return {
		version,
		body: 'Release notes',
		download: vi.fn(async (onEvent?: (event: DownloadEvent) => void) => {
			onEvent?.({ event: 'Started', data: { contentLength: 1000 } })
			onEvent?.({ event: 'Progress', data: { chunkLength: 400 } })
			if (options.failDownload) throw new Error('network down')
			onEvent?.({ event: 'Progress', data: { chunkLength: 600 } })
			onEvent?.({ event: 'Finished' })
		}),
		install: vi.fn(async () => {
			if (options.failInstall) throw new Error('signature rejected')
		}),
	} as unknown as Update & { download: ReturnType<typeof vi.fn>; install: ReturnType<typeof vi.fn> }
}

const busy = writable(false)
let disconnect: () => void

beforeEach(() => {
	vi.clearAllMocks()
	localStorage.clear()
	disconnect?.()
	busy.set(false)
	updaterStore.reset()
	disconnect = updaterStore.connectBusy(busy)
	;(appVersion as ReturnType<typeof writable<string>>).set('0.3.0-staging.1')
	;(isDev as ReturnType<typeof writable<boolean>>).set(false)
})

describe('updaterStore.check', () => {
	it('offers a newer version and records when it checked', async () => {
		vi.mocked(updaterApi.checkForUpdate).mockResolvedValue(fakeUpdate())
		await updaterStore.check(true)

		const state = get(updaterStore)
		expect(state.status).toBe('available')
		expect(state.version).toBe('0.3.0-staging.2')
		expect(state.body).toBe('Release notes')
		expect(state.lastChecked).toBeGreaterThan(0)
		expect(get(updateAvailable)).toBe(true)
		expect(get(updateBannerVisible)).toBe(true)
	})

	it('is up to date when nothing or the same version is published', async () => {
		vi.mocked(updaterApi.checkForUpdate).mockResolvedValue(null)
		await updaterStore.check(false)
		expect(get(updateStatus)).toBe('upToDate')

		vi.mocked(updaterApi.checkForUpdate).mockResolvedValue(fakeUpdate('0.3.0-staging.1'))
		await updaterStore.check(false)
		expect(get(updateStatus)).toBe('upToDate')
		expect(get(updateBannerVisible)).toBe(false)
	})

	it('never runs an automatic check in a development build', async () => {
		;(isDev as ReturnType<typeof writable<boolean>>).set(true)
		await updaterStore.check(true)
		expect(updaterApi.checkForUpdate).not.toHaveBeenCalled()
	})

	it('does not reset an update that is downloading (hourly check during a download)', async () => {
		let release!: () => void
		const pending = fakeUpdate()
		pending.download.mockImplementation(() => new Promise<void>((resolve) => (release = resolve)))
		vi.mocked(updaterApi.checkForUpdate).mockResolvedValue(pending)
		await updaterStore.check(true)

		const running = updaterStore.updateNow()
		expect(get(updateStatus)).toBe('downloading')
		await updaterStore.check(true)
		await updaterStore.check(false)
		expect(updaterApi.checkForUpdate).toHaveBeenCalledTimes(1)
		expect(get(updateStatus)).toBe('downloading')

		release()
		await running
	})

	it('keeps a failed automatic check silent and out of the banner', async () => {
		vi.mocked(updaterApi.checkForUpdate).mockRejectedValue(new Error('offline'))
		await updaterStore.check(true)

		expect(get(updaterStore)).toMatchObject({ status: 'error', errorPhase: 'check', error: 'offline' })
		expect(toastStore.error).not.toHaveBeenCalled()
		expect(get(updateBannerVisible)).toBe(false)
	})

	it('reports a failed manual check with a toast', async () => {
		vi.mocked(updaterApi.checkForUpdate).mockRejectedValue(new Error('offline'))
		await updaterStore.check(false)
		expect(toastStore.error).toHaveBeenCalledWith('errors.updateCheckFailed')
	})
})

describe('updaterStore.updateNow', () => {
	it('downloads with progress, installs, then relaunches when Crate is idle', async () => {
		const pending = fakeUpdate()
		vi.mocked(updaterApi.checkForUpdate).mockResolvedValue(pending)
		await updaterStore.check(true)

		const seen: number[] = []
		const stop = updaterStore.subscribe(
			(s) => s.status === 'downloading' && s.progress !== null && seen.push(s.progress)
		)
		await updaterStore.updateNow()
		stop()

		expect(seen).toEqual(expect.arrayContaining([0, 40, 100]))
		expect(pending.install).toHaveBeenCalledTimes(1)
		expect(updaterApi.relaunch).toHaveBeenCalledTimes(1)
	})

	it('never relaunches while audio plays: it installs for the next launch instead', async () => {
		const pending = fakeUpdate()
		vi.mocked(updaterApi.checkForUpdate).mockResolvedValue(pending)
		await updaterStore.check(true)
		busy.set(true)

		await updaterStore.updateNow()

		expect(pending.install).toHaveBeenCalledTimes(1)
		expect(updaterApi.relaunch).not.toHaveBeenCalled()
		expect(get(updateStatus)).toBe('scheduled')
		expect(get(updateBannerVisible)).toBe(true)
	})

	it('does not relaunch if playback started during the download', async () => {
		const pending = fakeUpdate()
		pending.download.mockImplementation(async () => busy.set(true))
		vi.mocked(updaterApi.checkForUpdate).mockResolvedValue(pending)
		await updaterStore.check(true)

		await updaterStore.updateNow()

		expect(updaterApi.relaunch).not.toHaveBeenCalled()
		expect(get(updateStatus)).toBe('scheduled')
	})

	it('restartNow relaunches only once an update is scheduled', async () => {
		await updaterStore.restartNow()
		expect(updaterApi.relaunch).not.toHaveBeenCalled()

		vi.mocked(updaterApi.checkForUpdate).mockResolvedValue(fakeUpdate())
		await updaterStore.check(true)
		await updaterStore.installOnQuit()
		await updaterStore.restartNow()
		expect(updaterApi.relaunch).toHaveBeenCalledTimes(1)
	})

	it('shows a download failure in the banner and retries from the download', async () => {
		const failing = fakeUpdate('0.3.0-staging.2', { failDownload: true })
		vi.mocked(updaterApi.checkForUpdate).mockResolvedValue(failing)
		await updaterStore.check(true)
		await updaterStore.updateNow()

		expect(get(updaterStore)).toMatchObject({ status: 'error', errorPhase: 'download', error: 'network down' })
		expect(get(updateBannerVisible)).toBe(true)
		expect(updaterApi.relaunch).not.toHaveBeenCalled()

		failing.download.mockImplementation(async () => {})
		await updaterStore.retry()
		expect(failing.install).toHaveBeenCalledTimes(1)
		expect(updaterApi.relaunch).toHaveBeenCalledTimes(1)
	})

	it('reports a rejected signature as an install failure and keeps the current version', async () => {
		vi.mocked(updaterApi.checkForUpdate).mockResolvedValue(fakeUpdate('0.3.0-staging.2', { failInstall: true }))
		await updaterStore.check(true)
		await updaterStore.updateNow()

		expect(get(updaterStore)).toMatchObject({ status: 'error', errorPhase: 'install', error: 'signature rejected' })
		expect(updaterApi.relaunch).not.toHaveBeenCalled()
	})

	it('shows a failure once, in the banner, even after "Later" (no toast on top)', async () => {
		vi.mocked(updaterApi.checkForUpdate).mockResolvedValue(fakeUpdate('0.3.0-staging.2', { failDownload: true }))
		await updaterStore.check(true)
		updaterStore.later()
		expect(get(updateBannerVisible)).toBe(false)

		// "Update now" from the release notes opened in Settings → About.
		await updaterStore.updateNow()
		expect(get(updateBannerVisible)).toBe(true)
		expect(toastStore.error).not.toHaveBeenCalled()
	})
})

describe('Later and Skip this version', () => {
	it('Later hides the banner and keeps automatic checks quiet for 24 hours', async () => {
		vi.mocked(updaterApi.checkForUpdate).mockResolvedValue(fakeUpdate())
		await updaterStore.check(true)
		updaterStore.later()
		expect(get(updateBannerVisible)).toBe(false)

		updaterStore.reset()
		await updaterStore.check(true)
		expect(get(updateStatus)).toBe('available')
		expect(get(updateBannerVisible)).toBe(false)

		// A manual check always shows what it found.
		updaterStore.reset()
		await updaterStore.check(false)
		expect(get(updateBannerVisible)).toBe(true)

		// After 24 hours, automatic checks show it again.
		const now = Date.now()
		vi.spyOn(Date, 'now').mockReturnValue(now + SNOOZE_MS + 1)
		updaterStore.reset()
		await updaterStore.check(true)
		expect(get(updateBannerVisible)).toBe(true)
		vi.restoreAllMocks()
	})

	it('Skip this version silences only that version', async () => {
		vi.mocked(updaterApi.checkForUpdate).mockResolvedValue(fakeUpdate('0.3.0-staging.2'))
		await updaterStore.check(true)
		updaterStore.skipVersion()

		updaterStore.reset()
		await updaterStore.check(true)
		expect(get(updateBannerVisible)).toBe(false)

		vi.mocked(updaterApi.checkForUpdate).mockResolvedValue(fakeUpdate('0.3.0-staging.3'))
		updaterStore.reset()
		await updaterStore.check(true)
		expect(get(updateBannerVisible)).toBe(true)
	})
})
