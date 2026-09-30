import type { HandlerMap } from '../types'

// Commands whose only job is a native side effect (menu, Now Playing widget, developer tools, logging…) or whose
// answer is a constant in every scenario. Listing them here keeps `window.__harness.unmocked` meaningful: it only
// names commands nobody has decided an answer for.

const VOID_COMMANDS = [
	'rebuild_menu',
	'set_menu_item_enabled',
	'set_onboarding_items_enabled',
	'set_dialog_conflicting_items_enabled',
	'update_now_playing',
	'update_playback_state',
	'clear_now_playing',
	'open_dev_tools',
	'close_dev_tools',
	'log_error',
	'cancel_analysis',
	'cancel_export',
	'cancel_sync',
	'cancel_scan_page',
	'cancel_bulk_import',
	'ignore_duplicate_group',
	'unignore_duplicate_group',
	'ignore_upgrade_match',
	'unignore_upgrade_match',
	'set_spotify_client_id',
	'set_spotify_client_secret',
	'mik_tracker_set_enabled',
]

const EMPTY_LIST_COMMANDS = [
	// Nothing to show in any harness scenario.
	'get_playlist_releases',
	'get_smart_playlist_releases',
	'get_playlists_containing_track',
	'get_playlists_containing_tracks',
	'get_devices_for_playlist',
	'get_devices_for_playlists',
	'get_device_exports',
	'get_pending_sync_playlists',
]

export function noopHandlers(): HandlerMap {
	const handlers: HandlerMap = {}
	for (const command of VOID_COMMANDS) handlers[command] = () => null
	for (const command of EMPTY_LIST_COMMANDS) handlers[command] = () => []
	handlers.get_pending_checkpoint = () => null
	handlers.has_pending_sync_changes = () => false
	handlers.is_syncing = () => false
	return handlers
}
