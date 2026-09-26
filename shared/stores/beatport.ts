import { writable, derived, get } from 'svelte/store'
import type {
	BeatportTrack,
	BeatportChart,
	BeatportGenre,
	BeatportPlaylist,
	BeatportAuthState,
	BeatportArtist,
	BeatportArtistDetail,
} from '../types/beatport'
import * as beatportApi from '../api/beatport'
import { toastStore } from './toast'
import { settingsStore } from './settings'
import { openUrl } from '@tauri-apps/plugin-opener'
import { playerStore } from './player'
import { toErrorMessage } from '../utils/errors'

export type BeatportNavSection = 'home' | 'purchased' | 'offline' | 'favorites' | 'playlist' | 'chart' | 'artist'

interface BeatportState {
	auth: BeatportAuthState
	navSection: BeatportNavSection
	selectedPlaylistId: string | null
	selectedPlaylistName: string | null
	selectedChartId: string | null
	selectedChartTitle: string | null
	selectedGenreId: number | null
	selectedGenreSlug: string | null
	selectedGenreName: string | null
	selectedArtistId: number | null
	selectedArtistName: string | null
	selectedArtistImage: string | null
	selectedArtistBio: string | null
	selectedArtistDetail: BeatportArtistDetail | null
	charts: BeatportChart[]
	genres: BeatportGenre[]
	topTracks: BeatportTrack[]
	currentSectionTracks: BeatportTrack[]
	searchResults: BeatportTrack[]
	searchArtists: BeatportArtist[]
	searchQuery: string
	searchType: 'tracks' | 'releases' | 'artists' | 'playlists'
	cart: BeatportTrack[]
	favorites: BeatportTrack[]
	purchases: BeatportTrack[]
	userPlaylists: BeatportPlaylist[]
	currentlyPlayingTrack: BeatportTrack | null
	isAudioPlaying: boolean
	audioCurrentTime: number
	audioDuration: number
	isDownloading: boolean
	downloadProgressText: string | null
	downloadDestination: string | null
	beatportdlPath: string | null
	showLoginModal: boolean
	loading: boolean
	error: string | null
}

const CART_STORAGE_KEY = 'crate-beatport-cart'
const LEGACY_AUTH_STORAGE_KEY = 'crate-beatport-auth'

function loadStoredCart(): BeatportTrack[] {
	if (typeof window === 'undefined') return []
	try {
		const stored = localStorage.getItem(CART_STORAGE_KEY)
		if (stored) return JSON.parse(stored)
	} catch (e) {
		console.warn('Failed to load cart from storage:', e)
	}
	return []
}

function defaultAuth(): BeatportAuthState {
	return {
		is_authenticated: false,
		username: null,
		token: null,
		refresh_token: null,
		has_subscription: false,
		subscription_tier: null,
	}
}

// Tokens are never kept in localStorage: the backend stores the session in the macOS Keychain.
// Remove the copy written by earlier builds.
function purgeLegacyStoredAuth() {
	if (typeof window === 'undefined') return
	try {
		localStorage.removeItem(LEGACY_AUTH_STORAGE_KEY)
	} catch {
		// Storage unavailable: nothing to purge
	}
}

function isJwtExpired(token: string | null | undefined): boolean {
	if (!token) return true
	try {
		const parts = token.split('.')
		if (parts.length < 2) return false
		const payloadJson = atob(parts[1].replace(/-/g, '+').replace(/_/g, '/'))
		const payload = JSON.parse(payloadJson)
		if (payload.exp) {
			// If expires in less than 90 seconds (or already expired), consider expired
			return Date.now() >= payload.exp * 1000 - 90000
		}
	} catch {
		// Ignore parsing error
	}
	return false
}

const initialAuth = defaultAuth()

const initialState: BeatportState = {
	auth: initialAuth,
	navSection: 'home',
	selectedPlaylistId: null,
	selectedPlaylistName: null,
	selectedChartId: null,
	selectedChartTitle: null,
	selectedGenreId: null,
	selectedGenreSlug: null,
	selectedGenreName: null,
	selectedArtistId: null,
	selectedArtistName: null,
	selectedArtistImage: null,
	selectedArtistBio: null,
	selectedArtistDetail: null,
	charts: [],
	genres: [],
	topTracks: [],
	currentSectionTracks: [],
	searchResults: [],
	searchArtists: [],
	searchQuery: '',
	searchType: 'tracks',
	cart: loadStoredCart(),
	favorites: [],
	purchases: [],
	userPlaylists: [],
	currentlyPlayingTrack: null,
	isAudioPlaying: false,
	audioCurrentTime: 0,
	audioDuration: 0,
	isDownloading: false,
	downloadProgressText: null,
	downloadDestination: null,
	beatportdlPath: null,
	showLoginModal: false,
	loading: false,
	error: null,
}

