import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { useKeyboardShortcuts, type KeyboardShortcutHandlers } from './useKeyboardShortcuts'
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
			{ id: 'st_2', file_path: '/music/cue.mp3', title: 'Cue', artist: 'A', duration_ms: 200000, format: 'mp3', is_in_library: false },
			false
		)
		const jumpSpy = vi.spyOn(playerStore, 'jumpToCueIndex')
		window.dispatchEvent(new KeyboardEvent('keydown', { key: '3', bubbles: true, cancelable: true }))
		expect(jumpSpy).toHaveBeenCalledWith(3)
	})
})
