import { writable, derived, get } from 'svelte/store'
import type {
	BpmBucketItem,
	HarmonicStatsItem,
	HeatmapCell,
	ListenEvent,
	RekordboxSession,
	SpotifyAuthState,
	SpotifyImportResult,
	SpotifyNowPlaying,
	StatsSummary,
	TimeRange,
	TopArtistItem,
	TopTrackItem,
} from '../types'
import * as statsApi from '../api/stats'
import { toastStore } from './toast'
import { openUrl } from '@tauri-apps/plugin-opener'
import { toErrorMessage } from '../utils/errors'

export interface StatsState {
	summary: StatsSummary | null
	topTracks: TopTrackItem[]
	topArtists: TopArtistItem[]
	harmonicStats: HarmonicStatsItem[]
	bpmStats: BpmBucketItem[]
	heatmap: HeatmapCell[]
	recentListens: ListenEvent[]
	spotifyAuth: SpotifyAuthState | null
	spotifyNowPlaying: SpotifyNowPlaying | null
	rekordboxDetected: boolean
	rekordboxSessions: RekordboxSession[]
	mikDetected: boolean
	selectedRange: TimeRange
	isLoading: boolean
	isSyncingRekordbox: boolean
	isImportingSpotify: boolean
	error: string | null
}

const initialState: StatsState = {
	summary: null,
	topTracks: [],
	topArtists: [],
	harmonicStats: [],
	bpmStats: [],
	heatmap: [],
	recentListens: [],
	spotifyAuth: null,
	spotifyNowPlaying: null,
	rekordboxDetected: false,
	rekordboxSessions: [],
	mikDetected: false,
	selectedRange: '7d',
	isLoading: false,
	isSyncingRekordbox: false,
	isImportingSpotify: false,
	error: null,
}

