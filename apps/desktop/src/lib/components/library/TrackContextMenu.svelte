<script lang="ts">
	import type { Track, TrackColor, Playlist, ContextMenuItem } from '$shared/types'
	import ContextMenu from '$lib/components/common/ContextMenu.svelte'
	import { missingTrackIds } from '$lib/stores'
	import { translate } from '$shared/i18n'
	import { get } from 'svelte/store'
	import { buildTrackContextMenuItems } from './trackContextMenuItems'

	type Props = {
		open: boolean
		x: number
		y: number
		selectedTracks: Track[]
		playlists: Playlist[]
		currentPlaylistId: string | null
		isAnalyzing?: boolean
		onClose: () => void
		onClosed?: () => void
		onRevealInExplorer: () => void
		onAddToPlaylist: (playlistId: string) => void
		onRemoveFromPlaylist: () => void
		onRemoveFromLibrary: () => void
		onDeleteTrackAndFile?: () => void
		onRelocate?: (track: Track) => void
		onSetColor?: (color: TrackColor | null) => void
		onAnalyze?: () => void
		onResyncMik?: () => void
		onBuildSet?: () => void
	}

	let {
		open,
		x,
		y,
		selectedTracks,
		playlists,
		currentPlaylistId,
		isAnalyzing = false,
		onClose,
		onClosed,
		onRevealInExplorer,
		onAddToPlaylist,
		onRemoveFromPlaylist,
		onRemoveFromLibrary,
		onDeleteTrackAndFile,
		onRelocate,
		onSetColor,
		onAnalyze,
		onResyncMik,
		onBuildSet,
	}: Props = $props()

	// Platform-specific label for "View in Finder/Explorer"
	const revealLabel = $derived.by(() => {
		const ua = navigator.userAgent
		if (ua.includes('Mac')) return get(translate)('contextMenu.viewInFinder')
		if (ua.includes('Windows')) return get(translate)('contextMenu.viewInExplorer')
		return get(translate)('contextMenu.viewInFileManager')
	})

	// Check if any selected track is missing
	const hasMissingTrack = $derived(selectedTracks.some((t) => $missingTrackIds.has(t.id)))

	// Get current color (for single track or common color across multi-selection)
	const currentColor = $derived.by(() => {
		if (selectedTracks.length === 0) return null
		const firstColor = selectedTracks[0].color
		// Only show as selected if all tracks have the same color
		return selectedTracks.every((t) => t.color === firstColor) ? firstColor : null
	})

	// Build menu items
	const menuItems = $derived.by<ContextMenuItem[]>(() => {
		return buildTrackContextMenuItems({
			selectedTracks,
			playlists,
			currentPlaylistId,
			isAnalyzing,
			hasMissingTrack,
			revealLabel,
			currentColor,
			t: (key) => get(translate)(key),
			onRevealInExplorer,
			onAddToPlaylist,
			onRemoveFromPlaylist,
			onRemoveFromLibrary,
			onDeleteTrackAndFile,
			onRelocate,
			onSetColor,
			onAnalyze,
			onResyncMik,
			onBuildSet,
		})
	})
</script>

<ContextMenu {open} {x} {y} items={menuItems} {onClose} {onClosed} />
