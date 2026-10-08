import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { controlOwnsActivationKey, useKeyboardShortcuts, type KeyboardShortcutHandlers } from './useKeyboardShortcuts'
import { uiStore, playerStore, recentTracksStore } from '$lib/stores'
import type { StandaloneTrack } from '$shared/types'
import * as standaloneApi from '$shared/api/standalone'

vi.mock('$shared/api/audio', () => ({
	play: vi.fn(),
	pause: vi.fn(),
	resume: vi.fn(),
	stop: vi.fn(),
	seek: vi.fn(),
	setVolume: vi.fn(),
	setSpeed: vi.fn(),
	getCurrentTime: vi.fn(),
	getDuration: vi.fn(),
	isPlaying: vi.fn(),
}))

vi.mock('$shared/api/standalone', () => ({
	readStandaloneTrack: vi.fn(),
	getRecentStandaloneTracks: vi.fn().mockResolvedValue([]),
	addRecentStandaloneTrack: vi.fn().mockResolvedValue(undefined),
	removeRecentStandaloneTrack: vi.fn(),
	clearRecentStandaloneTracks: vi.fn(),
	takeStartupFiles: vi.fn().mockResolvedValue([]),
	playStandaloneTrack: vi.fn().mockResolvedValue({
		is_playing: true,
		position_ms: 0,
		duration_ms: 200000,
		speed: 1.0,
		volume: 1.0,
		current_track_id: 'st_1',
		current_track_path: '/music/song.mp3',
	}),
}))

describe('useKeyboardShortcuts - Player View Spacebar', () => {
	let cleanup: () => void
	let handlers: KeyboardShortcutHandlers

	beforeEach(async () => {
		vi.clearAllMocks()
		playerStore.reset()
		uiStore.reset()
		await recentTracksStore.clear()

		handlers = {
			onPlayPause: vi.fn(),
			onFocusSearch: vi.fn(),
			onClearSelection: vi.fn(),
			onSelectAll: vi.fn(),
			onOpenSettings: vi.fn(),
			onNewPlaylist: vi.fn(),
			onNewFolder: vi.fn(),
			onImport: vi.fn(),
			onDeleteSelected: vi.fn().mockReturnValue(false),
			onPlaySelected: vi.fn(),
			onSeekBackward: vi.fn(),
			onSeekForward: vi.fn(),
			onFineSeekBackward: vi.fn(),
			onFineSeekForward: vi.fn(),
			onPreviousTrack: vi.fn(),
			onNextTrack: vi.fn(),
			onVolumeUp: vi.fn(),
			onVolumeDown: vi.fn(),
			onToggleMute: vi.fn(),
			onSelectPreviousTrack: vi.fn(),
			onSelectNextTrack: vi.fn(),
			onQuickExport: vi.fn(),
			onJumpToPlayingTrack: vi.fn(),
			onToggleView: vi.fn(),
			onAddRelease: vi.fn(),
			onRefreshMetadata: vi.fn(),
		}

		cleanup = useKeyboardShortcuts(handlers)
	})

	afterEach(() => {
		cleanup?.()
	})

	it('toggles play/pause when in player view with an active standalone track', async () => {
		uiStore.setActiveView('player')

		const mockTrack: StandaloneTrack = {
			id: 'st_1',
			file_path: '/music/song.mp3',
			title: 'Song 1',
			artist: 'Artist',
			duration_ms: 200000,
			format: 'mp3',
			is_in_library: false,
		}

		await playerStore.playStandalone(mockTrack, false)

		const toggleSpy = vi.spyOn(playerStore, 'togglePlayPause')

		const event = new KeyboardEvent('keydown', {
			code: 'Space',
			bubbles: true,
			cancelable: true,
		})
		const preventSpy = vi.spyOn(event, 'preventDefault')

		window.dispatchEvent(event)

		expect(preventSpy).toHaveBeenCalled()
		expect(toggleSpy).toHaveBeenCalled()
		expect(handlers.onPlayPause).not.toHaveBeenCalled()
	})

	it('plays first recent standalone track when in player view with no active track', async () => {
		uiStore.setActiveView('player')

		const recentTrack: StandaloneTrack = {
			id: 'recent_1',
			file_path: '/music/recent.flac',
			title: 'Recent Song',
			artist: 'Producer',
			duration_ms: 180000,
			format: 'flac',
			is_in_library: false,
		}

		await recentTracksStore.addTrack(recentTrack)

		const playStandaloneSpy = vi.spyOn(playerStore, 'playStandalone')

		const event = new KeyboardEvent('keydown', {
			code: 'Space',
			bubbles: true,
			cancelable: true,
		})
		const preventSpy = vi.spyOn(event, 'preventDefault')

		window.dispatchEvent(event)

		expect(preventSpy).toHaveBeenCalled()
		expect(playStandaloneSpy).toHaveBeenCalledWith(
			expect.objectContaining({
				id: 'recent_1',
				file_path: '/music/recent.flac',
			}),
			false
		)
		expect(handlers.onPlayPause).not.toHaveBeenCalled()
	})

	it('delegates to handlers.onPlayPause when in library view', () => {
		uiStore.setActiveView('library')

		const event = new KeyboardEvent('keydown', {
			code: 'Space',
			bubbles: true,
			cancelable: true,
		})
		const preventSpy = vi.spyOn(event, 'preventDefault')

		window.dispatchEvent(event)

		expect(preventSpy).toHaveBeenCalled()
		expect(handlers.onPlayPause).toHaveBeenCalled()
	})

	it('ignores digit keys outside the Player and Library views', () => {
		uiStore.setActiveView('discovery')
		const jumpSpy = vi.spyOn(playerStore, 'jumpToCueIndex')
		window.dispatchEvent(new KeyboardEvent('keydown', { key: '3', bubbles: true, cancelable: true }))
		expect(jumpSpy).not.toHaveBeenCalled()
	})

	it('maps key 3 to hot cue pad 3 in the player view with a track loaded', async () => {
		uiStore.setActiveView('player')
		await playerStore.playStandalone(
			{
				id: 'st_2',
				file_path: '/music/cue.mp3',
				title: 'Cue',
				artist: 'A',
				duration_ms: 200000,
				format: 'mp3',
				is_in_library: false,
			},
			false
		)
		const jumpSpy = vi.spyOn(playerStore, 'jumpToCueIndex')
		window.dispatchEvent(new KeyboardEvent('keydown', { key: '3', bubbles: true, cancelable: true }))
		expect(jumpSpy).toHaveBeenCalledWith(3)
	})
})

