<script lang="ts">
	import type { SpotifyAuthState, SpotifyNowPlaying, RekordboxSession } from '$shared/types'
	import { statsStore, isSyncingRekordbox, isImportingSpotify } from '$shared/stores/stats'
	import * as statsApi from '$shared/api/stats'
	import Icon from '$lib/components/common/Icon.svelte'
	import { Spinner } from '$lib/components/common'
	import { openUrl } from '@tauri-apps/plugin-opener'
	import { toastStore } from '$shared/stores/toast'
	import { listen } from '@tauri-apps/api/event'
	import { onMount } from 'svelte'

	type Props = {
		spotifyAuth: SpotifyAuthState | null
		spotifyNowPlaying: SpotifyNowPlaying | null
		rekordboxDetected: boolean
		rekordboxSessions: RekordboxSession[]
		mikDetected?: boolean
	}

	let { spotifyAuth, spotifyNowPlaying, rekordboxDetected, rekordboxSessions, mikDetected = false }: Props = $props()

	let fileInputRef: HTMLInputElement | undefined
	let showSpotifyModal = $state(false)
	let spotifyClientIdInput = $state('')
	let spotifyClientSecretInput = $state('')
	let hasStoredSpotifySecret = $state(false)
	let isCopiedRedirectUri = $state(false)
	let showManualCodeFallback = $state(false)
	let manualCodeInput = $state('')
	let isSubmittingManualCode = $state(false)

	onMount(() => {
		let unlistenAuth: (() => void) | null = null
		listen<SpotifyAuthState>('spotify-auth-changed', (event) => {
			if (event.payload?.is_connected) {
				showSpotifyModal = false
				statsStore.refreshAll()
				toastStore.success('Compte Spotify connecté avec succès !')
			}
		}).then((unlisten) => {
			unlistenAuth = unlisten
		})

		return () => {
			unlistenAuth?.()
		}
	})

	function handleTriggerFilePicker() {
		fileInputRef?.click()
	}

	async function handleFileSelected(e: Event) {
		const target = e.target as HTMLInputElement
		const file = target.files?.[0]
		if (!file) return

		try {
			const text = await file.text()
			await statsStore.importSpotifyJson(text)
		} catch (err) {
			console.error('Failed to read Spotify JSON file:', err)
		} finally {
			target.value = ''
		}
	}

	async function openSpotifyConnectModal() {
		try {
			const storedId = await statsApi.getSpotifyClientId()
			if (storedId && storedId !== 'crate-pulse-spotify') {
				spotifyClientIdInput = storedId
			}
			hasStoredSpotifySecret = await statsApi.hasSpotifyClientSecret()
			spotifyClientSecretInput = ''
		} catch (err) {
			console.error('Error fetching Spotify credentials:', err)
		}
		manualCodeInput = ''
		showManualCodeFallback = false
		showSpotifyModal = true
	}

	function closeSpotifyModal() {
		showSpotifyModal = false
	}

	async function handleCopyRedirectUri() {
		try {
			await navigator.clipboard.writeText('http://127.0.0.1:8888/callback')
			isCopiedRedirectUri = true
			toastStore.success('Redirect URI copié dans le presse-papier !')
			setTimeout(() => {
				isCopiedRedirectUri = false
			}, 3000)
		} catch {
			toastStore.info('http://127.0.0.1:8888/callback')
		}
	}

	async function handleOpenDeveloperDashboard() {
		try {
			await openUrl('https://developer.spotify.com/dashboard')
		} catch (err) {
			console.error('Failed to open URL:', err)
		}
	}

	async function handleConfirmSpotifyConnect() {
		const cleanId = spotifyClientIdInput.trim()
		if (!cleanId) {
			toastStore.error('Veuillez renseigner un Client ID Spotify valide')
			return
		}
		const cleanSecret = spotifyClientSecretInput.trim()
		await statsStore.connectSpotify(cleanId, cleanSecret)
	}

	async function handleManualCodeSubmit() {
		const raw = manualCodeInput.trim()
		if (!raw) {
			toastStore.error("Veuillez coller le code d'autorisation ou l'URL de redirection")
			return
		}
		isSubmittingManualCode = true
		try {
			const cleanId = spotifyClientIdInput.trim() || undefined
			const cleanSecret = spotifyClientSecretInput.trim()
			if (cleanSecret) {
				await statsApi.setSpotifyClientSecret(cleanSecret)
			}
			const result = await statsStore.handleSpotifyCallback(raw, cleanId)
			if (result && result.is_connected) {
				manualCodeInput = ''
				showSpotifyModal = false
			}
		} finally {
			isSubmittingManualCode = false
		}
	}