let refreshTimer: ReturnType<typeof setInterval> | null = null

function setupAutoRefresh() {
	if (typeof window === 'undefined') return
	if (refreshTimer) {
		clearInterval(refreshTimer)
		refreshTimer = null
	}

	refreshTimer = setInterval(
		async () => {
			const state = get(beatportStore)
			if (state.auth.refresh_token && state.auth.is_authenticated) {
				try {
					await beatportStore.refreshSession()
				} catch (e) {
					console.warn('Background token refresh failed:', e)
				}
			}
		},
		3 * 60 * 1000
	)
}

function createBeatportStore() {
	const { subscribe, set, update } = writable<BeatportState>(initialState)

	function saveCart(cart: BeatportTrack[]) {
		if (typeof window !== 'undefined') {
			try {
				localStorage.setItem(CART_STORAGE_KEY, JSON.stringify(cart))
			} catch (e) {
				console.warn('Failed to save cart:', e)
			}
		}
	}

	function saveAuth(auth: BeatportAuthState) {
		beatportApi.saveBeatportPersistedAuth(auth).catch((e) => {
			console.warn('Failed to persist auth to disk:', e)
		})
	}

	return {
		subscribe,

		updateAuth(auth: BeatportAuthState) {
			update((s) => ({ ...s, auth }))
		},

		updateAudioProgress(currentTime: number, duration: number) {
			update((s) => ({ ...s, audioCurrentTime: currentTime, audioDuration: duration }))
		},

		setAudioPlayingState(isPlaying: boolean) {
			update((s) => ({ ...s, isAudioPlaying: isPlaying }))
		},

		openLoginModal() {
			update((s) => ({ ...s, showLoginModal: true }))
		},

		closeLoginModal() {
			update((s) => ({ ...s, showLoginModal: false }))
		},

		async startPkceLogin() {
			try {
				const authUrl = await beatportApi.getBeatportPkceAuthUrl()
				await openUrl(authUrl)
				toastStore.info("Connexion ouverte dans votre navigateur. Autorisez puis collez le code ou l'URL.")
				update((s) => ({ ...s, showLoginModal: true }))
			} catch (e) {
				console.error('Failed to open Beatport Auth URL:', e)
				toastStore.error(`Impossible d'ouvrir le navigateur : ${toErrorMessage(e, String(e))}`)
			}
		},

		async loginWithPkce(codeOrUrl: string) {
			update((s) => ({ ...s, loading: true, error: null }))
			try {
				const auth = await beatportApi.loginBeatportPkce(codeOrUrl)
				const formattedAuth: BeatportAuthState = {
					...auth,
					is_authenticated: true,
					has_subscription: true,
				}
				saveAuth(formattedAuth)
				toastStore.success(`Compte Beatport connecté avec succès (${formattedAuth.username})`)
				update((s) => ({ ...s, auth: formattedAuth, showLoginModal: false, loading: false }))
				await this.loadInitialData()
			} catch (e) {
				toastStore.error(`Échec de connexion Beatport : ${toErrorMessage(e, String(e))}`)
				update((s) => ({ ...s, loading: false, error: toErrorMessage(e, String(e)) }))
			}
		},

		async loginWithToken(tokenInput: string) {
			update((s) => ({ ...s, loading: true, error: null }))
			try {
				let accessToken = tokenInput.trim()
				let refreshToken: string | null = null

				if (accessToken.startsWith('{') || accessToken.includes('access_token')) {
					try {
						const cleaned = accessToken.trim().replace(/,\s*$/, '')
						const parsed = JSON.parse(cleaned.startsWith('{') ? cleaned : `{${cleaned}}`)
						if (parsed.access_token) {
							accessToken = parsed.access_token
						}
						if (parsed.refresh_token) {
							refreshToken = parsed.refresh_token
						}
					} catch (e) {
						const accessMatch = accessToken.match(/["']?access_token["']?\s*:\s*["']([^"']+)["']/)
						if (accessMatch) accessToken = accessMatch[1]
						const refreshMatch = accessToken.match(/["']?refresh_token["']?\s*:\s*["']([^"']+)["']/)
						if (refreshMatch) refreshToken = refreshMatch[1]
					}
				}

				accessToken = accessToken.replace(/^["']|["']$/g, '').trim()

				const auth = await beatportApi.validateBeatportToken(accessToken, refreshToken)
				const formattedAuth: BeatportAuthState = {
					...auth,
					token: accessToken,
					refresh_token: refreshToken || auth.refresh_token,
					is_authenticated: true,
					has_subscription: true,
				}
				saveAuth(formattedAuth)
				toastStore.success(`Connecté avec token Beatport (${formattedAuth.username})`)
				update((s) => ({ ...s, auth: formattedAuth, showLoginModal: false, loading: false }))
				await this.loadInitialData()
			} catch (e) {
				toastStore.error(`Token Beatport invalide : ${toErrorMessage(e, String(e))}`)
				update((s) => ({ ...s, loading: false, error: toErrorMessage(e, String(e)) }))
			}
		},

		async refreshSession(): Promise<boolean> {
			const state = get({ subscribe })
			const refTok = state.auth.refresh_token
			if (!refTok) return false
			try {
				const newAuth = await beatportApi.refreshBeatportToken(refTok)
				if (newAuth && newAuth.token) {
					const formattedAuth: BeatportAuthState = {
						...newAuth,
						refresh_token: newAuth.refresh_token || refTok,
						is_authenticated: true,
						has_subscription: true,
					}
					saveAuth(formattedAuth)
					update((s) => ({ ...s, auth: formattedAuth, error: null }))
					return true
				}
			} catch (e) {
				console.warn('Session refresh error:', e)
			}
			return false
		},

		async getValidToken(): Promise<string | null> {
			const state = get({ subscribe })
			if (!state.auth.token) return null
			if (isJwtExpired(state.auth.token)) {
				if (state.auth.refresh_token) {
					console.log('Beatport token expired or expiring soon, auto-refreshing...')
					const success = await this.refreshSession()
					if (success) {
						return get({ subscribe }).auth.token
					}
				}
				update((s) => ({
					...s,
					error: 'Votre session Beatport a expiré. Veuillez renouveler votre connexion.',
				}))
				return null
			}
			return state.auth.token
		},

		/**
		 * Loads the session saved by the backend (Keychain) if the store has none yet. Called
		 * explicitly at app start-up, which also starts the token refresh timer (no side effect
		 * when the module is merely imported).
		 */
		async restoreSession() {
			if (!refreshTimer) {
				purgeLegacyStoredAuth()
				setupAutoRefresh()
			}
			const current = get({ subscribe })
			if (current.auth.is_authenticated && current.auth.token) return
			try {
				const persisted = await beatportApi.getBeatportPersistedAuth()
				if (persisted && persisted.is_authenticated && (persisted.token || persisted.refresh_token)) {
					const formattedAuth: BeatportAuthState = {
						...persisted,
						is_authenticated: true,
						has_subscription: true,
					}
					update((s) => ({ ...s, auth: formattedAuth }))
				}
			} catch (e) {
				console.warn('Failed to restore the Beatport session:', e)
			}
		},

		async loadInitialData(forceRefresh = false) {
			const currentSettings = get(settingsStore)
			if (currentSettings.beatportDownloadDestination) {
				update((s) => ({
					...s,
					downloadDestination: s.downloadDestination || currentSettings.beatportDownloadDestination,
				}))
			}

			await this.restoreSession()
			const state = get({ subscribe })

			if (!state.auth.is_authenticated) {
				update((s) => ({
					...s,
					genres: [],
					charts: [],
					topTracks: [],
					currentSectionTracks: [],
					userPlaylists: [],
					favorites: [],
					purchases: [],
					loading: false,
				}))
				return
			}

			const token = await this.getValidToken()
			if (!token) {
				update((s) => ({ ...s, loading: false }))
				return
			}

			// Cache-First: If catalog data is already present, do not block UI with a loading spinner
			const hasExistingData = state.genres.length > 0 && state.topTracks.length > 0
			if (!hasExistingData || forceRefresh) {
				update((s) => ({ ...s, loading: true, error: null }))
			}

			try {
				const [genres, charts, topTracks] = await Promise.all([
					beatportApi.getBeatportGenres(token).catch(() => []),
					beatportApi.getBeatportFeaturedCharts(token).catch(() => []),
					beatportApi.getBeatportTopTracks(token, null).catch(() => []),
				])

				update((s) => {
					// If user already navigated to a specific section, preserve currentSectionTracks unless it was empty
					const currentTracks =
						s.currentSectionTracks.length > 0 && s.navSection !== 'home' ? s.currentSectionTracks : topTracks
					return {
						...s,
						genres: genres.length > 0 ? genres : s.genres,
						charts: charts.length > 0 ? charts : s.charts,
						topTracks: topTracks.length > 0 ? topTracks : s.topTracks,
						currentSectionTracks: currentTracks,
						loading: false,
						error: null,
					}
				})

				await this.loadUserData(token)
			} catch (e) {
				console.error('Failed to load Beatport catalog data:', e)
				update((s) => ({ ...s, loading: false, error: hasExistingData ? null : toErrorMessage(e, String(e)) }))
			}
		},

		async loadUserData(tokenOverride?: string) {
			const token = tokenOverride || (await this.getValidToken())
			if (!token) return

			try {
				const [playlists, favorites, purchases] = await Promise.all([
					beatportApi.getBeatportUserPlaylists(token).catch(() => []),
					beatportApi.getBeatportUserFavorites(token).catch(() => []),
					beatportApi.getBeatportUserPurchases(token).catch(() => []),
				])

				update((s) => ({
					...s,
					userPlaylists: playlists,
					favorites,
					purchases,
					error: null,
				}))
			} catch (e) {
				console.warn('Error loading user data:', e)
			}
		},

		async setNavSection(
			section: BeatportNavSection,
			playlistId: string | null = null,
			playlistName: string | null = null
		) {
			const state = get({ subscribe })
			update((s) => ({
				...s,
				navSection: section,
				selectedPlaylistId: playlistId,
				selectedPlaylistName: playlistName,
				selectedChartId: null,
				selectedChartTitle: null,
				selectedGenreId: null,
				selectedGenreSlug: null,
				selectedGenreName: null,
				selectedArtistId: null,
				selectedArtistName: null,
				selectedArtistImage: null,
				selectedArtistBio: null,
				selectedArtistDetail: null,
				searchQuery: '',
				searchResults: [],
				searchArtists: [],
				loading: true,
			}))

			const token = await this.getValidToken()
			if (!token) {
				update((s) => ({ ...s, loading: false }))
				return
			}

			try {
				let tracks: BeatportTrack[] = []
				if (section === 'home') {
					tracks = state.topTracks.length > 0 ? state.topTracks : await beatportApi.getBeatportTopTracks(token, null)
				} else if (section === 'playlist' && playlistId) {
					tracks = await beatportApi.getBeatportPlaylistTracks(token, playlistId)
				} else if (section === 'favorites') {
					tracks = await beatportApi.getBeatportUserFavorites(token)
				} else if (section === 'purchased') {
					tracks = await beatportApi.getBeatportUserPurchases(token)
				} else if (section === 'offline') {
					tracks = state.favorites.length > 0 ? state.favorites : state.topTracks
				}

				update((s) => ({
					...s,
					currentSectionTracks: tracks,
					loading: false,
					error: null,
				}))
			} catch (e) {
				update((s) => ({ ...s, loading: false, error: toErrorMessage(e, String(e)) }))
			}
		},

		async setNavChart(chartId: string, chartTitle: string) {
			update((s) => ({
				...s,
				navSection: 'chart',
				selectedChartId: chartId,
				selectedChartTitle: chartTitle,
				selectedPlaylistId: null,
				selectedPlaylistName: null,
				selectedGenreId: null,
				selectedGenreSlug: null,
				selectedGenreName: null,
				selectedArtistId: null,
				selectedArtistName: null,
				selectedArtistImage: null,
				selectedArtistBio: null,
				selectedArtistDetail: null,
				searchQuery: '',
				searchResults: [],
				searchArtists: [],
				loading: true,
			}))

			const token = await this.getValidToken()
			if (!token) {
				update((s) => ({ ...s, loading: false }))
				return
			}

			try {
				const tracks = await beatportApi.getBeatportChartTracks(token, chartId)
				update((s) => ({
					...s,
					currentSectionTracks: tracks,
					loading: false,
					error: null,
				}))
			} catch (e) {
				update((s) => ({ ...s, loading: false, error: toErrorMessage(e, String(e)) }))
			}
		},

		async setNavArtist(artistId: number, artistName?: string, artistImage?: string) {
			update((s) => ({
				...s,
				navSection: 'artist',
				selectedArtistId: artistId,
				selectedArtistName: artistName ?? null,
				selectedArtistImage: artistImage ?? null,
				selectedChartId: null,
				selectedChartTitle: null,
				selectedPlaylistId: null,
				selectedPlaylistName: null,
				selectedGenreId: null,
				selectedGenreSlug: null,
				selectedGenreName: null,
				searchQuery: '',
				searchResults: [],
				searchArtists: [],
				loading: true,
			}))

			const token = await this.getValidToken()
			if (!token) {
				update((s) => ({ ...s, loading: false }))
				return
			}

			try {
				const [tracks, detail] = await Promise.all([
					beatportApi.getBeatportArtistTracks(token, artistId),
					beatportApi.getBeatportArtistDetail(token, artistId).catch(() => null),
				])

				update((s) => ({
					...s,
					currentSectionTracks: tracks,
					selectedArtistName: detail?.name ?? artistName ?? s.selectedArtistName,
					selectedArtistImage: detail?.image_url ?? artistImage ?? s.selectedArtistImage,
					selectedArtistBio: detail?.biography ?? null,
					selectedArtistDetail: detail,
					loading: false,
					error: null,
				}))
			} catch (e) {
				update((s) => ({ ...s, loading: false, error: toErrorMessage(e, String(e)) }))
			}
		},

		async selectGenre(genreId: number | null, slug: string | null = null, name: string | null = null) {
			update((s) => ({
				...s,
				navSection: 'home',
				selectedGenreId: genreId,
				selectedGenreSlug: slug,
				selectedGenreName: name,
				selectedChartId: null,
				selectedChartTitle: null,
				selectedPlaylistId: null,
				selectedPlaylistName: null,
				selectedArtistId: null,
				selectedArtistName: null,
				selectedArtistImage: null,
				selectedArtistBio: null,
				selectedArtistDetail: null,
				searchQuery: '',
				searchResults: [],
				searchArtists: [],
				loading: true,
			}))

			const token = await this.getValidToken()
			if (!token) {
				update((s) => ({ ...s, loading: false }))
				return
			}

			try {
				const tracks = await beatportApi.getBeatportTopTracks(token, genreId)
				update((s) => ({ ...s, topTracks: tracks, currentSectionTracks: tracks, loading: false, error: null }))
			} catch (e) {
				update((s) => ({ ...s, loading: false, error: toErrorMessage(e, String(e)) }))
			}
		},

		async search(query: string, searchType: 'tracks' | 'releases' | 'artists' | 'playlists' = 'tracks') {
			if (!query.trim()) {
				update((s) => ({ ...s, searchQuery: '', searchResults: [], searchArtists: [] }))
				return
			}

			update((s) => ({ ...s, searchQuery: query, searchType, loading: true }))

			const token = await this.getValidToken()

			try {
				const searchData = await beatportApi.searchBeatport(token, query)
				update((s) => ({
					...s,
					searchResults: searchData.tracks,
					searchArtists: searchData.artists,
					loading: false,
					error: null,
				}))
			} catch (e) {
				update((s) => ({ ...s, loading: false, error: toErrorMessage(e, String(e)) }))
			}
		},

		clearSearch() {
			update((s) => ({ ...s, searchQuery: '', searchResults: [], searchArtists: [] }))
		},

		addToCart(track: BeatportTrack) {
			update((s) => {
				if (s.cart.some((t) => String(t.id) === String(track.id))) return s
				const nextCart = [...s.cart, track]
				saveCart(nextCart)
				toastStore.success(`Ajouté au panier Beatport : ${track.title}`)
				return { ...s, cart: nextCart }
			})
		},

		removeFromCart(trackId: string | number) {
			update((s) => {
				const nextCart = s.cart.filter((t) => String(t.id) !== String(trackId))
				saveCart(nextCart)
				return { ...s, cart: nextCart }
			})
		},

		clearCart() {
			update((s) => {
				saveCart([])
				return { ...s, cart: [] }
			})
		},

		addBulkToCart(tracks: BeatportTrack[]) {
			update((s) => {
				const existingIds = new Set(s.cart.map((t) => String(t.id)))
				const newTracks = tracks.filter((t) => !existingIds.has(String(t.id)))
				const nextCart = [...s.cart, ...newTracks]
				saveCart(nextCart)
				toastStore.success(`${newTracks.length} morceau(x) ajouté(s) au panier Beatport`)
				return { ...s, cart: nextCart }
			})
		},

		toggleFavorite(track: BeatportTrack) {
			update((s) => {
				const isFav = s.favorites.some((t) => String(t.id) === String(track.id))
				const nextFavs = isFav ? s.favorites.filter((t) => String(t.id) !== String(track.id)) : [...s.favorites, track]
				toastStore.info(
					isFav
						? `Retiré des favoris locaux : ${track.title}`
						: `Ajouté aux favoris locaux (non synchronisés avec Beatport) : ${track.title}`
				)
				return { ...s, favorites: nextFavs }
			})
		},

		createPlaylist(name: string) {
			update((s) => {
				const newPl: BeatportPlaylist = {
					id: `pl-${Date.now()}`,
					name,
					track_count: 0,
					is_public: false,
				}
				toastStore.info(`Playlist locale créée (non synchronisée avec Beatport) : ${name}`)
				return { ...s, userPlaylists: [...s.userPlaylists, newPl] }
			})
		},

		playAudioPreview(track: BeatportTrack) {
			playerStore.playBeatport(track)
			update((s) => ({
				...s,
				currentlyPlayingTrack: track,
				isAudioPlaying: true,
			}))
		},

		stopAudio() {
			playerStore.stop()
			update((s) => ({
				...s,
				isAudioPlaying: false,
			}))
		},

		togglePlayPause(track?: BeatportTrack) {
			if (track) {
				playerStore.playBeatport(track)
			} else {
				playerStore.togglePlayPause()
			}
		},

		async downloadSelection(tracksToDownload?: BeatportTrack[]) {
			const state = get({ subscribe })
			const tracks = tracksToDownload || state.cart
			if (tracks.length === 0) {
				toastStore.info('Aucun morceau sélectionné pour le téléchargement')
				return
			}

			const currentSettings = get(settingsStore)
			const dest = state.downloadDestination || currentSettings.beatportDownloadDestination || undefined

			// One track per call: each success leaves the cart as soon as it is verified, failures
			// stay in it, and the progress is known.
			update((s) => ({ ...s, isDownloading: true, downloadProgressText: null }))
			let succeeded = 0
			const failures: string[] = []
			for (const [index, track] of tracks.entries()) {
				update((s) => ({
					...s,
					downloadProgressText: `Téléchargement ${index + 1}/${tracks.length} : ${track.title}`,
				}))
				try {
					const result = await beatportApi.downloadBeatportTracks([track], dest, state.beatportdlPath ?? undefined)
					if (result.success_count > 0) {
						succeeded++
						update((s) => {
							const nextCart = s.cart.filter((t) => String(t.id) !== String(track.id))
							saveCart(nextCart)
							return { ...s, cart: nextCart }
						})
					} else {
						failures.push(`${track.title}${result.errors.length > 0 ? ` (${result.errors[0]})` : ''}`)
					}
				} catch (e) {
					failures.push(`${track.title} (${toErrorMessage(e, 'erreur inconnue')})`)
				}
			}

			update((s) => ({ ...s, isDownloading: false, downloadProgressText: null }))
			if (succeeded > 0) {
				toastStore.success(`${succeeded} morceau(x) téléchargé(s) en FLAC et importé(s) dans Crate`)
			}
			if (failures.length > 0) {
				toastStore.warning(`${failures.length} échec(s), restés dans le panier : ${failures.join(' ; ')}`)
			}
		},

		logout() {
			const resetAuth: BeatportAuthState = defaultAuth()
			saveAuth(resetAuth)
			beatportApi.clearBeatportPersistedAuth().catch(() => {})
			toastStore.info('Déconnecté de Beatport')
			update((s) => ({
				...s,
				auth: resetAuth,
				genres: [],
				charts: [],
				topTracks: [],
				currentSectionTracks: [],
				userPlaylists: [],
				favorites: [],
				purchases: [],
				error: null,
			}))
		},

		setBeatportdlPath(path: string | null) {
			update((s) => ({ ...s, beatportdlPath: path }))
		},

		setDownloadDestination(path: string | null) {
			update((s) => ({ ...s, downloadDestination: path }))
		},
	}
}

export const beatportStore = createBeatportStore()

export const beatportCart = derived(beatportStore, ($s) => $s.cart)
export const beatportCartCount = derived(beatportStore, ($s) => $s.cart.length)
export const beatportCartDuration = derived(beatportStore, ($s) =>
	$s.cart.reduce((acc, t) => acc + (t.duration_ms || 0), 0)
)
export const isBeatportAuthenticated = derived(beatportStore, ($s) => $s.auth.is_authenticated)
