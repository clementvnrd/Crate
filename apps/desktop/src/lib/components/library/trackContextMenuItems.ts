import type { Track, TrackColor, Playlist, ContextMenuItem } from '$shared/types'
import { TRACK_COLORS } from '$shared/types'

export interface BuildTrackContextMenuOptions {
	selectedTracks: Track[]
	playlists?: Playlist[]
	currentPlaylistId?: string | null
	isAnalyzing?: boolean
	hasMissingTrack: boolean
	revealLabel?: string
	currentColor?: TrackColor | null
	t?: (key: string, options?: unknown) => string
	onRevealInExplorer?: () => void
	onAddToPlaylist?: (playlistId: string) => void
	onRemoveFromPlaylist?: () => void
	onRemoveFromLibrary?: () => void
	onDeleteTrackAndFile?: () => void
	onRelocate?: (track: Track) => void
	onSetColor?: (color: TrackColor | null) => void
	onAnalyze?: () => void
	onResyncMik?: () => void
}

/**
 * Builds the list of context menu items for track rows in the library.
 */
export function buildTrackContextMenuItems(options: BuildTrackContextMenuOptions): ContextMenuItem[] {
	const {
		selectedTracks,
		playlists = [],
		currentPlaylistId = null,
		isAnalyzing = false,
		hasMissingTrack,
		revealLabel = 'Reveal in Explorer',
		currentColor = null,
		t = (key: string) => key,
		onRevealInExplorer,
		onAddToPlaylist,
		onRemoveFromPlaylist,
		onRemoveFromLibrary,
		onDeleteTrackAndFile,
		onRelocate,
		onSetColor,
		onAnalyze,
		onResyncMik,
	} = options

	const items: ContextMenuItem[] = []

	// "Sync with Mixed In Key"
	if (onResyncMik) {
		items.push({
			id: 'resync-mik',
			label: t('contextMenu.resyncMik'),
			icon: 'refresh-cw',
			action: onResyncMik,
			disabled: isAnalyzing,
		})
	}

	// "Analyze" - analyze tracks for BPM and key (disabled during analysis)
	if (onAnalyze) {
		items.push({
			id: 'analyze',
			label: t('contextMenu.analyze'),
			icon: 'activity',
			action: onAnalyze,
			disabled: isAnalyzing,
		})
	}

	// "Relocate..." - only for single missing track (disabled during analysis)
	if (hasMissingTrack && onRelocate) {
		items.push({
			id: 'relocate',
			label: t('contextMenu.relocate'),
			icon: 'folder',
			action: () => selectedTracks[0] && onRelocate(selectedTracks[0]),
			disabled: isAnalyzing,
		})
		items.push({
			id: 'relocate-divider',
			label: '',
			divider: true,
		})
	}

	// "View in Finder/Explorer" - only for single track selection
	if (selectedTracks.length === 1 && onRevealInExplorer) {
		items.push({
			id: 'reveal-in-explorer',
			label: revealLabel,
			icon: 'folder-open',
			action: onRevealInExplorer,
		})
		items.push({
			id: 'reveal-divider',
			label: '',
			divider: true,
		})
	}

	// Add to Playlist submenu (exclude smart playlists since their content is rule-generated)
	const playlistItems = playlists.filter((p) => !p.is_folder && !p.is_smart && p.context === 'library')
	if (playlistItems.length > 0) {
		items.push({
			id: 'add-to-playlist',
			label: t('contextMenu.addToPlaylist'),
			icon: 'list-plus',
			submenu: playlistItems.map((playlist) => ({
				id: `playlist-${playlist.id}`,
				label: playlist.name,
				action: onAddToPlaylist ? () => onAddToPlaylist(playlist.id) : undefined,
			})),
		})
	} else {
		items.push({
			id: 'add-to-playlist',
			label: t('contextMenu.addToPlaylist'),
			icon: 'list-plus',
			disabled: true,
		})
	}

	// Set Color submenu
	if (onSetColor) {
		const colorItems: ContextMenuItem[] = TRACK_COLORS.map((color) => ({
			id: `color-${color.id}`,
			label: t(`colors.${color.id}`),
			colorDot: color.hex,
			selected: currentColor === color.id,
			action: () => onSetColor(color.id),
		}))
		colorItems.push({
			id: 'color-divider',
			label: '',
			divider: true,
		})
		colorItems.push({
			id: 'remove-color',
			label: t('contextMenu.removeColor'),
			icon: 'minus-circle',
			variant: 'danger',
			action: () => onSetColor(null),
		})
		items.push({
			id: 'set-color',
			label: t('contextMenu.setColor'),
			icon: 'palette',
			submenu: colorItems,
		})
	}

	// Divider before removal actions
	items.push({
		id: 'removal-divider',
		label: '',
		divider: true,
	})

	// "Remove from Playlist" - only when viewing a non-smart playlist
	const currentPlaylist = currentPlaylistId ? playlists.find((p) => p.id === currentPlaylistId) : null
	if (currentPlaylistId && !currentPlaylist?.is_smart && onRemoveFromPlaylist) {
		items.push({
			id: 'remove-from-playlist',
			label: t('contextMenu.removeFromPlaylist'),
			icon: 'list-minus',
			variant: 'danger',
			action: onRemoveFromPlaylist,
			disabled: isAnalyzing,
		})
	}

	// "Supprimer de la bibliothèque" - always visible
	items.push({
		id: 'remove-from-library',
		label: t('contextMenu.removeFromLibrary'),
		icon: 'trash',
		variant: 'danger',
		action: onRemoveFromLibrary,
		disabled: isAnalyzing,
	})

	// "Supprimer le morceau" (Mettre à la corbeille) - only when file exists on disk
	if (!hasMissingTrack && onDeleteTrackAndFile) {
		items.push({
			id: 'delete-track-and-file',
			label: t('contextMenu.deleteTrackAndFile'),
			icon: 'trash',
			variant: 'danger',
			action: onDeleteTrackAndFile,
			disabled: isAnalyzing,
		})
	}

	return items
}
