import type { Playlist, Tag, TagCategory } from '$shared/types'
import type { HandlerMap } from '../types'
import type { HarnessState } from '../state'
import { REFERENCE_NOW } from '../fixtures/reference'

// Creating, renaming and deleting playlists, folders, tags and categories, in memory: enough for a designer to
// click "New Playlist" or "New tag" and see the interface react. Nothing is validated beyond what the views need.

function requirePlaylist(state: HarnessState, id: unknown): Playlist {
	const playlist = state.playlists.find((entry) => entry.id === id)
	if (!playlist) throw `Playlist not found: ${String(id)}`
	return playlist
}

function requireCategory(state: HarnessState, id: unknown): TagCategory {
	const category = state.tagCategories.find((entry) => entry.id === id)
	if (!category) throw `Tag category not found: ${String(id)}`
	return category
}

export function organiseHandlers(state: HarnessState): HandlerMap {
	let counter = 0
	const nextId = (prefix: string) => `${prefix}-new-${++counter}`

	function addPlaylist(args: Record<string, unknown>, isFolder: boolean): Playlist {
		const playlist: Playlist = {
			id: nextId('pl'),
			name: String(args.name),
			parent_id: (args.parentId as string | null) ?? null,
			is_folder: isFolder,
			is_smart: false,
			smart_rules: null,
			sort_order: state.playlists.length,
			date_created: REFERENCE_NOW,
			date_modified: REFERENCE_NOW,
			track_count: 0,
			context: (args.context as Playlist['context'] | undefined) ?? 'library',
		}
		state.playlists.push(playlist)
		state.playlistTrackIds[playlist.id] = []
		return playlist
	}

	function setMembers(playlistId: unknown, trackIds: string[]): Playlist {
		const playlist = requirePlaylist(state, playlistId)
		state.playlistTrackIds[playlist.id] = trackIds
		playlist.track_count = trackIds.length
		return playlist
	}

	return {
		create_playlist: (args) => addPlaylist(args, false),
		create_folder: (args) => addPlaylist(args, true),
		rename_playlist: ({ id, name }) => Object.assign(requirePlaylist(state, id), { name: String(name) }),
		delete_playlist: ({ id }) => {
			const doomed = new Set<string>([String(id)])
			// Folders take their content with them, however deeply nested.
			for (let grew = true; grew; ) {
				grew = false
				for (const playlist of state.playlists) {
					if (playlist.parent_id && doomed.has(playlist.parent_id) && !doomed.has(playlist.id)) {
						doomed.add(playlist.id)
						grew = true
					}
				}
			}
			state.playlists = state.playlists.filter((playlist) => !doomed.has(playlist.id))
			return null
		},
		add_to_playlist: ({ playlistId, trackIds }) => {
			const current = state.playlistTrackIds[String(playlistId)] ?? []
			return setMembers(playlistId, [...new Set([...current, ...(trackIds as string[])])])
		},
		remove_from_playlist: ({ playlistId, trackIds }) => {
			const removed = new Set(trackIds as string[])
			const current = state.playlistTrackIds[String(playlistId)] ?? []
			return setMembers(
				playlistId,
				current.filter((id) => !removed.has(id))
			)
		},

		create_tag_category: ({ name, color }) => {
			const category: TagCategory = {
				id: nextId('cat'),
				name: String(name),
				color: (color as string | null) ?? null,
				sort_order: state.tagCategories.length,
				tags: [],
			}
			state.tagCategories.push(category)
			return category
		},
		update_tag_category: ({ id, name, color }) => {
			const category = requireCategory(state, id)
			if (typeof name === 'string') category.name = name
			if (typeof color === 'string') category.color = color
			return category
		},
		delete_tag_category: ({ id }) => {
			const category = requireCategory(state, id)
			const removed = new Set(category.tags.map((tag) => tag.id))
			state.tagCategories = state.tagCategories.filter((entry) => entry.id !== category.id)
			for (const track of state.tracks) track.tags = track.tags.filter((tag) => !removed.has(tag.id))
			return null
		},
		create_tag: ({ categoryId, name, color }) => {
			const category = requireCategory(state, categoryId)
			const tag: Tag = {
				id: nextId('tag'),
				category_id: category.id,
				name: String(name),
				color: (color as string | null) ?? category.color,
				sort_order: category.tags.length,
			}
			category.tags.push(tag)
			return tag
		},
		update_tag: ({ id, name, color }) => {
			const tag = state.tagCategories.flatMap((category) => category.tags).find((entry) => entry.id === id)
			if (!tag) throw `Tag not found: ${String(id)}`
			if (typeof name === 'string') tag.name = name
			if (typeof color === 'string') tag.color = color
			for (const track of state.tracks)
				track.tags = track.tags.map((entry) => (entry.id === tag.id ? { ...tag } : entry))
			return tag
		},
		delete_tag: ({ id }) => {
			for (const category of state.tagCategories) category.tags = category.tags.filter((tag) => tag.id !== id)
			for (const track of state.tracks) track.tags = track.tags.filter((tag) => tag.id !== id)
			return null
		},

		// The Mixed In Key button of the toolbar: "everything is up to date".
		sync_from_mik_database: () => ({
			added: 0,
			updated: state.tracks.length,
			removed: 0,
			total: state.tracks.length,
		}),
	}
}
