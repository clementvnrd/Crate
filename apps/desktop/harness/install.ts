// Entry point of the browser harness. vite.harness.config.ts injects it as a module script in <head>, so it runs
// before the application code: by the time the app first calls `invoke()` or `listen()`, the fake backend is
// installed. Nothing here ships in a production build.

import { emit } from '@tauri-apps/api/event'
import { mockConvertFileSrc, mockIPC, mockWindows } from '@tauri-apps/api/mocks'
import { createBackend } from './backend'
import { TRACKS } from './fixtures/library'
import { readParams } from './params'
import type { HarnessParams } from './types'

/**
 * Seed what the app itself restores from localStorage at start-up (it reads these when its modules evaluate,
 * which happens after this script): `?playing=trk-03` shows that library track in the player bar, paused.
 */
function seedPlayer(params: HarnessParams): void {
	const track = TRACKS.find((entry) => entry.id === params.playingTrackId)
	if (!track || params.libraryEmpty) return
	try {
		window.localStorage.setItem('crate:player.playbackSource', 'library')
		window.localStorage.setItem('crate:player.trackId', track.id)
		window.localStorage.setItem('crate:player.positionMs', String(Math.round(track.duration_ms * 0.35)))
		window.localStorage.setItem('crate:player.durationMs', String(track.duration_ms))
	} catch {
		// Storage unavailable: the player simply starts empty.
	}
}

const params = readParams()
const backend = createBackend(params)

seedPlayer(params)

// `getCurrentWebview()` (drag and drop) needs a window label; `convertFileSrc()` builds artwork URLs.
mockWindows('main')
mockConvertFileSrc('macos')
mockIPC((command, args) => backend.handle(command, args), { shouldMockEvents: true })

window.__harness = {
	params,
	unmocked: backend.unmocked,
	calls: backend.calls,
	emit: (event, payload) => emit(event, payload),
}
