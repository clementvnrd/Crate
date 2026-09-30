import type {
	AccentColor,
	AppSettings,
	AudioDevice,
	CloudSyncStatus,
	DiagnosticEntry,
	DiagnosticsReport,
	Font,
	Language,
	MikDatabaseStatus,
	SystemInfo,
	Theme,
} from '$shared/types'
import type { AppInfo } from '$shared/api/app'
import type { BackupInfo } from '$shared/api/backup'
import type { HarnessParams } from '../types'
import { REFERENCE_NOW, isoAgo } from './reference'
import { TRACKS } from './library'

const THEMES: Theme[] = ['light', 'dark', 'system']
const ACCENTS: AccentColor[] = [
	'blue',
	'indigo',
	'violet',
	'purple',
	'pink',
	'rose',
	'orange',
	'amber',
	'emerald',
	'teal',
]
const FONTS: Font[] = [
	'jost',
	'dm-sans',
	'inter',
	'nunito',
	'open-sans',
	'fira-code',
	'ibm-plex-mono',
	'source-code-pro',
]
const LANGUAGES: Language[] = ['en', 'ja', 'nl', 'fr', 'de', 'es', 'it', 'sv', 'ko', 'pt', 'zh', 'uk', 'ro', 'pl', 'tr']

function stored(key: string): string | null {
	try {
		return window.localStorage.getItem(key)
	} catch {
		return null
	}
}

function pick<T extends string>(value: string | null, allowed: T[], fallback: T): T {
	return allowed.includes(value as T) ? (value as T) : fallback
}

/**
 * Initial settings. Appearance comes from the localStorage keys the app itself writes (`crate-theme`,
 * `crate-accent`, `crate-language`, `crate-font`), so `?theme=light&lang=fr` and a reload both work; the
 * default theme is dark (not "system") so a run never depends on the machine's appearance.
 */
export function createSettings(params: HarnessParams): AppSettings {
	return {
		theme: pick(stored('crate-theme'), THEMES, 'dark'),
		accentColor: pick(stored('crate-accent'), ACCENTS, 'blue'),
		font: pick(stored('crate-font'), FONTS, 'open-sans'),
		audioDevice: null,
		language: pick(stored('crate-language'), LANGUAGES, 'en'),
		keyNotationFormat: 'camelot',
		dateFormat: 'locale',
		exportFormat: 'pdb',
		autoAnalyzeOnImport: true,
		autoSyncOnConnect: false,
		autoSyncOnChange: false,
		continuousPlayback: true,
		autoFetchMetadata: true,
		transferTagsOnImport: true,
		removeReleaseAfterImport: true,
		followCheckCadence: 'daily',
		autoFollowOnImport: 'off',
		releaseDayReminders: true,
		newReleasesSummary: true,
		ignoredDeviceIds: [],
		lastBackupAt: isoAgo(9),
		backupFrequency: 'monthly',
		lastBackupType: 'automatic',
		hasCompletedOnboarding: !params.onboarding,
		hasCompletedWizard: true,
		beatportDownloadDestination: '~/Music/My Library/FLAC',
		beatportAudioQuality: 'flac',
		beatportAutoSyncMik: true,
		displayOptions: null,
	}
}

export function createAppInfo(params: HarnessParams): AppInfo {
	return {
		version: '0.0.0-harness',
		environment: params.dev ? 'development' : 'production',
		isDev: params.dev,
		dataDir: '/harness/data',
	}
}

export const AUDIO_DEVICES: AudioDevice[] = [
	{ name: 'MacBook Pro Speakers', isDefault: true, isBuiltIn: true },
	{ name: 'Pioneer DJ XDJ-RX3', isDefault: false, isBuiltIn: false },
	{ name: 'Focusrite Scarlett 2i2 USB', isDefault: false, isBuiltIn: false },
]

export function mikStatus(libraryEmpty: boolean): MikDatabaseStatus {
	return {
		is_connected: true,
		db_path: '/harness/mixedinkey/Collection11.mikdb',
		total_songs: libraryEmpty ? 0 : TRACKS.length,
		total_cues: libraryEmpty ? 0 : TRACKS.length * 6,
		total_synced_in_crate: libraryEmpty ? 0 : TRACKS.length,
		last_sync_time: isoAgo(0, 2),
	}
}

export const CLOUD_SYNC_STATUS: CloudSyncStatus = {
	phase: 'signedout',
	email: null,
	display_name: null,
	photo_url: null,
	device_id: 'harness-device',
	device_name: 'Harness MacBook',
	last_error: null,
	last_synced_at: null,
	onboarding: null,
}

export const SYSTEM_INFO: SystemInfo = {
	osName: 'macOS',
	osVersion: '15.0',
	cpuBrand: 'Apple M3 Pro',
	cpuCores: 12,
	totalMemoryBytes: 38_654_705_664,
	usedMemoryBytes: 21_474_836_480,
	dataDirSizeBytes: 412_000_000,
	databaseSizeBytes: 18_500_000,
}

export const DIAGNOSTIC_ENTRIES: DiagnosticEntry[] = [
	{
		id: 'diag-1',
		timestamp: isoAgo(1, 4),
		level: 'warning',
		category: 'analysis',
		message: 'Could not read the BPM of untitled_export_final_v3.wav',
		details: 'No rhythmic content detected in the first 30 seconds.',
	},
	{
		id: 'diag-2',
		timestamp: isoAgo(4, 2),
		level: 'error',
		category: 'sync',
		message: 'Mixed In Key database was locked, retrying in 30 seconds',
		details: null,
	},
]

export const DIAGNOSTICS_REPORT: DiagnosticsReport = {
	appVersion: '0.0.0-harness',
	environment: 'production',
	generatedAt: REFERENCE_NOW,
	systemInfo: SYSTEM_INFO,
	entries: DIAGNOSTIC_ENTRIES,
}

export const BACKUP_INFO: BackupInfo = {
	version: 1,
	app_version: '0.0.0-harness',
	created_at: isoAgo(9),
	counts: {
		tracks: TRACKS.length,
		cues: TRACKS.length * 6,
		tag_categories: 3,
		tags: 11,
		playlists: 9,
		discovery_releases: 0,
		artwork_files: 12,
	},
}