</script>

<div class="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
	<!-- 1. Spotify Integration Card -->
	<div
		class="relative flex flex-col justify-between overflow-hidden rounded-2xl border border-[#1DB954]/25 bg-gradient-to-b from-[#121c15]/60 to-surface-1/80 p-5 shadow-lg backdrop-blur-xl space-y-4"
	>
		<div class="space-y-4">
			<div class="flex items-start justify-between">
				<div class="flex items-center gap-3">
					<div class="flex h-11 w-11 items-center justify-center rounded-xl bg-[#1DB954]/15 border border-[#1DB954]/30 text-[#1DB954] shadow-inner flex-shrink-0">
						<Icon name="spotify" class="h-6 w-6 text-[#1DB954]" />
					</div>
					<div>
						<div class="flex items-center gap-2">
							<h3 class="text-sm font-bold text-text-primary">Spotify Pulse</h3>
							{#if spotifyAuth?.is_connected}
								<span class="inline-flex items-center gap-1 rounded-full bg-[#1DB954]/20 border border-[#1DB954]/40 px-2 py-0.5 text-[10px] font-bold text-[#1DB954]">
									<span class="h-1.5 w-1.5 rounded-full bg-[#1DB954] animate-pulse"></span>
									En direct
								</span>
							{:else}
								<span class="rounded-full bg-surface-3 border border-stroke px-2 py-0.5 text-[10px] font-medium text-text-tertiary">
									Non connecté
								</span>
							{/if}
						</div>
						<p class="text-[11px] text-text-secondary mt-0.5">
							{#if spotifyAuth?.is_connected}
								Compte associé : <strong class="text-text-primary">{spotifyAuth.user_name || spotifyAuth.user_id || 'Utilisateur'}</strong>
							{:else}
								Écoutes en temps réel (OAuth2 PKCE)
							{/if}
						</p>
					</div>
				</div>
			</div>

			<!-- Live Now Playing Mini Status (if playing on Spotify) -->
			{#if spotifyNowPlaying?.is_playing}
				<div class="flex items-center gap-3 rounded-xl border border-[#1DB954]/30 bg-[#1DB954]/10 p-2.5">
					{#if spotifyNowPlaying.artwork_url}
						<img src={spotifyNowPlaying.artwork_url} alt="" class="h-9 w-9 rounded-lg object-cover shadow-sm" />
					{:else}
						<div class="flex h-9 w-9 items-center justify-center rounded-lg bg-surface-3 text-[#1DB954]">
							<Icon name="music-note" class="h-4 w-4" />
						</div>
					{/if}
					<div class="min-w-0 flex-1">
						<div class="flex items-center gap-1.5">
							<span class="text-[10px] font-bold uppercase tracking-wider text-[#1DB954]">Lecture en cours</span>
							{#if spotifyNowPlaying.device_name}
								<span class="text-[10px] text-text-tertiary">({spotifyNowPlaying.device_name})</span>
							{/if}
						</div>
						<div class="truncate text-xs font-bold text-text-primary">{spotifyNowPlaying.title}</div>
						<div class="truncate text-[11px] text-text-secondary">{spotifyNowPlaying.artist}</div>
					</div>
				</div>
			{/if}
		</div>

		<!-- Action Buttons -->
		<div class="flex flex-wrap items-center gap-2 pt-1">
			{#if spotifyAuth?.is_connected}
				<button
					type="button"
					class="flex items-center gap-1.5 rounded-xl border border-red-500/30 bg-red-500/10 px-3 py-1.5 text-xs font-bold text-red-400 hover:bg-red-500/20 active:scale-95 transition-all cursor-pointer"
					onclick={() => statsStore.disconnectSpotify()}
				>
					<Icon name="x" class="h-3.5 w-3.5" />
					<span>Déconnecter</span>
				</button>
			{:else}
				<button
					type="button"
					class="flex items-center gap-1.5 rounded-xl bg-[#1DB954] px-3.5 py-1.5 text-xs font-bold text-black shadow-lg shadow-[#1DB954]/20 hover:bg-[#1ed760] active:scale-95 transition-all cursor-pointer"
					onclick={openSpotifyConnectModal}
				>
					<Icon name="link" class="h-3.5 w-3.5" />
					<span>Se connecter</span>
				</button>
			{/if}

			<!-- Hidden File Input for JSON import -->
			<input
				type="file"
				accept=".json,application/json"
				bind:this={fileInputRef}
				onchange={handleFileSelected}
				class="hidden"
			/>

			<button
				type="button"
				class="flex items-center gap-1.5 rounded-xl border border-stroke bg-surface-2/80 px-3 py-1.5 text-xs font-semibold text-text-primary hover:border-stroke-strong hover:bg-surface-3 active:scale-95 transition-all cursor-pointer"
				onclick={handleTriggerFilePicker}
				disabled={$isImportingSpotify}
			>
				{#if $isImportingSpotify}
					<Spinner class="h-3.5 w-3.5 text-text-primary" />
					<span>Import...</span>
				{:else}
					<Icon name="upload" class="h-3.5 w-3.5 text-text-secondary" />
					<span>Importer JSON</span>
				{/if}
			</button>
		</div>
	</div>

	<!-- 2. Rekordbox Integration Card -->
	<div
		class="relative flex flex-col justify-between overflow-hidden rounded-2xl border border-red-500/25 bg-gradient-to-b from-[#1c1214]/60 to-surface-1/80 p-5 shadow-lg backdrop-blur-xl space-y-4"
	>
		<div class="space-y-4">
			<div class="flex items-start justify-between">
				<div class="flex items-center gap-3">
					<div class="flex h-11 w-11 items-center justify-center rounded-xl bg-red-500/15 border border-red-500/30 text-red-400 shadow-inner flex-shrink-0">
						<Icon name="activity" class="h-6 w-6 text-red-400" />
					</div>
					<div>
						<div class="flex items-center gap-2">
							<h3 class="text-sm font-bold text-text-primary">Rekordbox DJ</h3>
							{#if rekordboxDetected}
								<span class="inline-flex items-center gap-1 rounded-full bg-emerald-500/20 border border-emerald-500/40 px-2 py-0.5 text-[10px] font-bold text-emerald-400">
									<span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>
									Détecté
								</span>
							{:else}
								<span class="rounded-full bg-surface-3 border border-stroke px-2 py-0.5 text-[10px] font-medium text-text-tertiary">
									Non détecté
								</span>
							{/if}
						</div>
						<p class="text-[11px] text-text-secondary mt-0.5">
							Pioneer DJ & Rekordbox 6/7
						</p>
					</div>
				</div>
			</div>

			<!-- Sessions Info Callout -->
			<div class="flex items-center justify-between rounded-xl border border-stroke/50 bg-surface-2/60 p-2.5 text-xs">
				<div>
					<div class="font-bold text-text-primary">{rekordboxSessions.length} sessions DJ</div>
					<div class="text-[11px] text-text-tertiary">Sets, transitions et cue points</div>
				</div>
				<span class="font-mono text-[11px] font-bold text-red-400">
					master.db
				</span>
			</div>
		</div>

		<!-- Action Button -->
		<div class="pt-1">
			<button
				type="button"
				class="flex items-center gap-2 rounded-xl bg-red-500 px-3.5 py-1.5 text-xs font-bold text-white shadow-lg shadow-red-500/20 hover:bg-red-600 active:scale-95 transition-all cursor-pointer"
				onclick={() => statsStore.syncRekordbox()}
				disabled={$isSyncingRekordbox}
			>
				{#if $isSyncingRekordbox}
					<Spinner class="h-3.5 w-3.5 text-white" />
					<span>Synchronisation...</span>
				{:else}
					<Icon name="refresh-cw" class="h-3.5 w-3.5 text-white" />
					<span>Synchroniser Rekordbox</span>
				{/if}
			</button>
		</div>
	</div>

	<!-- 3. Mixed In Key 11 Pro Integration Card -->
	<div
		class="relative flex flex-col justify-between overflow-hidden rounded-2xl border border-[#00D2FF]/25 bg-gradient-to-b from-[#0c1a24]/60 to-surface-1/80 p-5 shadow-lg backdrop-blur-xl space-y-4"
	>
		<div class="space-y-4">
			<div class="flex items-start justify-between">
				<div class="flex items-center gap-3">
					<div class="flex h-11 w-11 items-center justify-center rounded-xl bg-[#00D2FF]/15 border border-[#00D2FF]/30 text-[#00D2FF] shadow-inner flex-shrink-0">
						<Icon name="sparkles" class="h-6 w-6 text-[#00D2FF]" />
					</div>
					<div>
						<div class="flex items-center gap-2">
							<h3 class="text-sm font-bold text-text-primary">Mixed In Key 11 Pro</h3>
							{#if mikDetected}
								<span class="inline-flex items-center gap-1 rounded-full bg-[#00D2FF]/20 border border-[#00D2FF]/40 px-2 py-0.5 text-[10px] font-bold text-[#00D2FF]">
									<span class="h-1.5 w-1.5 rounded-full bg-[#00D2FF] animate-pulse"></span>
									En direct
								</span>
							{:else}
								<span class="rounded-full bg-surface-3 border border-stroke px-2 py-0.5 text-[10px] font-medium text-text-tertiary">
									Détecteur actif
								</span>
							{/if}
						</div>
						<p class="text-[11px] text-text-secondary mt-0.5">
							Analyse harmonique & Energy Level
						</p>
					</div>
				</div>
			</div>

			<!-- Info Callout -->
			<div class="flex items-center justify-between rounded-xl border border-stroke/50 bg-surface-2/60 p-2.5 text-xs">
				<div>
					<div class="font-bold text-text-primary">Chronométrage direct MIK</div>
					<div class="text-[11px] text-text-tertiary">Capture automatique dès l'écoute dans MIK 11</div>
				</div>
				<span class="font-mono text-[11px] font-bold text-[#00D2FF]">
					1A - 12B
				</span>
			</div>
		</div>

		<!-- Status Label -->
		<div class="flex items-center gap-2 pt-1 text-xs text-text-secondary">
			<span class="inline-block h-2 w-2 rounded-full {mikDetected ? 'bg-[#00D2FF] animate-ping' : 'bg-surface-4'}"></span>
			<span class="text-[11px]">
				{mikDetected ? 'Morceau chargé détecté dans Mixed In Key 11' : 'En attente de lecture dans Mixed In Key 11'}
			</span>
		</div>
	</div>
</div>

<!-- ========================================================================= -->
<!-- Spotify Connection Modal (Liquid Glass) -->
<!-- ========================================================================= -->
{#if showSpotifyModal}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-md animate-in fade-in duration-200"
		role="dialog"
		aria-modal="true"
		tabindex="-1"
		onclick={(e) => {
			if (e.target === e.currentTarget) closeSpotifyModal()
		}}
		onkeydown={(e) => {
			if (e.key === 'Escape') closeSpotifyModal()
		}}
	>
		<div
			class="relative w-full max-w-lg rounded-3xl border border-[#1DB954]/30 bg-[#0d1711]/95 p-6 shadow-2xl backdrop-blur-2xl text-text-primary space-y-6"
		>
			<!-- Header -->
			<div class="flex items-start justify-between">
				<div class="flex items-center gap-3">
					<div class="flex h-12 w-12 items-center justify-center rounded-2xl bg-[#1DB954]/20 border border-[#1DB954]/40 text-[#1DB954] shadow-lg shadow-[#1DB954]/10">
						<Icon name="spotify" class="h-7 w-7 text-[#1DB954]" />
					</div>
					<div>
						<h2 class="text-lg font-bold text-white">Connexion Spotify Pulse</h2>
						<p class="text-xs text-text-secondary">Guide de configuration rapide en 3 étapes (30s)</p>
					</div>
				</div>
				<button
					type="button"
					class="flex h-8 w-8 items-center justify-center rounded-full bg-surface-2 text-text-tertiary hover:text-white hover:bg-surface-3 transition-colors cursor-pointer"
					onclick={closeSpotifyModal}
					aria-label="Fermer"
				>
					<Icon name="x" class="h-4 w-4" />
				</button>
			</div>

			<!-- 3-Step Guide -->
			<div class="space-y-3 rounded-2xl border border-stroke/50 bg-surface-1/80 p-4 text-xs">
				<div class="flex items-start gap-3">
					<span class="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-full bg-[#1DB954]/20 text-[11px] font-bold text-[#1DB954]">1</span>
					<div class="flex-1 space-y-1">
						<div class="font-bold text-text-primary">Ouvrir le portail développeur Spotify</div>
						<div class="flex items-center gap-2">
							<button
								type="button"
								class="inline-flex items-center gap-1.5 rounded-lg bg-surface-3 px-2.5 py-1 text-[11px] font-medium text-emerald-400 hover:bg-surface-4 border border-emerald-500/20 transition-all cursor-pointer"
								onclick={handleOpenDeveloperDashboard}
							>
								<span>Spotify Developer Dashboard</span>
								<Icon name="external-link" class="h-3 w-3" />
							</button>
						</div>
					</div>
				</div>

				<div class="flex items-start gap-3">
					<span class="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-full bg-[#1DB954]/20 text-[11px] font-bold text-[#1DB954]">2</span>
					<div class="flex-1 space-y-1.5">
						<div class="font-bold text-text-primary">Créer une application gratuite</div>
						<p class="text-[11px] text-text-secondary">
							Nom de l'app : <strong class="text-white">Crate</strong> &middot; API : <strong class="text-white">Web API</strong>
						</p>
						<div class="flex items-center gap-2">
							<span class="text-[11px] text-text-tertiary">Redirect URI :</span>
							<code class="rounded bg-black/50 px-2 py-0.5 font-mono text-[10px] text-emerald-400 border border-emerald-500/20">
								http://127.0.0.1:8888/callback
							</code>
							<button
								type="button"
								class="rounded bg-surface-3 px-2 py-0.5 text-[10px] font-semibold text-text-primary hover:bg-surface-4 transition-colors cursor-pointer"
								onclick={handleCopyRedirectUri}
							>
								{isCopiedRedirectUri ? '✓ Copié' : 'Copier'}
							</button>
						</div>
						<p class="text-[10.5px] text-amber-300/90 font-medium">
							Important : Spotify exige l'adresse IP exacte <code class="rounded bg-black/40 px-1 py-0.5 font-mono text-[10px] text-amber-200">http://127.0.0.1:8888/callback</code> (et refuse <code class="line-through text-amber-400/60">localhost</code>).
						</p>
					</div>
				</div>

				<div class="flex items-start gap-3">
					<span class="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-full bg-[#1DB954]/20 text-[11px] font-bold text-[#1DB954]">3</span>
					<div class="flex-1">
						<div class="font-bold text-text-primary">Coller vos identifiants Spotify ci-dessous</div>
					</div>
				</div>
			</div>

			<!-- Input Fields -->
			<div class="space-y-4">
				<div class="space-y-1.5">
					<label for="spotify-client-id" class="flex items-center justify-between text-xs font-bold uppercase tracking-wider text-text-secondary">
						<span>Client ID Spotify</span>
						<span class="text-[10px] text-emerald-400 font-semibold lowercase">obligatoire</span>
					</label>
					<div class="relative">
						<input
							id="spotify-client-id"
							type="text"
							bind:value={spotifyClientIdInput}
							placeholder="Ex: 4a2b8c9d0e1f2a3b4c5d6e7f8a9b0c1d"
							class="w-full rounded-xl border border-stroke bg-surface-1/90 px-4 py-2.5 font-mono text-xs text-white placeholder-text-tertiary focus:border-[#1DB954] focus:outline-none focus:ring-2 focus:ring-[#1DB954]/20"
							onkeydown={(e) => {
								if (e.key === 'Enter') handleConfirmSpotifyConnect()
							}}
						/>
					</div>
				</div>

				<div class="space-y-1.5">
					<label for="spotify-client-secret" class="flex items-center justify-between text-xs font-bold uppercase tracking-wider text-text-secondary">
						<span>Client Secret Spotify</span>
						<span class="text-[10px] text-text-tertiary font-normal lowercase">optionnel mais recommandé</span>
					</label>
					<div class="relative">
						<input
							id="spotify-client-secret"
							type="password"
							bind:value={spotifyClientSecretInput}
							placeholder={hasStoredSpotifySecret ? 'Secret enregistré — laisser vide pour le conserver' : 'Ex: 8f7e6d5c4b3a210987654321fedcba09'}
							class="w-full rounded-xl border border-stroke bg-surface-1/90 px-4 py-2.5 font-mono text-xs text-white placeholder-text-tertiary focus:border-[#1DB954] focus:outline-none focus:ring-2 focus:ring-[#1DB954]/20"
							onkeydown={(e) => {
								if (e.key === 'Enter') handleConfirmSpotifyConnect()
							}}
						/>
					</div>
					<p class="text-[11px] text-text-tertiary italic">
						Cliquez sur &laquo;&nbsp;View client secret&nbsp;&raquo; sur votre dashboard Spotify pour l'obtenir
					</p>
				</div>
			</div>

			<!-- Fallback Section: Collapsible Manual Code / URL Input -->
			<div class="border-t border-stroke/40 pt-3">
				<button
					type="button"
					class="flex w-full items-center justify-between py-1 text-xs font-semibold text-text-tertiary hover:text-text-primary transition-colors cursor-pointer"
					onclick={() => (showManualCodeFallback = !showManualCodeFallback)}
				>
					<span class="flex items-center gap-1.5">
						<Icon name="key" class="h-3.5 w-3.5 text-[#1DB954]" />
						<span>Ou coller le code d'autorisation / URL reçue manuellement</span>
					</span>
					<Icon
						name="chevron-right"
						class="h-3.5 w-3.5 transition-transform duration-200 {showManualCodeFallback ? 'rotate-90' : ''}"
					/>
				</button>

				{#if showManualCodeFallback}
					<div class="mt-2.5 space-y-2 rounded-2xl border border-stroke/50 bg-surface-1/90 p-3.5 text-xs">
						<p class="text-[11px] text-text-secondary leading-relaxed">
							Si le navigateur ne redirige pas automatiquement, collez ici l'URL complète affichée dans la barre d'adresse ou le code d'autorisation :
						</p>
						<div class="flex gap-2">
							<input
								type="text"
								bind:value={manualCodeInput}
								placeholder="Ex: http://127.0.0.1:8888/callback?code=AQD... ou AQD..."
								class="flex-1 rounded-xl border border-stroke bg-surface-2/90 px-3 py-2 font-mono text-[11px] text-white placeholder-text-tertiary focus:border-[#1DB954] focus:outline-none focus:ring-1 focus:ring-[#1DB954]/20"
								onkeydown={(e) => {
									if (e.key === 'Enter') handleManualCodeSubmit()
								}}
							/>
							<button
								type="button"
								class="flex items-center gap-1.5 rounded-xl bg-emerald-500/20 border border-emerald-500/40 px-3.5 py-2 text-xs font-bold text-emerald-400 hover:bg-emerald-500/30 active:scale-95 transition-all cursor-pointer disabled:opacity-50"
								onclick={handleManualCodeSubmit}
								disabled={isSubmittingManualCode || !manualCodeInput.trim()}
							>
								{#if isSubmittingManualCode}
									<Spinner class="h-3.5 w-3.5 text-emerald-400" />
									<span>Validation...</span>
								{:else}
									<Icon name="check" class="h-3.5 w-3.5" />
									<span>Valider</span>
								{/if}
							</button>
						</div>
					</div>
				{/if}
			</div>

			<!-- Modal Footer Actions -->
			<div class="flex items-center justify-end gap-3 pt-2">
				<button
					type="button"
					class="rounded-xl border border-stroke bg-surface-2 px-4 py-2 text-xs font-semibold text-text-secondary hover:text-text-primary hover:bg-surface-3 transition-colors cursor-pointer"
					onclick={closeSpotifyModal}
				>
					Annuler
				</button>
				<button
					type="button"
					class="flex items-center gap-2 rounded-xl bg-[#1DB954] px-5 py-2 text-xs font-bold text-black shadow-lg shadow-[#1DB954]/25 hover:bg-[#1ed760] active:scale-95 transition-all cursor-pointer"
					onclick={handleConfirmSpotifyConnect}
				>
					<Icon name="link" class="h-3.5 w-3.5" />
					<span>Se connecter à Spotify</span>
				</button>
			</div>
		</div>
	</div>
{/if}

