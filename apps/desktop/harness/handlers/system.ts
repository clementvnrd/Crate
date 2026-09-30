import type { AppSettings } from '$shared/types'
import type { HandlerMap } from '../types'
import type { HarnessState } from '../state'
import {
	AUDIO_DEVICES,
	BACKUP_INFO,
	CLOUD_SYNC_STATUS,
	DIAGNOSTICS_REPORT,
	DIAGNOSTIC_ENTRIES,
	SYSTEM_INFO,
	createAppInfo,
	mikStatus,
} from '../fixtures/system'

/** `key_notation_format` → `keyNotationFormat`, the name of the field in `AppSettings`. */
function toCamelCase(key: string): string {
	return key.replace(/_([a-z])/g, (_, letter: string) => letter.toUpperCase())
}

/** `set_setting` sends every value as a string; the backend stores it as text and parses it back on read. */
function parseSettingValue(key: string, value: string): unknown {
	if (value === 'true') return true
	if (value === 'false') return false
	if (key === 'ignored_device_ids') return JSON.parse(value) as string[]
	if (key === 'audio_device') return value === '' ? null : value
	return value
}

export function systemHandlers(state: HarnessState): HandlerMap {
	const appInfo = createAppInfo(state.params)
	let diagnosticEntries = structuredClone(DIAGNOSTIC_ENTRIES)
	return {
		get_app_info: () => appInfo,

		// Settings live in memory for the life of the page; appearance also round-trips through localStorage.
		get_settings: () => state.settings,
		set_setting: ({ key, value }) => {
			const field = toCamelCase(String(key)) as keyof AppSettings
			;(state.settings as unknown as Record<string, unknown>)[field] = parseSettingValue(String(key), String(value))
			return null
		},
		get_audio_devices: () => AUDIO_DEVICES,
		set_audio_device: ({ deviceName }) => {
			state.settings.audioDevice = (deviceName as string | null) ?? null
			return null
		},

		get_diagnostic_entries: () => diagnosticEntries,
		get_system_info: () => SYSTEM_INFO,
		get_diagnostics_report: () => ({ ...DIAGNOSTICS_REPORT, entries: diagnosticEntries }),
		clear_diagnostic_entries: () => {
			diagnosticEntries = []
			return null
		},

		get_mik_database_status: () => mikStatus(state.params.libraryEmpty),

		// Cloud sync: signed out, no other device.
		get_sync_status: () => CLOUD_SYNC_STATUS,
		list_devices: () => [],
		list_library_roots: () => [],
		suggest_library_roots: () => [],

		get_backup_info: () => BACKUP_INFO,
	}
}
