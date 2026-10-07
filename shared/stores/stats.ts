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
	SpotifyResetResult,
	StatsSummary,
	TimeRange,
	TopArtistItem,
	TopTrackItem,
} from '../types'
import * as statsApi from '../api/stats'
import { toastStore } from './toast'
import { openUrl } from '@tauri-apps/plugin-opener'
import { toErrorMessage } from '../utils/errors'
import { formatNumber } from '../utils/format'
import { locale, translate } from '../i18n'

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
	isResettingSpotifyHistory: boolean
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
	isResettingSpotifyHistory: false,
	error: null,
}

function createStatsStore() {
	const { subscribe, set, update } = writable<StatsState>(initialState)

	// Every load takes the next number; a response only lands if its number is still the latest, so a
	// slow answer for an earlier period can never overwrite the one the owner is looking at now.
	let loadSeq = 0
	let isSyncingSpotify = false

	/**
	 * Loads every statistic for the selected period from the local database. Makes no network call:
	 * this is what a period change runs.
	 */
	async function reload(): Promise<void> {
		const token = ++loadSeq
		const currentRange = get({ subscribe }).selectedRange
		update((s) => ({ ...s, isLoading: true, error: null }))

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

		// A newer load (or a reset) started while this one was waiting: drop this answer, and leave
		// the spinner to the load that is still running.
		if (token !== loadSeq) return

		// The period-scoped answers describe the period just asked for. When one fails, keeping the
		// previous period's numbers under the new label would be wrong, so they are emptied instead
		// and the failure is reported.
		const scoped = [summaryRes, topTracksRes, topArtistsRes, harmonicRes, bpmRes, heatmapRes]
		const failed = scoped.find((r): r is PromiseRejectedResult => r.status === 'rejected')
		const errorMsg = failed ? toErrorMessage(failed.reason, get(translate)('stats.toast.loadFailed')) : null

		update((s) => ({
			...s,
			isLoading: false,
			error: errorMsg,
			summary: summaryRes.status === 'fulfilled' ? summaryRes.value : null,
			topTracks: topTracksRes.status === 'fulfilled' ? topTracksRes.value : [],
			topArtists: topArtistsRes.status === 'fulfilled' ? topArtistsRes.value : [],
			harmonicStats: harmonicRes.status === 'fulfilled' ? harmonicRes.value : [],
			bpmStats: bpmRes.status === 'fulfilled' ? bpmRes.value : [],
			heatmap: heatmapRes.status === 'fulfilled' ? heatmapRes.value : [],
			// Not tied to the period: a failure keeps what was already shown.
			recentListens: recentRes.status === 'fulfilled' ? recentRes.value : s.recentListens,
			spotifyAuth: spotifyAuthRes.status === 'fulfilled' ? spotifyAuthRes.value : s.spotifyAuth,
			spotifyNowPlaying: spotifyNowRes.status === 'fulfilled' ? spotifyNowRes.value : s.spotifyNowPlaying,
			rekordboxDetected: rekordboxDetectedRes.status === 'fulfilled' ? rekordboxDetectedRes.value : s.rekordboxDetected,
			rekordboxSessions: rekordboxSessionsRes.status === 'fulfilled' ? rekordboxSessionsRes.value : s.rekordboxSessions,
			mikDetected: mikDetectedRes.status === 'fulfilled' ? mikDetectedRes.value : s.mikDetected,
		}))
		if (errorMsg) toastStore.error(errorMsg)
	}

	/**
	 * Pulls the recently played tracks from Spotify (a network call) when it is connected. Resolves to
	 * how many listens were added. Never throws: a Spotify hiccup must not break the local numbers.
	 */
	async function syncSpotify(): Promise<number> {
		if (isSyncingSpotify || !get({ subscribe }).spotifyAuth?.is_connected) return 0
		isSyncingSpotify = true
		try {
			return await statsApi.syncSpotifyRecentlyPlayed()
		} catch (e) {
			console.debug('Spotify recently played sync error:', e)
			return 0
		} finally {
			isSyncingSpotify = false
		}
	}

	return {
		subscribe,

		/**
		 * Set active time range and reload the analytics. A pure period change reads the local
		 * database only: it makes no Spotify call (that belongs to `refreshAll`).
		 */
		async setRange(range: TimeRange) {
			update((s) => ({ ...s, selectedRange: range }))
			await reload()
		},

		/**
		 * Reload the statistics for the current period without any network call. Used after a local
		 * change (import, set sync, history reset) that only needs the numbers redrawn.
		 */
		reload,

		/**
		 * Full refresh (first load, refresh button, new Spotify connection): load the local numbers
		 * at once, then pull Spotify's recently played tracks and reload if that added any listens.
		 */
		async refreshAll() {
			await reload()
			if ((await syncSpotify()) > 0) await reload()
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
					toastStore.info(get(translate)('stats.toast.authorizeInBrowser'))
				}
			} catch (err) {
				console.error('Spotify connect error:', err)
				toastStore.error(get(translate)('stats.toast.connectFailed'))
			}
		},

		/**
		 * Handle Spotify OAuth redirect authorization code
		 */
		async handleSpotifyCallback(code: string, clientId?: string, redirectUri?: string, state?: string) {
			try {
				const authState = await statsApi.handleSpotifyCallback(code, clientId, redirectUri, state)
				update((s) => ({ ...s, spotifyAuth: authState }))
				toastStore.success(get(translate)('stats.toast.spotifyConnected'))
				await this.refreshAll()
				return authState
			} catch (err) {
				console.error('Spotify auth callback error:', err)
				toastStore.error(get(translate)('stats.toast.authFailed'))
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
				toastStore.success(get(translate)('stats.toast.spotifyDisconnected'))
			} catch (err) {
				console.error('Spotify disconnect error:', err)
				toastStore.error(get(translate)('stats.toast.disconnectFailed'))
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
				toastStore.success(
					get(translate)('stats.toast.spotifyImported', {
						values: {
							count: result.imported_count,
							minutes: formatNumber(result.total_minutes, get(locale) ?? undefined),
						},
					})
				)
				await reload()
				return result
			} catch (err) {
				const errorMsg = toErrorMessage(err, get(translate)('stats.toast.importFailed'))
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
				toastStore.success(get(translate)('stats.toast.rekordboxSynced', { values: { count } }))
				await reload()
				return count
			} catch (err) {
				const errorMsg = toErrorMessage(err, get(translate)('stats.toast.rekordboxFailed'))
				update((s) => ({ ...s, isSyncingRekordbox: false }))
				toastStore.error(errorMsg)
				return null
			}
		},

		/**
		 * Deletes every Spotify-sourced listen, after a safety backup of the whole history. The
		 * caller is responsible for confirming with the owner first; this only runs the deletion
		 * and reports the result. The reload that follows makes no Spotify call, so the listens
		 * just deleted are not pulled straight back by a "recently played" sync.
		 */
		async resetSpotifyHistory(): Promise<SpotifyResetResult | null> {
			update((s) => ({ ...s, isResettingSpotifyHistory: true }))
			try {
				const result = await statsApi.resetSpotifyListeningHistory()
				update((s) => ({ ...s, isResettingSpotifyHistory: false }))
				toastStore.success(
					get(translate)('stats.toast.spotifyHistoryReset', { values: { count: result.deleted_count } })
				)
				await reload()
				return result
			} catch (err) {
				const errorMsg = toErrorMessage(err, get(translate)('stats.toast.spotifyHistoryResetFailed'))
				update((s) => ({ ...s, isResettingSpotifyHistory: false }))
				toastStore.error(errorMsg)
				return null
			}
		},

		/**
		 * Reset store. Answers still in flight are discarded.
		 */
		reset() {
			loadSeq += 1
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
export const isResettingSpotifyHistory = derived(statsStore, ($s) => $s.isResettingSpotifyHistory)
