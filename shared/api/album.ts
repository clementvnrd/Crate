import { invoke } from '@tauri-apps/api/core'
import type { PlayerAlbum, PlayerAlbumTrack, AddAlbumResult } from '../types'

/**
 * Add an album by scanning a folder on the filesystem.
 */
export async function addPlayerAlbum(folderPath: string): Promise<AddAlbumResult> {
	return invoke<AddAlbumResult>('add_player_album', { folderPath })
}

/**
 * Get all imported player albums.
 */
export async function getPlayerAlbums(): Promise<PlayerAlbum[]> {
	return invoke<PlayerAlbum[]>('get_player_albums')
}

/**
 * Get all tracks for a specific player album.
 */
export async function getPlayerAlbumTracks(albumId: string): Promise<PlayerAlbumTrack[]> {
	return invoke<PlayerAlbumTrack[]>('get_player_album_tracks', { albumId })
}

/**
 * Remove an album and its tracks from Crate Player.
 */
export async function removePlayerAlbum(albumId: string): Promise<void> {
	return invoke<void>('remove_player_album', { albumId })
}
