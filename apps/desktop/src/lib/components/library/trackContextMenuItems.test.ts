import { describe, it, expect, vi } from 'vitest'
import { buildTrackContextMenuItems } from './trackContextMenuItems'
import type { Track, Playlist } from '$shared/types'

describe('buildTrackContextMenuItems', () => {
	const mockTrack: Track = {
		id: 'track-1',
		file_path: '/music/track1.mp3',
		file_hash: 'hash1',
		title: 'Track 1',
		artist: 'Artist 1',
		album: 'Album 1',
		year: 2024,
		genre: 'House',
		label: 'Label 1',
		catalog_number: null,
		duration_ms: 180000,
		bpm: 124,
		key: '8A',
		energy: 7,
		bitrate: 320,
		sample_rate: 44100,
		format: 'mp3',
		analysis_source: null,
		waveform_data: null,
		rating: 5,
		play_count: 10,
		date_added: '2024-01-01T00:00:00Z',
		date_modified: '2024-01-01T00:00:00Z',
		last_played: null,
		rekordbox_id: null,
		artwork_path: null,
		artwork_source: null,
		color: null,
		library_root_id: null,
		relative_path: null,
		tags: [],
	}

	const mockTrack2: Track = {
		...mockTrack,
		id: 'track-2',
		file_path: '/music/track2.mp3',
		title: 'Track 2',
	}

	const mockPlaylist: Playlist = {
		id: 'playlist-1',
		name: 'My Playlist',
		parent_id: null,
		is_folder: false,
		is_smart: false,
		smart_rules: null,
		sort_order: 1,
		date_created: '2024-01-01T00:00:00Z',
		date_modified: '2024-01-01T00:00:00Z',
		track_count: 5,
		context: 'library',
	}

	const defaultHandlers = {
		onRevealInExplorer: vi.fn(),
		onAddToPlaylist: vi.fn(),
		onRemoveFromPlaylist: vi.fn(),
		onRemoveFromLibrary: vi.fn(),
		onDeleteTrackAndFile: vi.fn(),
		onRelocate: vi.fn(),
		onSetColor: vi.fn(),
		onAnalyze: vi.fn(),
		onResyncMik: vi.fn(),
		onBuildSet: vi.fn(),
		t: (key: string) => key,
	}

	describe('Normal track (file exists, hasMissingTrack: false)', () => {
		it('includes "remove-from-library" and "delete-track-and-file", but NOT "relocate"', () => {
			const items = buildTrackContextMenuItems({
				selectedTracks: [mockTrack],
				hasMissingTrack: false,
				...defaultHandlers,
			})

			const itemIds = items.map((i) => i.id)

			// "remove-from-library" must be present
			expect(itemIds).toContain('remove-from-library')
			const removeLibItem = items.find((i) => i.id === 'remove-from-library')
			expect(removeLibItem?.variant).toBe('danger')
			expect(removeLibItem?.label).toBe('contextMenu.removeFromLibrary')

			// "delete-track-and-file" must be present when not missing
			expect(itemIds).toContain('delete-track-and-file')
			const deleteTrackItem = items.find((i) => i.id === 'delete-track-and-file')
			expect(deleteTrackItem?.variant).toBe('danger')
			expect(deleteTrackItem?.label).toBe('contextMenu.deleteTrackAndFile')

			// "relocate" must NOT be present
			expect(itemIds).not.toContain('relocate')
			expect(itemIds).not.toContain('relocate-divider')
		})

		it('does not include "delete-track-and-file" if onDeleteTrackAndFile callback is omitted', () => {
			const { onDeleteTrackAndFile: _, ...handlersWithoutDelete } = defaultHandlers
			const items = buildTrackContextMenuItems({
				selectedTracks: [mockTrack],
				hasMissingTrack: false,
				...handlersWithoutDelete,
			})

			const itemIds = items.map((i) => i.id)
			expect(itemIds).toContain('remove-from-library')
			expect(itemIds).not.toContain('delete-track-and-file')
		})
	})

	describe('Missing track (file missing on disk, hasMissingTrack: true)', () => {
		it('includes "remove-from-library" and "relocate", but NOT "delete-track-and-file"', () => {
			const items = buildTrackContextMenuItems({
				selectedTracks: [mockTrack],
				hasMissingTrack: true,
				...defaultHandlers,
			})

			const itemIds = items.map((i) => i.id)

			// "remove-from-library" must always be present
			expect(itemIds).toContain('remove-from-library')

			// "delete-track-and-file" must NOT be present for missing tracks
			expect(itemIds).not.toContain('delete-track-and-file')

			// "relocate" must be present
			expect(itemIds).toContain('relocate')
			expect(itemIds).toContain('relocate-divider')
			const relocateItem = items.find((i) => i.id === 'relocate')
			expect(relocateItem?.label).toBe('contextMenu.relocate')
		})

		it('calls onRelocate with the first selected track', () => {
			const onRelocate = vi.fn()
			const items = buildTrackContextMenuItems({
				selectedTracks: [mockTrack, mockTrack2],
				hasMissingTrack: true,
				...defaultHandlers,
				onRelocate,
			})

			const relocateItem = items.find((i) => i.id === 'relocate')
			expect(relocateItem?.action).toBeDefined()
			relocateItem?.action?.()
			expect(onRelocate).toHaveBeenCalledWith(mockTrack)
		})
	})

	describe('Single vs Multi-selection behavior', () => {
		it('includes "reveal-in-explorer" for single selection only', () => {
			const singleItems = buildTrackContextMenuItems({
				selectedTracks: [mockTrack],
				hasMissingTrack: false,
				revealLabel: 'Afficher dans le Finder',
				...defaultHandlers,
			})
			expect(singleItems.map((i) => i.id)).toContain('reveal-in-explorer')
			const revealItem = singleItems.find((i) => i.id === 'reveal-in-explorer')
			expect(revealItem?.label).toBe('Afficher dans le Finder')

			const multiItems = buildTrackContextMenuItems({
				selectedTracks: [mockTrack, mockTrack2],
				hasMissingTrack: false,
				...defaultHandlers,
			})
			expect(multiItems.map((i) => i.id)).not.toContain('reveal-in-explorer')
		})
	})

	describe('Playlist actions', () => {
		it('shows "remove-from-playlist" only when in a non-smart playlist', () => {
			// In library view (no playlist selected)
			const libraryItems = buildTrackContextMenuItems({
				selectedTracks: [mockTrack],
				currentPlaylistId: null,
				playlists: [mockPlaylist],
				hasMissingTrack: false,
				...defaultHandlers,
			})
			expect(libraryItems.map((i) => i.id)).not.toContain('remove-from-playlist')

			// In regular playlist
			const playlistItems = buildTrackContextMenuItems({
				selectedTracks: [mockTrack],
				currentPlaylistId: 'playlist-1',
				playlists: [mockPlaylist],
				hasMissingTrack: false,
				...defaultHandlers,
			})
			expect(playlistItems.map((i) => i.id)).toContain('remove-from-playlist')

			// In smart playlist
			const smartPlaylist: Playlist = { ...mockPlaylist, id: 'smart-1', is_smart: true }
			const smartItems = buildTrackContextMenuItems({
				selectedTracks: [mockTrack],
				currentPlaylistId: 'smart-1',
				playlists: [smartPlaylist],
				hasMissingTrack: false,
				...defaultHandlers,
			})
			expect(smartItems.map((i) => i.id)).not.toContain('remove-from-playlist')
		})

		it('populates "add-to-playlist" submenu with valid library playlists', () => {
			const smartPlaylist: Playlist = { ...mockPlaylist, id: 'smart-1', is_smart: true, name: 'Smart' }
			const folderPlaylist: Playlist = { ...mockPlaylist, id: 'folder-1', is_folder: true, name: 'Folder' }
			const discoveryPlaylist: Playlist = {
				...mockPlaylist,
				id: 'disc-1',
				context: 'discovery',
				name: 'Discovery',
			}

			const items = buildTrackContextMenuItems({
				selectedTracks: [mockTrack],
				playlists: [mockPlaylist, smartPlaylist, folderPlaylist, discoveryPlaylist],
				hasMissingTrack: false,
				...defaultHandlers,
			})

			const addToPlaylistItem = items.find((i) => i.id === 'add-to-playlist')
			expect(addToPlaylistItem?.disabled).toBeUndefined()
			expect(addToPlaylistItem?.submenu).toHaveLength(1)
			expect(addToPlaylistItem?.submenu?.[0].id).toBe('playlist-playlist-1')
			expect(addToPlaylistItem?.submenu?.[0].label).toBe('My Playlist')
		})

		it('disables "add-to-playlist" when no eligible playlists exist', () => {
			const items = buildTrackContextMenuItems({
				selectedTracks: [mockTrack],
				playlists: [],
				hasMissingTrack: false,
				...defaultHandlers,
			})

			const addToPlaylistItem = items.find((i) => i.id === 'add-to-playlist')
			expect(addToPlaylistItem?.disabled).toBe(true)
			expect(addToPlaylistItem?.submenu).toBeUndefined()
		})
	})

	describe('Analysis state handling (isAnalyzing: true)', () => {
		it('disables mutating and analysis-related actions when analyzing', () => {
			const items = buildTrackContextMenuItems({
				selectedTracks: [mockTrack],
				hasMissingTrack: true,
				currentPlaylistId: 'playlist-1',
				playlists: [mockPlaylist],
				isAnalyzing: true,
				...defaultHandlers,
			})

			const resyncItem = items.find((i) => i.id === 'resync-mik')
			const analyzeItem = items.find((i) => i.id === 'analyze')
			const relocateItem = items.find((i) => i.id === 'relocate')
			const removePlaylistItem = items.find((i) => i.id === 'remove-from-playlist')
			const removeLibItem = items.find((i) => i.id === 'remove-from-library')

			expect(resyncItem?.disabled).toBe(true)
			expect(analyzeItem?.disabled).toBe(true)
			expect(relocateItem?.disabled).toBe(true)
			expect(removePlaylistItem?.disabled).toBe(true)
			expect(removeLibItem?.disabled).toBe(true)
		})
	})

	describe('Build a set', () => {
		it('includes "build-set" when onBuildSet is provided, and calls it', () => {
			const onBuildSet = vi.fn()
			const items = buildTrackContextMenuItems({
				selectedTracks: [mockTrack, mockTrack2],
				hasMissingTrack: false,
				...defaultHandlers,
				onBuildSet,
			})

			const buildSetItem = items.find((i) => i.id === 'build-set')
			expect(buildSetItem).toBeDefined()
			expect(buildSetItem?.label).toBe('contextMenu.buildSet')
			buildSetItem?.action?.()
			expect(onBuildSet).toHaveBeenCalledTimes(1)
		})

		it('omits "build-set" when onBuildSet is not provided', () => {
			const { onBuildSet: _, ...handlersWithoutBuildSet } = defaultHandlers
			const items = buildTrackContextMenuItems({
				selectedTracks: [mockTrack],
				hasMissingTrack: false,
				...handlersWithoutBuildSet,
			})
			expect(items.map((i) => i.id)).not.toContain('build-set')
		})
	})

	describe('Color actions', () => {
		it('constructs color submenu with track colors and remove color', () => {
			const items = buildTrackContextMenuItems({
				selectedTracks: [mockTrack],
				hasMissingTrack: false,
				currentColor: 'pink',
				...defaultHandlers,
			})

			const setColorItem = items.find((i) => i.id === 'set-color')
			expect(setColorItem?.submenu).toBeDefined()
			const pinkOption = setColorItem?.submenu?.find((c) => c.id === 'color-pink')
			expect(pinkOption?.selected).toBe(true)
			const blueOption = setColorItem?.submenu?.find((c) => c.id === 'color-blue')
			expect(blueOption?.selected).toBe(false)
			expect(setColorItem?.submenu?.some((c) => c.id === 'remove-color')).toBe(true)
		})
	})
})
