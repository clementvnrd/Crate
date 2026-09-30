import { writable, derived, get } from 'svelte/store'
import { translate } from '../i18n'
import type { PlayerAlbum, PlayerAlbumTrack, StandaloneTrack } from '../types'
import * as albumApi from '../api/album'
import { playerStore } from './player'
import { toastStore } from './toast'
import { open } from '@tauri-apps/plugin-dialog'
import { toErrorMessage } from '../utils/errors'

interface AlbumsState {
	albums: PlayerAlbum[]
	selectedAlbum: PlayerAlbum | null
	selectedAlbumTracks: PlayerAlbumTrack[]
	isLoading: boolean
	isAdding: boolean
	isPlayingAlbum: boolean
	currentTrackIndex: number
}

const initialState: AlbumsState = {
	albums: [],
	selectedAlbum: null,
	selectedAlbumTracks: [],
	isLoading: false,
	isAdding: false,
	isPlayingAlbum: false,
	currentTrackIndex: 0,
}

function createAlbumsStore() {
	const { subscribe, set, update } = writable<AlbumsState>(initialState)

	async function loadAlbums() {
		update((s) => ({ ...s, isLoading: true }))
		try {
			const albums = await albumApi.getPlayerAlbums()
			update((s) => ({ ...s, albums, isLoading: false }))
		} catch (e) {
			console.error('Failed to load player albums:', e)
			update((s) => ({ ...s, isLoading: false }))
		}
	}

	async function selectAlbum(album: PlayerAlbum | null) {
		if (!album) {
			update((s) => ({ ...s, selectedAlbum: null, selectedAlbumTracks: [] }))
			return
		}

		update((s) => ({ ...s, selectedAlbum: album, selectedAlbumTracks: [] }))
		try {
			const tracks = await albumApi.getPlayerAlbumTracks(album.id)
			update((s) => {
				if (s.selectedAlbum?.id === album.id) {
					return { ...s, selectedAlbumTracks: tracks }
				}
				return s
			})
		} catch (e) {
			console.error('Failed to load album tracks:', e)
			toastStore.error(get(translate)('player.toast.albumTracksLoadFailed'))
		}
	}

	async function addAlbumFromDialog() {
		try {
			const selected = await open({
				directory: true,
				multiple: false,
				title: get(translate)('player.albums.dialogTitle'),
			})

			if (!selected || typeof selected !== 'string') {
				return
			}

			update((s) => ({ ...s, isAdding: true }))
			toastStore.info(get(translate)('player.toast.albumScanning'))

			const result = await albumApi.addPlayerAlbum(selected)
			update((s) => {
				const existingIdx = s.albums.findIndex(
					(a) => a.id === result.album.id || a.folder_path === result.album.folder_path
				)
				let updatedAlbums: PlayerAlbum[]
				if (existingIdx >= 0) {
					updatedAlbums = [...s.albums]
					updatedAlbums[existingIdx] = result.album
				} else {
					updatedAlbums = [result.album, ...s.albums]
				}
				return {
					...s,
					albums: updatedAlbums,
					selectedAlbum: result.album,
					selectedAlbumTracks: result.tracks,
					isAdding: false,
				}
			})

			toastStore.success(
				get(translate)('player.toast.albumImported', {
					values: { title: result.album.title, count: result.tracks.length },
				})
			)
		} catch (e) {
			console.error('Failed to add album:', e)
			const errorMsg = toErrorMessage(e, get(translate)('player.toast.albumAddFailed'))
			toastStore.error(String(errorMsg))
			update((s) => ({ ...s, isAdding: false }))
		}
	}

	async function removeAlbum(albumId: string) {
		try {
			await albumApi.removePlayerAlbum(albumId)
			update((s) => {
				const updatedAlbums = s.albums.filter((a) => a.id !== albumId)
				const isCurrentSelected = s.selectedAlbum?.id === albumId
				return {
					...s,
					albums: updatedAlbums,
					selectedAlbum: isCurrentSelected ? null : s.selectedAlbum,
					selectedAlbumTracks: isCurrentSelected ? [] : s.selectedAlbumTracks,
				}
			})
			toastStore.info(get(translate)('player.toast.albumRemoved'))
		} catch (e) {
			console.error('Failed to remove album:', e)
			toastStore.error(get(translate)('player.toast.albumRemoveFailed'))
		}
	}

	function convertToStandaloneTrack(track: PlayerAlbumTrack, album: PlayerAlbum | null): StandaloneTrack {
		return {
			id: track.id,
			file_path: track.file_path,
			title: track.title,
			artist: track.artist,
			album: album?.title || null,
			duration_ms: track.duration_ms,
			format: track.format,
			bitrate: track.bitrate,
			sample_rate: track.sample_rate,
			bpm: track.bpm,
			key: track.key,
			energy: track.energy,
			artwork_path: track.artwork_path || album?.artwork_path || null,
			is_in_library: true, // Marked as library/album track so stats engine records listen event in Crate Pulse Stats!
			last_played_at: new Date().toISOString(),
		}
	}

	async function playTrackByIndex(index: number, tracks: PlayerAlbumTrack[], album: PlayerAlbum) {
		if (index < 0 || index >= tracks.length) return
		const track = tracks[index]
		const standalone = convertToStandaloneTrack(track, album)

		update((s) => ({
			...s,
			isPlayingAlbum: true,
			currentTrackIndex: index,
		}))

		await playerStore.playStandalone(standalone, true)
	}

	async function playNextAlbumTrack() {
		const state = get(albumsStore)
		if (!state.selectedAlbum || state.selectedAlbumTracks.length === 0) return
		const nextIndex = (state.currentTrackIndex + 1) % state.selectedAlbumTracks.length
		await playTrackByIndex(nextIndex, state.selectedAlbumTracks, state.selectedAlbum)
	}

	async function playPreviousAlbumTrack() {
		const state = get(albumsStore)
		if (!state.selectedAlbum || state.selectedAlbumTracks.length === 0) return
		const prevIndex =
			(state.currentTrackIndex - 1 + state.selectedAlbumTracks.length) % state.selectedAlbumTracks.length
		await playTrackByIndex(prevIndex, state.selectedAlbumTracks, state.selectedAlbum)
	}

	async function playAlbumTrack(track: PlayerAlbumTrack, albumTracks?: PlayerAlbumTrack[], album?: PlayerAlbum | null) {
		const state = get(albumsStore)
		const targetAlbum = album || state.selectedAlbum
		const tracks = albumTracks || state.selectedAlbumTracks

		const trackIndex = tracks.findIndex((t) => t.id === track.id || t.file_path === track.file_path)
		if (trackIndex >= 0 && targetAlbum) {
			await playTrackByIndex(trackIndex, tracks, targetAlbum)
		} else {
			const standalone = convertToStandaloneTrack(track, targetAlbum)
			await playerStore.playStandalone(standalone, true)
		}
	}

	async function playAlbum(album: PlayerAlbum, shuffle: boolean = false) {
		let tracks = get(albumsStore).selectedAlbumTracks
		if (get(albumsStore).selectedAlbum?.id !== album.id || tracks.length === 0) {
			tracks = await albumApi.getPlayerAlbumTracks(album.id)
			update((s) => ({
				...s,
				selectedAlbum: album,
				selectedAlbumTracks: tracks,
			}))
		}

		if (tracks.length === 0) {
			toastStore.error(get(translate)('player.toast.albumEmpty'))
			return
		}

		const startIndex = shuffle ? Math.floor(Math.random() * tracks.length) : 0
		await playTrackByIndex(startIndex, tracks, album)
	}

	return {
		subscribe,
		loadAlbums,
		selectAlbum,
		addAlbumFromDialog,
		removeAlbum,
		playAlbum,
		playAlbumTrack,
		playNextAlbumTrack,
		playPreviousAlbumTrack,
	}
}

export const albumsStore = createAlbumsStore()

export const playerAlbums = derived(albumsStore, ($s) => $s.albums)
export const selectedAlbum = derived(albumsStore, ($s) => $s.selectedAlbum)
export const selectedAlbumTracks = derived(albumsStore, ($s) => $s.selectedAlbumTracks)
export const albumsLoading = derived(albumsStore, ($s) => $s.isLoading)
export const albumsAdding = derived(albumsStore, ($s) => $s.isAdding)