describe('useKeyboardShortcuts - Enter and Space on a focused control', () => {
	let cleanup: () => void
	let handlers: KeyboardShortcutHandlers
	let host: HTMLDivElement

	beforeEach(() => {
		vi.clearAllMocks()
		playerStore.reset()
		uiStore.reset()
		uiStore.setActiveView('library')
		handlers = {
			onPlayPause: vi.fn(),
			onFocusSearch: vi.fn(),
			onClearSelection: vi.fn(),
			onSelectAll: vi.fn(),
			onOpenSettings: vi.fn(),
			onNewPlaylist: vi.fn(),
			onNewFolder: vi.fn(),
			onImport: vi.fn(),
			onDeleteSelected: vi.fn().mockReturnValue(false),
			onPlaySelected: vi.fn(),
			onSeekBackward: vi.fn(),
			onSeekForward: vi.fn(),
			onFineSeekBackward: vi.fn(),
			onFineSeekForward: vi.fn(),
			onPreviousTrack: vi.fn(),
			onNextTrack: vi.fn(),
			onVolumeUp: vi.fn(),
			onVolumeDown: vi.fn(),
			onToggleMute: vi.fn(),
			onSelectPreviousTrack: vi.fn(),
			onSelectNextTrack: vi.fn(),
			onQuickExport: vi.fn(),
			onJumpToPlayingTrack: vi.fn(),
			onToggleView: vi.fn(),
			onAddRelease: vi.fn(),
			onRefreshMetadata: vi.fn(),
		}
		cleanup = useKeyboardShortcuts(handlers)
		host = document.createElement('div')
		document.body.appendChild(host)
	})

	afterEach(() => {
		cleanup?.()
		host.remove()
	})

	/** Focus `element` the way Tab does: a key press, then the focus move. */
	function focusFromKeyboard(element: HTMLElement): void {
		element.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true }))
		element.focus()
	}

	/** Focus `element` the way a click does: a pointer press, then the focus move. */
	function focusFromPointer(element: HTMLElement): void {
		element.dispatchEvent(new Event('pointerdown', { bubbles: true }))
		element.focus()
	}

	function press(target: HTMLElement, init: KeyboardEventInit): KeyboardEvent {
		const event = new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init })
		target.dispatchEvent(event)
		return event
	}

	function make(html: string): HTMLElement {
		host.innerHTML = html
		return host.firstElementChild as HTMLElement
	}

	it('leaves Space to a button focused from the keyboard', () => {
		const button = make('<button type="button">Shuffle</button>')
		focusFromKeyboard(button)
		const event = press(button, { code: 'Space', key: ' ' })
		expect(event.defaultPrevented).toBe(false)
		expect(handlers.onPlayPause).not.toHaveBeenCalled()
	})

	it('leaves Enter to a link and to a switch focused from the keyboard', () => {
		const link = make('<a href="#x">Open</a>')
		focusFromKeyboard(link)
		expect(press(link, { key: 'Enter' }).defaultPrevented).toBe(false)

		const toggle = make('<button type="button" role="switch" aria-checked="false">Auto</button>')
		focusFromKeyboard(toggle)
		expect(press(toggle, { key: 'Enter' }).defaultPrevented).toBe(false)
		expect(handlers.onPlaySelected).not.toHaveBeenCalled()
	})

	it('keeps Space = play / pause on a button focused by a click', () => {
		const button = make('<button type="button">Shuffle</button>')
		focusFromPointer(button)
		const event = press(button, { code: 'Space', key: ' ' })
		expect(event.defaultPrevented).toBe(true)
		expect(handlers.onPlayPause).toHaveBeenCalled()
	})

	it('keeps Space = play / pause on a slider (the waveform) focused from the keyboard', () => {
		const slider = make('<div role="slider" tabindex="0" aria-valuenow="0">Waveform</div>')
		focusFromKeyboard(slider)
		press(slider, { code: 'Space', key: ' ' })
		expect(handlers.onPlayPause).toHaveBeenCalled()
	})

	it('steps aside on a list row only for a key the row handled itself', () => {
		const row = make('<div role="row" tabindex="0">Release</div>')
		row.addEventListener('keydown', (e) => {
			if (e.key === ' ') e.preventDefault()
		})
		focusFromKeyboard(row)
		press(row, { code: 'Space', key: ' ' })
		expect(handlers.onPlayPause).not.toHaveBeenCalled()

		// The row has no Enter action of its own: Enter still plays the selection
		press(row, { key: 'Enter' })
		expect(handlers.onPlaySelected).toHaveBeenCalled()
	})

	it('keeps both shortcuts when nothing is focused', () => {
		;(document.activeElement as HTMLElement | null)?.blur()
		press(document.body, { code: 'Space', key: ' ' })
		press(document.body, { key: 'Enter' })
		expect(handlers.onPlayPause).toHaveBeenCalled()
		expect(handlers.onPlaySelected).toHaveBeenCalled()
	})

	// CRA-196: a focused widget that used an arrow (it called preventDefault) owns it, even if it let the event bubble.
	it('leaves an arrow a focused widget already used: no seek, no volume change', () => {
		const group = make('<div role="radiogroup"><button type="button" role="radio">Week</button></div>')
		const radio = group.querySelector('button') as HTMLElement
		radio.addEventListener('keydown', (e) => e.preventDefault())
		focusFromKeyboard(radio)
		for (const key of ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown']) press(radio, { key })
		expect(handlers.onSeekBackward).not.toHaveBeenCalled()
		expect(handlers.onSeekForward).not.toHaveBeenCalled()
		expect(handlers.onVolumeUp).not.toHaveBeenCalled()
		expect(handlers.onVolumeDown).not.toHaveBeenCalled()
	})

	it('keeps the arrow shortcuts everywhere else', () => {
		;(document.activeElement as HTMLElement | null)?.blur()
		for (const key of ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown']) press(document.body, { key })
		press(document.body, { key: 'ArrowRight', shiftKey: true })
		expect(handlers.onSeekBackward).toHaveBeenCalledOnce()
		expect(handlers.onSeekForward).toHaveBeenCalledOnce()
		expect(handlers.onVolumeUp).toHaveBeenCalledOnce()
		expect(handlers.onVolumeDown).toHaveBeenCalledOnce()
		expect(handlers.onNextTrack).toHaveBeenCalledOnce()
	})
})

describe('controlOwnsActivationKey', () => {
	it('recognises native and ARIA controls, and rows only when they handled the key', () => {
		const make = (html: string) => {
			const wrapper = document.createElement('div')
			wrapper.innerHTML = html
			return wrapper.firstElementChild as HTMLElement
		}
		expect(controlOwnsActivationKey(make('<button></button>'), false)).toBe(true)
		expect(controlOwnsActivationKey(make('<a href="#">x</a>'), false)).toBe(true)
		expect(controlOwnsActivationKey(make('<a>x</a>'), false)).toBe(false)
		expect(controlOwnsActivationKey(make('<div role="checkbox"></div>'), false)).toBe(true)
		expect(controlOwnsActivationKey(make('<div role="slider"></div>'), true)).toBe(false)
		expect(controlOwnsActivationKey(make('<div role="treeitem"></div>'), false)).toBe(false)
		expect(controlOwnsActivationKey(make('<div role="treeitem"></div>'), true)).toBe(true)
		expect(controlOwnsActivationKey(document.body, true)).toBe(false)
		expect(controlOwnsActivationKey(null, true)).toBe(false)
	})
})
