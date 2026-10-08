import { get } from 'svelte/store'
import { isInputFocused, isNativeDialogOpen } from '$shared/utils'
import {
	activeView,
	currentTrack,
	standaloneTrack,
	playerStore,
	recentStandaloneTracks,
	playbackPosition,
} from '$lib/stores'

// =============================================================================
// Types
// =============================================================================

export interface KeyboardShortcutHandlers {
	onPlayPause: () => void
	onFocusSearch: () => void
	onClearSelection: () => void
	onSelectAll: () => void
	onOpenSettings: () => void
	onNewPlaylist: () => void
	onNewFolder: () => void
	onImport: () => void
	onDeleteSelected: () => boolean
	onPlaySelected: () => void
	onSeekBackward: () => void
	onSeekForward: () => void
	onFineSeekBackward: () => void
	onFineSeekForward: () => void
	onPreviousTrack: () => void
	onNextTrack: () => void
	onVolumeUp: () => void
	onVolumeDown: () => void
	onToggleMute: () => void
	onSelectPreviousTrack: () => void
	onSelectNextTrack: () => void
	onQuickExport: () => void
	onJumpToPlayingTrack: () => void
	onToggleView: () => void
	onAddRelease: () => void
	onRefreshMetadata: () => void
	isModalOpen?: () => boolean
}

// =============================================================================
// Enter and Space on a focused control
// =============================================================================

/**
 * Controls that answer Enter and Space themselves, natively or through their ARIA role. When one of them has
 * keyboard focus, the global Enter (play the selection) and Space (play / pause) shortcuts step aside so the key
 * activates the control instead of being swallowed. Sliders are deliberately left out: Enter and Space mean nothing
 * to a slider, so on the waveform or the seek bar Space keeps toggling playback.
 */
const CONTROL_SELECTOR = [
	'button',
	'a[href]',
	'summary',
	'select',
	'[role="button"]',
	'[role="link"]',
	'[role="switch"]',
	'[role="checkbox"]',
	'[role="radio"]',
	'[role="tab"]',
	'[role="menuitem"]',
	'[role="menuitemcheckbox"]',
	'[role="menuitemradio"]',
	'[role="option"]',
].join(', ')

/**
 * List rows only step aside for a key they handled themselves (they called `preventDefault`): a row without an
 * Enter or Space action of its own leaves the key to the shortcut, as before.
 */
const ROW_SELECTOR = '[role="row"], [role="treeitem"], [role="gridcell"]'

/**
 * Whether the focused `element` owns Enter and Space, so the global shortcut must not run. `handledByElement` is
 * whether the element's own handler already consumed the key (`event.defaultPrevented`).
 */
export function controlOwnsActivationKey(element: Element | null, handledByElement: boolean): boolean {
	if (!element || element === document.body || element === document.documentElement) return false
	if (element.matches(CONTROL_SELECTOR)) return true
	return handledByElement && element.matches(ROW_SELECTOR)
}

// =============================================================================
// Hook
// =============================================================================

/**
 * Set up global keyboard shortcuts for the application.
 *
 * Shortcuts:
 * - Space: toggle play/pause (when not typing)
 * - Cmd/Ctrl+F: focus search input
 * - Escape: clear selection
 * - Cmd/Ctrl+A: select all (text in input, or tracks)
 * - Cmd/Ctrl+,: open settings
 * - Cmd/Ctrl+N: new playlist
 * - Cmd/Ctrl+Shift+N: new folder
 * - Cmd/Ctrl+L: import files
 * - Delete/Backspace: remove selected tracks
 * - Enter: play selected track
 * - Left Arrow: seek backward 10s
 * - Right Arrow: seek forward 10s
 * - Cmd/Ctrl+Left Arrow: fine seek backward 1s
 * - Cmd/Ctrl+Right Arrow: fine seek forward 1s
 * - Shift+Left Arrow: previous track
 * - Shift+Right Arrow: next track
 * - Up Arrow: volume up 10%
 * - Down Arrow: volume down 10%
 * - M: toggle mute
 * - Cmd/Ctrl+Up Arrow: select previous track
 * - Cmd/Ctrl+Down Arrow: select next track
 * - Cmd/Ctrl+E: quick export
 * - Cmd/Ctrl+J: jump to playing track
 * - Shift+Tab: toggle between Library and Discovery views
 * - Cmd/Ctrl+D: add release (handled by native menu)
 * - Cmd/Ctrl+R: refresh metadata for selected discovery releases
 *
 * Enter and Space are left to a button, link, switch, checkbox, tab, menu item or list row that has keyboard focus
 * (see `controlOwnsActivationKey`), so a control reached with Tab can be activated; everywhere else they keep their
 * shortcut. The arrow keys are left to a focused widget that handles them itself (segmented control, select,
 * waveform, recent files list, heatmap).
 *
 * @returns Cleanup function to remove the event listener
 */
