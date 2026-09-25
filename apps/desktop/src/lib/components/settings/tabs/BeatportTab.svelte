<script lang="ts">
	import { beatportStore } from '$shared/stores/beatport'
	import { settingsStore } from '$shared/stores/settings'
	import * as settingsApi from '$shared/api/settings'
	import { Button, Text, Icon, ToggleSwitch } from '$lib/components/common'
	import { toastStore } from '$shared/stores/toast'
	import { open } from '@tauri-apps/plugin-dialog'
	import { withNativeDialog } from '$shared/utils'

	let downloadDest = $state(
		$settingsStore.beatportDownloadDestination ||
		$beatportStore.downloadDestination ||
		'~/Music/My Library/FLAC'
	)
	let autoSyncMik = $state($settingsStore.beatportAutoSyncMik ?? true)
	let selectedAudioQuality = $state<'flac' | 'aac' | 'mp3'>(
		($settingsStore.beatportAudioQuality as 'flac' | 'aac' | 'mp3') || 'flac'
	)
	let isSaving = $state(false)

	async function handleBrowseFolder() {
		const selected = await withNativeDialog(() =>
			open({
				directory: true,
				multiple: false,
				title: 'Sélectionner le dossier de destination des FLAC',
			})
		)
		if (selected && typeof selected === 'string') {
			downloadDest = selected
		}
	}

	async function handleSave() {
		isSaving = true
		try {
			await settingsStore.setBeatportDownloadDestination(downloadDest)
			await settingsApi.setSetting('beatport_download_destination', downloadDest)
			await settingsStore.setBeatportAudioQuality(selectedAudioQuality)
			await settingsApi.setSetting('beatport_audio_quality', selectedAudioQuality)
			await settingsStore.setBeatportAutoSyncMik(autoSyncMik)
			await settingsApi.setSetting('beatport_auto_sync_mik', autoSyncMik ? 'true' : 'false')
			beatportStore.setDownloadDestination(downloadDest)
			toastStore.success('Paramètres Beatport enregistrés avec succès')
		} catch (e: any) {
			toastStore.error(`Erreur lors de l'enregistrement : ${e?.message || e}`)
		} finally {
			isSaving = false
		}
	}

	function handleOpenLogin() {
		beatportStore.openLoginModal()
	}

	function handleLogout() {
		beatportStore.logout()
		toastStore.info('Session Beatport déconnectée')
	}
</script>

<div class="space-y-6 max-w-xl select-none">
	<!-- Section 1: Compte Beatport -->
	<div>
		<div class="text-xs font-semibold text-text-primary mb-2">Compte Beatport</div>
		<div class="flex items-center justify-between rounded-lg border border-stroke/70 bg-surface-1 px-3.5 py-2.5 shadow-xs">
			<div class="flex items-center gap-3 min-w-0">
				<Icon name="beatport" class="h-4 w-4 text-text-primary flex-shrink-0" />
				<div class="flex items-center gap-2 min-w-0">
					{#if $beatportStore.auth.isAuthenticated}
						<span class="truncate text-xs font-semibold text-text-primary">{$beatportStore.auth.username}</span>
						<span class="h-1.5 w-1.5 rounded-full bg-emerald-500 flex-shrink-0" title="Connecté"></span>
					{:else}
						<span class="text-xs text-text-secondary">Non connecté</span>
					{/if}
				</div>
			</div>

			<div>
				{#if $beatportStore.auth.isAuthenticated}
					<button
						type="button"
						onclick={handleLogout}
						class="text-xs text-text-tertiary hover:text-red-500 transition-colors cursor-pointer"
					>
						Déconnexion
					</button>
				{:else}
					<Button variant="primary" size="sm" onclick={handleOpenLogin}>
						Se connecter
					</Button>
				{/if}
			</div>
		</div>
	</div>

	<!-- Section 2: Format & Qualité Audio -->
	<div>
		<div class="text-xs font-semibold text-text-primary mb-1">Qualité audio</div>
		<p class="text-[11px] text-text-tertiary mb-2">
			Qualité appliquée pour la lecture en streaming et les téléchargements.
		</p>

		<div class="inline-flex rounded-lg bg-surface-2 p-1 border border-stroke/60 gap-1">
			<button
				type="button"
				class="rounded-md px-3 py-1.5 text-xs font-medium transition-all cursor-pointer {selectedAudioQuality === 'flac'
					? 'bg-surface-0 text-text-primary font-semibold shadow-xs border border-stroke/50'
					: 'text-text-secondary hover:text-text-primary'}"
				onclick={() => (selectedAudioQuality = 'flac')}
			>
				FLAC Lossless
			</button>
			<button
				type="button"
				class="rounded-md px-3 py-1.5 text-xs font-medium transition-all cursor-pointer {selectedAudioQuality === 'aac'
					? 'bg-surface-0 text-text-primary font-semibold shadow-xs border border-stroke/50'
					: 'text-text-secondary hover:text-text-primary'}"
				onclick={() => (selectedAudioQuality = 'aac')}
			>
				AAC 256k
			</button>
			<button
				type="button"
				class="rounded-md px-3 py-1.5 text-xs font-medium transition-all cursor-pointer {selectedAudioQuality === 'mp3'
					? 'bg-surface-0 text-text-primary font-semibold shadow-xs border border-stroke/50'
					: 'text-text-secondary hover:text-text-primary'}"
				onclick={() => (selectedAudioQuality = 'mp3')}
			>
				MP3 320k
			</button>
		</div>
	</div>

	<!-- Section 3: Dossier de téléchargement -->
	<div>
		<label for="bp-dest-dir" class="block text-xs font-semibold text-text-primary mb-1">
			Dossier de destination
		</label>
		<p class="text-[11px] text-text-tertiary mb-2">
			Emplacement local où sont stockés les fichiers FLAC téléchargés.
		</p>
		<div class="flex items-center gap-2">
			<input
				id="bp-dest-dir"
				type="text"
				bind:value={downloadDest}
				placeholder="/Users/.../Music/My Library/FLAC"
				class="flex-1 rounded-lg bg-surface-2 border border-stroke px-3 py-1.5 text-xs font-mono text-text-primary focus:border-brand-primary focus:outline-none"
			/>
			<Button variant="secondary" size="sm" onclick={handleBrowseFolder}>
				Parcourir...
			</Button>
		</div>
	</div>

	<!-- Section 4: Synchronisation Mixed In Key & Crate -->
	<div class="flex items-center justify-between rounded-lg border border-stroke/60 bg-surface-1 p-3">
		<div>
			<div class="text-xs font-medium text-text-primary">Import & synchronisation automatique</div>
			<div class="text-[11px] text-text-tertiary">Importer dans la bibliothèque Crate et Mixed In Key après téléchargement</div>
		</div>
		<ToggleSwitch checked={autoSyncMik} onchange={(v) => (autoSyncMik = v)} />
	</div>

	<!-- Save Action -->
	<div class="pt-2">
		<Button variant="primary" size="sm" onclick={handleSave} disabled={isSaving}>
			Enregistrer
		</Button>
	</div>
</div>