function createStatsStore() {
	const { subscribe, set, update } = writable<StatsState>(initialState)

	return {
		subscribe,

		/**
		 * Set active time range and refresh all analytics
		 */
		async setRange(range: TimeRange) {
			update((s) => ({ ...s, selectedRange: range }))
			await this.refreshAll()
		},

		/**
		 * Refresh all statistics and data from the backend
		 */
		async refreshAll() {
			const currentRange = get(statsStore).selectedRange
			update((s) => ({ ...s, isLoading: true, error: null }))

			// Sync any offline/recent Spotify plays if connected
			if (get(statsStore).spotifyAuth?.is_connected) {
				try {
					await statsApi.syncSpotifyRecentlyPlayed()
				} catch (e) {
					console.debug('Spotify recently played sync error:', e)
				}
			}

			try {
				const [
					summaryRes,
					topTracksRes,
					topArtistsRes,
					harmonicRes,
					bpmRes,
					heatmapRes,
					recentRes,
					spotifyAuthRes,
					spotifyNowRes,
					rekordboxDetectedRes,
					rekordboxSessionsRes,
					mikDetectedRes,
				] = await Promise.allSettled([
					statsApi.getStatsSummary(currentRange),
					statsApi.getTopTracks(currentRange, 50),
					statsApi.getTopArtists(currentRange, 50),
					statsApi.getHarmonicStats(currentRange),
					statsApi.getBpmStats(currentRange),
					statsApi.getListeningHeatmap(currentRange),
					statsApi.getRecentListens(50),
					statsApi.getSpotifyAuthState(),
					statsApi.getSpotifyNowPlaying(),
					statsApi.getRekordboxDetectStatus(),
					statsApi.getRekordboxSessions(),
					statsApi.getMikDetectStatus(),
				])

				update((s) => ({
					...s,
					isLoading: false,
					summary: summaryRes.status === 'fulfilled' ? summaryRes.value : s.summary,
					topTracks: topTracksRes.status === 'fulfilled' ? topTracksRes.value : s.topTracks,
					topArtists: topArtistsRes.status === 'fulfilled' ? topArtistsRes.value : s.topArtists,
					harmonicStats: harmonicRes.status === 'fulfilled' ? harmonicRes.value : s.harmonicStats,
					bpmStats: bpmRes.status === 'fulfilled' ? bpmRes.value : s.bpmStats,
					heatmap: heatmapRes.status === 'fulfilled' ? heatmapRes.value : s.heatmap,
					recentListens: recentRes.status === 'fulfilled' ? recentRes.value : s.recentListens,
					spotifyAuth: spotifyAuthRes.status === 'fulfilled' ? spotifyAuthRes.value : s.spotifyAuth,
					spotifyNowPlaying: spotifyNowRes.status === 'fulfilled' ? spotifyNowRes.value : s.spotifyNowPlaying,
					rekordboxDetected:
						rekordboxDetectedRes.status === 'fulfilled' ? rekordboxDetectedRes.value : s.rekordboxDetected,
					rekordboxSessions:
						rekordboxSessionsRes.status === 'fulfilled' ? rekordboxSessionsRes.value : s.rekordboxSessions,
					mikDetected: mikDetectedRes.status === 'fulfilled' ? mikDetectedRes.value : s.mikDetected,
				}))
			} catch (err) {
				const errorMsg = toErrorMessage(err, 'Erreur lors du chargement des statistiques')
				update((s) => ({ ...s, isLoading: false, error: errorMsg }))
				toastStore.error(errorMsg)
			}
		},

		/**
		 * Refresh only the currently playing Spotify track
		 */
		async refreshNowPlaying() {
			try {
				const now = await statsApi.getSpotifyNowPlaying()
				update((s) => ({ ...s, spotifyNowPlaying: now }))
				return now
			} catch {
				return null
			}
		},

		/**
		 * Connect to Spotify via OAuth
		 */
		async connectSpotify(clientId?: string, clientSecret?: string, redirectUri?: string) {
			try {
				if (clientId && clientId.trim().length > 0) {
					await statsApi.setSpotifyClientId(clientId.trim())
				}
				// An empty field keeps the stored secret (it is never sent back to the UI to prefill).
				if (clientSecret && clientSecret.trim().length > 0) {
					await statsApi.setSpotifyClientSecret(clientSecret.trim())
				}
				const authUrl = await statsApi.getSpotifyAuthUrl(clientId, redirectUri)
				if (authUrl) {
					await openUrl(authUrl)
					toastStore.info('Autorisez Crate sur Spotify dans votre navigateur...')
				}
			} catch (err) {
				console.error('Spotify connect error:', err)
				toastStore.error("Impossible d'initialiser la connexion Spotify")
			}
		},

		/**
		 * Handle Spotify OAuth redirect authorization code
		 */
		async handleSpotifyCallback(code: string, clientId?: string, redirectUri?: string, state?: string) {
			try {
				const authState = await statsApi.handleSpotifyCallback(code, clientId, redirectUri, state)
				update((s) => ({ ...s, spotifyAuth: authState }))
				toastStore.success('Compte Spotify connecté avec succès !')
				await this.refreshAll()
				return authState
			} catch (err) {
				console.error('Spotify auth callback error:', err)
				toastStore.error("Erreur d'authentification avec Spotify")
				return null
			}
		},

		/**
		 * Disconnect Spotify integration
		 */
		async disconnectSpotify() {
			try {
				await statsApi.disconnectSpotify()
				update((s) => ({
					...s,
					spotifyAuth: { is_connected: false, user_id: null, user_name: null, expires_at: null },
					spotifyNowPlaying: null,
				}))
				toastStore.success('Spotify déconnecté')
			} catch (err) {
				console.error('Spotify disconnect error:', err)
				toastStore.error('Erreur lors de la déconnexion de Spotify')
			}
		},

		/**
		 * Import Spotify history from streaming history JSON string
		 */
		async importSpotifyJson(jsonContent: string): Promise<SpotifyImportResult | null> {
			update((s) => ({ ...s, isImportingSpotify: true }))
			try {
				const result = await statsApi.importSpotifyHistoryJson(jsonContent)
				update((s) => ({ ...s, isImportingSpotify: false }))
				toastStore.success(`Historique Spotify importé : ${result.imported_count} titres (${result.total_minutes} min)`)
				await this.refreshAll()
				return result
			} catch (err) {
				const errorMsg = toErrorMessage(err, "Erreur lors de l'import de l'archive Spotify")
				update((s) => ({ ...s, isImportingSpotify: false }))
				toastStore.error(errorMsg)
				return null
			}
		},

		/**
		 * Synchronize local Rekordbox listening and DJ history
		 */
		async syncRekordbox(): Promise<number | null> {
			update((s) => ({ ...s, isSyncingRekordbox: true }))
			try {
				const count = await statsApi.syncRekordboxHistory()
				update((s) => ({ ...s, isSyncingRekordbox: false }))
				toastStore.success(`Rekordbox synchronisé : ${count} écoutes enregistrées`)
				await this.refreshAll()
				return count
			} catch (err) {
				const errorMsg = toErrorMessage(err, 'Erreur lors de la synchronisation avec Rekordbox')
				update((s) => ({ ...s, isSyncingRekordbox: false }))
				toastStore.error(errorMsg)
				return null
			}
		},

		/**
		 * Reset store
		 */
		reset() {
			set(initialState)
		},
	}
}

export const statsStore = createStatsStore()

// =============================================================================
// Derived Stores
// =============================================================================

export const statsSummary = derived(statsStore, ($s) => $s.summary)
export const topTracks = derived(statsStore, ($s) => $s.topTracks)
export const topArtists = derived(statsStore, ($s) => $s.topArtists)
export const harmonicStats = derived(statsStore, ($s) => $s.harmonicStats)
export const bpmStats = derived(statsStore, ($s) => $s.bpmStats)
export const listeningHeatmap = derived(statsStore, ($s) => $s.heatmap)
export const recentListens = derived(statsStore, ($s) => $s.recentListens)
export const spotifyAuth = derived(statsStore, ($s) => $s.spotifyAuth)
export const spotifyNowPlaying = derived(statsStore, ($s) => $s.spotifyNowPlaying)
export const rekordboxDetected = derived(statsStore, ($s) => $s.rekordboxDetected)
export const rekordboxSessions = derived(statsStore, ($s) => $s.rekordboxSessions)
export const mikDetected = derived(statsStore, ($s) => $s.mikDetected)
export const statsSelectedRange = derived(statsStore, ($s) => $s.selectedRange)
export const isStatsLoading = derived(statsStore, ($s) => $s.isLoading)
export const isSyncingRekordbox = derived(statsStore, ($s) => $s.isSyncingRekordbox)
export const isImportingSpotify = derived(statsStore, ($s) => $s.isImportingSpotify)