export function useKeyboardShortcuts(handlers: KeyboardShortcutHandlers): () => void {
	const {
		onPlayPause,
		onFocusSearch,
		onClearSelection,
		onSelectAll,
		onOpenSettings,
		onNewPlaylist,
		onNewFolder,
		onImport,
		onDeleteSelected,
		onPlaySelected,
		onSeekBackward,
		onSeekForward,
		onFineSeekBackward,
		onFineSeekForward,
		onPreviousTrack,
		onNextTrack,
		onVolumeUp,
		onVolumeDown,
		onToggleMute,
		onSelectPreviousTrack,
		onSelectNextTrack,
		onQuickExport,
		onJumpToPlayingTrack,
		onToggleView,
		onAddRelease,
		onRefreshMetadata,
		isModalOpen,
	} = handlers

	// How the focused element got its focus. Enter and Space only go to a control focused from the keyboard (Tab,
	// arrows, focus given back by a dialog closed with Escape): a control focused by a click (Chromium focuses
	// buttons on click, every engine focuses rows with a tabindex) keeps today's Space = play / pause.
	let lastInputWasPointer = false
	let focusCameFromPointer = false

	function handlePointerDown(): void {
		lastInputWasPointer = true
	}

	function handleAnyKeydown(): void {
		lastInputWasPointer = false
	}

	function handleFocusIn(): void {
		focusCameFromPointer = lastInputWasPointer
	}

	function handleKeydown(e: KeyboardEvent): void {
		if (isModalOpen?.()) return
		if (isNativeDialogOpen()) return
		const inputFocused = isInputFocused()

		// Shift+Tab: toggle between Library and Discovery views
		if (e.key === 'Tab' && e.shiftKey && !inputFocused) {
			e.preventDefault()
			onToggleView()
			return
		}

		// Enter or Space on a control that has keyboard focus: the control handles it, not the shortcut
		if (
			(e.code === 'Space' || e.key === 'Enter') &&
			!inputFocused &&
			!focusCameFromPointer &&
			controlOwnsActivationKey(document.activeElement, e.defaultPrevented)
		) {
			return
		}

		// Space: toggle play/pause
		if (e.code === 'Space' && !inputFocused) {
			e.preventDefault()
			if (get(activeView) === 'player') {
				const player = get(playerStore)
				// Anything loaded or playing (library track, standalone file, Beatport or discovery
				// preview) is paused/resumed; a recent file is only started when nothing is loaded.
				const active =
					player.playbackState.is_playing ||
					get(currentTrack) ||
					get(standaloneTrack) ||
					player.previewInfo ||
					player.beatportTrack
				if (active) {
					playerStore.togglePlayPause()
				} else {
					const recents = get(recentStandaloneTracks)
					if (recents.length > 0) {
						playerStore.playStandalone(recents[0], recents[0].is_in_library)
					}
				}
				return
			}
			onPlayPause()
		}

		// Cmd/Ctrl+F: focus search
		if ((e.metaKey || e.ctrlKey) && e.key === 'f') {
			e.preventDefault()
			onFocusSearch()
		}

		// Escape: clear selection
		if (e.key === 'Escape') {
			onClearSelection()
		}

		// Cmd/Ctrl+A: select all (text in input, or tracks)
		if ((e.metaKey || e.ctrlKey) && e.key === 'a') {
			e.preventDefault()
			if (inputFocused) {
				const input = document.activeElement as HTMLInputElement | HTMLTextAreaElement
				input.select()
			} else {
				onSelectAll()
			}
		}

		// Cmd/Ctrl+,: open settings
		if ((e.metaKey || e.ctrlKey) && e.key === ',') {
			e.preventDefault()
			onOpenSettings()
		}

		// Cmd/Ctrl+N: new playlist
		if ((e.metaKey || e.ctrlKey) && !e.shiftKey && e.key === 'n') {
			e.preventDefault()
			onNewPlaylist()
		}

		// Cmd/Ctrl+Shift+N: new folder
		if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 'N') {
			e.preventDefault()
			onNewFolder()
		}

		// Cmd/Ctrl+L: import files
		if ((e.metaKey || e.ctrlKey) && e.key === 'l') {
			e.preventDefault()
			onImport()
		}

		// Cmd/Ctrl+E: quick export
		if ((e.metaKey || e.ctrlKey) && e.key === 'e') {
			e.preventDefault()
			onQuickExport()
		}

		// Cmd/Ctrl+J: jump to playing track
		if ((e.metaKey || e.ctrlKey) && e.key === 'j') {
			e.preventDefault()
			onJumpToPlayingTrack()
		}

		// Cmd/Ctrl+R: refresh metadata for selected discovery releases
		if ((e.metaKey || e.ctrlKey) && e.key === 'r' && !inputFocused) {
			e.preventDefault()
			onRefreshMetadata()
		}

		// Delete/Backspace: remove selected tracks (when not typing)
		if ((e.key === 'Delete' || e.key === 'Backspace') && !inputFocused) {
			if (onDeleteSelected()) {
				e.preventDefault()
			}
		}

		// Enter: play selected track (when not typing)
		if (e.key === 'Enter' && !inputFocused) {
			e.preventDefault()
			onPlaySelected()
		}

		// Number keys 1 to 8: jump to hot cues — only in the Player and Library views, with a track
		// loaded, and never while typing (other views keep their own use of the digits)
		const view = get(activeView)
		const cueKeysActive = (view === 'player' || view === 'library') && !!(get(currentTrack) || get(standaloneTrack))
		if (cueKeysActive && !inputFocused && !e.metaKey && !e.ctrlKey && !e.altKey) {
			const num = parseInt(e.key, 10)
			if (num >= 1 && num <= 8) {
				e.preventDefault()
				playerStore.jumpToCueIndex(num)
				return
			}
		}

		// Arrow keys (when not typing). A focused widget that already used the arrow (a segmented control, a select,
		// the waveform, a keyboard grid: it called `preventDefault`) owns it, so it never also seeks or changes the
		// volume (CRA-196). Those widgets also stop the event; this guard covers one that forgets to.
		if (!inputFocused && !e.defaultPrevented) {
			// Cmd/Ctrl+Up: select previous track
			if ((e.metaKey || e.ctrlKey) && e.key === 'ArrowUp') {
				e.preventDefault()
				onSelectPreviousTrack()
				return
			}

			// Cmd/Ctrl+Down: select next track
			if ((e.metaKey || e.ctrlKey) && e.key === 'ArrowDown') {
				e.preventDefault()
				onSelectNextTrack()
				return
			}

			// Cmd/Ctrl+Left: fine seek backward 1s
			if ((e.metaKey || e.ctrlKey) && e.key === 'ArrowLeft') {
				e.preventDefault()
				onFineSeekBackward()
				return
			}

			// Cmd/Ctrl+Right: fine seek forward 1s
			if ((e.metaKey || e.ctrlKey) && e.key === 'ArrowRight') {
				e.preventDefault()
				onFineSeekForward()
				return
			}

			// Shift+Left: in player mode, seek backward 15s (32 beats / 8 bars phrase jump). Otherwise, previous track
			if (e.shiftKey && e.key === 'ArrowLeft') {
				e.preventDefault()
				if (get(activeView) === 'player') {
					playerStore.seek(Math.max(0, get(playbackPosition) - 15000))
				} else {
					onPreviousTrack()
				}
				return
			}

			// Shift+Right: in player mode, seek forward 15s (32 beats / 8 bars phrase jump). Otherwise, next track
			if (e.shiftKey && e.key === 'ArrowRight') {
				e.preventDefault()
				if (get(activeView) === 'player') {
					playerStore.seek(get(playbackPosition) + 15000)
				} else {
					onNextTrack()
				}
				return
			}

			// Left Arrow: seek backward 10s
			if (e.key === 'ArrowLeft') {
				e.preventDefault()
				onSeekBackward()
			}

			// Right Arrow: seek forward 10s
			if (e.key === 'ArrowRight') {
				e.preventDefault()
				onSeekForward()
			}

			// Up Arrow: volume up
			if (e.key === 'ArrowUp') {
				e.preventDefault()
				onVolumeUp()
			}

			// Down Arrow: volume down
			if (e.key === 'ArrowDown') {
				e.preventDefault()
				onVolumeDown()
			}
		}

		// M: toggle mute (when not typing)
		if (e.key === 'm' && !inputFocused && !e.metaKey && !e.ctrlKey) {
			e.preventDefault()
			onToggleMute()
		}
	}

	window.addEventListener('pointerdown', handlePointerDown, true)
	window.addEventListener('keydown', handleAnyKeydown, true)
	window.addEventListener('focusin', handleFocusIn, true)
	window.addEventListener('keydown', handleKeydown)

	return () => {
		window.removeEventListener('pointerdown', handlePointerDown, true)
		window.removeEventListener('keydown', handleAnyKeydown, true)
		window.removeEventListener('focusin', handleFocusIn, true)
		window.removeEventListener('keydown', handleKeydown)
	}
}
