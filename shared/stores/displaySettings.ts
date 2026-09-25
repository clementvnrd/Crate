import { writable } from 'svelte/store'

export interface ColumnVisibility {
	color: boolean
	artwork: boolean
	title: boolean
	artist: boolean
	album: boolean
	bpm: boolean
	key: boolean
	energy: boolean
	format: boolean
	bitrate: boolean
	duration_ms: boolean
	tags: boolean
	date_added: boolean
	rating: boolean
	file_path: boolean
	file_size: boolean
	sample_rate: boolean
	genre: boolean
	label: boolean
	year: boolean
}

export interface DisplaySettings {
	showAudioImprovementMessage: boolean
	showDetectedCuePoints: boolean
	showDetectedEnergyLevels: boolean
	camelotZeroPadding: boolean
	columns: ColumnVisibility
}

const STORAGE_KEY = 'crate-display-options'

const defaultSettings: DisplaySettings = {
	showAudioImprovementMessage: false,
	showDetectedCuePoints: true,
	showDetectedEnergyLevels: true,
	camelotZeroPadding: false,
	columns: {
		color: true,
		artwork: true,
		title: true,
		artist: true,
		album: false,
		bpm: true,
		key: true,
		energy: true,
		format: true,
		bitrate: true,
		duration_ms: true,
		tags: true,
		date_added: false,
		rating: false,
		file_path: false,
		file_size: false,
		sample_rate: false,
		genre: false,
		label: false,
		year: false,
	},
}

function loadInitialSettings(): DisplaySettings {
	if (typeof window === 'undefined') return defaultSettings
	try {
		const stored = localStorage.getItem(STORAGE_KEY)
		if (stored) {
			const parsed = JSON.parse(stored)
			return {
				...defaultSettings,
				...parsed,
				columns: {
					...defaultSettings.columns,
					...(parsed.columns || {}),
				},
			}
		}
	} catch (e) {
		console.warn('Failed to load display settings from localStorage:', e)
	}
	return defaultSettings
}

function createDisplaySettingsStore() {
	const { subscribe, set, update } = writable<DisplaySettings>(loadInitialSettings())

	function save(settings: DisplaySettings) {
		if (typeof window !== 'undefined') {
			try {
				localStorage.setItem(STORAGE_KEY, JSON.stringify(settings))
			} catch (e) {
				console.warn('Failed to save display settings to localStorage:', e)
			}
		}
	}

	return {
		subscribe,
		set(value: DisplaySettings) {
			save(value)
			set(value)
		},
		toggleColumn(columnKey: keyof ColumnVisibility) {
			update((state) => {
				const next = {
					...state,
					columns: {
						...state.columns,
						[columnKey]: !state.columns[columnKey],
					},
				}
				save(next)
				return next
			})
		},
		setColumn(columnKey: keyof ColumnVisibility, visible: boolean) {
			update((state) => {
				const next = {
					...state,
					columns: {
						...state.columns,
						[columnKey]: visible,
					},
				}
				save(next)
				return next
			})
		},
		setCamelotZeroPadding(enabled: boolean) {
			update((state) => {
				const next = { ...state, camelotZeroPadding: enabled }
				save(next)
				return next
			})
		},
		setShowAudioImprovementMessage(enabled: boolean) {
			update((state) => {
				const next = { ...state, showAudioImprovementMessage: enabled }
				save(next)
				return next
			})
		},
		setShowDetectedCuePoints(enabled: boolean) {
			update((state) => {
				const next = { ...state, showDetectedCuePoints: enabled }
				save(next)
				return next
			})
		},
		setShowDetectedEnergyLevels(enabled: boolean) {
			update((state) => {
				const next = { ...state, showDetectedEnergyLevels: enabled }
				save(next)
				return next
			})
		},
		resetToDefaults() {
			save(defaultSettings)
			set(defaultSettings)
		},
	}
}

export const displaySettingsStore = createDisplaySettingsStore()
