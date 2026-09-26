<script lang="ts">
	import { onMount } from 'svelte'
	import { listen } from '@tauri-apps/api/event'
	import { Button, IconButton, Tooltip, MixedInKeyLogo } from '$lib/components/common'
	import Icon from '$lib/components/common/Icon.svelte'
	import { SyncStatusIndicator } from '$lib/components/cloud-sync'
	import { isDev, libraryStore } from '$lib/stores'
	import { duplicateStore, duplicateGroupCount, duplicateTrackCount } from '$shared/stores/duplicate'
	import { upgraderStore, upgraderMatchCount } from '$shared/stores/upgrader'
	import { toastStore } from '$shared/stores/toast'
	import { translate } from '$shared/i18n'
	import * as libraryApi from '$shared/api/library'
	import * as exportApi from '$shared/api/export'
	import { save } from '@tauri-apps/plugin-dialog'
	import { withNativeDialog } from '$shared/utils'

	type Props = {
		activeView?: string
		onViewChange?: (view: 'library' | 'discovery' | 'beatport' | 'player' | 'stats') => void
		onImport?: () => void
		onAddRelease?: () => void
		onSettings?: () => void
		onCloudSync?: () => void
		onDevTools?: () => void
		onOpenDuplicates?: () => void
		onOpenUpgrader?: () => void
	}

	let {
		activeView,
		onViewChange,
		onImport,
		onAddRelease,
		onSettings,
		onCloudSync,
		onDevTools,
		onOpenDuplicates,
		onOpenUpgrader,
	}: Props = $props()
	let syncingMik = $state(false)

	onMount(() => {
		upgraderStore.loadCount()
		let unlisten: (() => void) | undefined
		listen('upgrades-updated', () => {
			upgraderStore.loadCount()
		}).then((un) => {
			unlisten = un
		})
		return () => {
			if (unlisten) unlisten()
		}
	})

	$effect(() => {
		// Periodically refresh duplicate counts when library changes
		if ($libraryStore.tracks.length > 0) {
			duplicateStore.loadCount()
		}
	})

	async function handleSyncMik() {
		if (syncingMik) return
		syncingMik = true
		try {
			const res = await libraryApi.syncFromMikDatabase()
			await libraryStore.reloadWithCurrentFilter()
			await duplicateStore.loadCount()
			await upgraderStore.loadCount()
			const parts: string[] = []
			if (res.added > 0) parts.push(`${res.added} importé${res.added > 1 ? 's' : ''}`)
			if (res.updated > 0) parts.push(`${res.updated} mis à jour`)
			if (res.removed > 0) parts.push(`${res.removed} supprimé${res.removed > 1 ? 's' : ''}`)
			const detail = parts.length > 0 ? parts.join(', ') : 'À jour'
			toastStore.success(
				`Bibliothèque Mixed In Key synchronisée : ${detail} (${res.total} au total)`
			)
		} catch (err) {
			console.error('MIK DB Sync error:', err)
			toastStore.error('Erreur lors de la synchronisation avec la bibliothèque Mixed In Key')
		} finally {
			syncingMik = false
		}
	}

	let exportingXml = $state(false)
	async function handleExportRekordboxXml() {
		if (exportingXml) return
		const path = await withNativeDialog(() =>
			save({
				defaultPath: 'rekordbox.xml',
				filters: [{ name: 'Pioneer Rekordbox XML', extensions: ['xml'] }],
			})
		)
		if (!path) return

		exportingXml = true
		try {
			const count = await exportApi.exportRekordboxXml(path)
			toastStore.success(`Export Rekordbox XML réussi : ${count} morceaux exportés avec grilles, clés et cues MIK !`)
		} catch (err) {
			const message = err instanceof Error ? err.message : String(err)
			toastStore.error(`Échec de l'export Rekordbox : ${message}`)
		} finally {
			exportingXml = false
		}
	}
</script>

<div class="flex flex-1 items-center justify-end gap-2 rounded-bl-md py-4 pr-3 pl-4">
	<Tooltip text="Synchronisé en direct avec Mixed In Key 11 Pro (Cliquer pour forcer)" position="bottom" delay={250}>
		<button
			type="button"
			class="group flex items-center gap-2 rounded-lg border border-sky-500/30 bg-surface-2/80 px-2.5 py-1 text-xs transition-all hover:border-sky-400/60 hover:bg-sky-950/40 active:scale-95 hover:cursor-pointer shadow-sm"
			onclick={handleSyncMik}
			disabled={syncingMik}
		>
			<MixedInKeyLogo variant="full" size="sm" showPro animated={syncingMik} />
			<div class="h-3 w-px bg-stroke"></div>
			<Icon name="refresh-cw" class="h-3 w-3 text-sky-400 {syncingMik ? 'animate-spin' : 'group-hover:rotate-180 transition-transform duration-300'}" />
		</button>
	</Tooltip>

	<!-- Duplicate Killer Button with Badge -->
	<Tooltip
		text={$duplicateGroupCount > 0
			? `Duplicate Killer : ${$duplicateGroupCount} groupe${$duplicateGroupCount > 1 ? 's' : ''} de doublons (${$duplicateTrackCount} morceaux)`
			: 'Duplicate Killer : Gestion des doublons'}
		position="bottom"
		delay={250}
	>
		<button
			type="button"
			class="relative flex h-8 w-8 items-center justify-center rounded-lg border transition-all hover:cursor-pointer shadow-sm active:scale-95
			{$duplicateGroupCount > 0
				? 'border-amber-500/40 bg-amber-950/30 text-amber-300 hover:border-amber-400 hover:bg-amber-900/40'
				: 'border-stroke bg-surface-2/80 text-text-secondary hover:border-stroke-strong hover:bg-surface-3 hover:text-text-primary'}"
			onclick={onOpenDuplicates}
			aria-label="Gestion des doublons"
		>
			<Icon name="clone" class="h-4 w-4 {$duplicateGroupCount > 0 ? 'text-amber-400' : 'text-text-secondary'}" />
			{#if $duplicateGroupCount > 0}
				<span class="absolute -bottom-1 -right-1 min-w-4 h-4 px-1 rounded-full bg-red-500 text-white font-bold text-[10px] font-mono flex items-center justify-center shadow-md border-2 border-surface-0">
					{$duplicateGroupCount}
				</span>
			{/if}
		</button>
	</Tooltip>

	<!-- Beatport Quality Upgrader Button with Emerald Badge (Positioned between Duplicates and Discovery) -->
	<Tooltip
		text={$upgraderMatchCount > 0
			? `Beatport Quality Upgrader : ${$upgraderMatchCount} morceau${$upgraderMatchCount > 1 ? 'x' : ''} améliorable${$upgraderMatchCount > 1 ? 's' : ''} en FLAC Lossless`
			: 'Beatport Quality Upgrader : Améliorer la qualité en FLAC Lossless'}
		position="bottom"
		delay={250}
	>
		<button
			type="button"
			class="relative flex h-8 w-8 items-center justify-center rounded-lg border transition-all hover:cursor-pointer shadow-sm active:scale-95
			{$upgraderMatchCount > 0
				? 'border-emerald-500/40 bg-emerald-950/30 text-emerald-300 hover:border-emerald-400 hover:bg-emerald-900/40'
				: 'border-stroke bg-surface-2/80 text-text-secondary hover:border-stroke-strong hover:bg-surface-3 hover:text-text-primary'}"
			onclick={onOpenUpgrader}
			aria-label="Beatport Quality Upgrader"
		>
			<Icon name="sparkles" class="h-4 w-4 {$upgraderMatchCount > 0 ? 'text-emerald-400' : 'text-text-secondary'}" />
			{#if $upgraderMatchCount > 0}
				<span class="absolute -bottom-1 -right-1 min-w-4 h-4 px-1 rounded-full bg-emerald-500 text-white font-bold text-[10px] font-mono flex items-center justify-center shadow-md border-2 border-surface-0">
					{$upgraderMatchCount}
				</span>
			{/if}
		</button>
	</Tooltip>

	<!-- Discovery (Découvertes) Button -->
	<Tooltip text={$translate('nav.discovery')} position="bottom" delay={250}>
		<button
			type="button"
			class="relative flex h-8 w-8 items-center justify-center rounded-lg border transition-all hover:cursor-pointer shadow-sm active:scale-95
			{activeView === 'discovery'
				? 'border-brand-primary/60 bg-brand-primary/20 text-brand-primary'
				: 'border-stroke bg-surface-2/80 text-text-secondary hover:border-stroke-strong hover:bg-surface-3 hover:text-text-primary'}"
			onclick={() => onViewChange?.('discovery')}
			aria-label={$translate('nav.discovery')}
		>
			<Icon name="globe" class="h-4 w-4 {activeView === 'discovery' ? 'text-brand-primary' : 'text-text-secondary'}" />
		</button>
	</Tooltip>

	<!-- Stats Button -->
	<Tooltip text="Crate Pulse & Statistiques" position="bottom" delay={250}>
		<button
			type="button"
			class="relative flex h-8 w-8 items-center justify-center rounded-lg border transition-all hover:cursor-pointer shadow-sm active:scale-95
			{activeView === 'stats'
				? 'border-brand-primary/60 bg-brand-primary/20 text-brand-primary'
				: 'border-stroke bg-surface-2/80 text-text-secondary hover:border-stroke-strong hover:bg-surface-3 hover:text-text-primary'}"
			onclick={() => onViewChange?.('stats')}
			aria-label="Statistiques"
		>
			<Icon name="chart" class="h-4 w-4 {activeView === 'stats' ? 'text-brand-primary' : 'text-text-secondary'}" />
		</button>
	</Tooltip>

	<!-- Rekordbox XML Export Button -->
	<Tooltip text="Exporter la collection en Rekordbox XML (Prêt pour CDJ / Rekordbox)" position="bottom" delay={250}>
		<button
			type="button"
			class="relative flex h-8 w-8 items-center justify-center rounded-lg border border-stroke bg-surface-2/80 text-text-secondary hover:border-cyan-500/50 hover:bg-cyan-950/30 hover:text-cyan-400 transition-all hover:cursor-pointer shadow-sm active:scale-95 disabled:opacity-50"
			onclick={handleExportRekordboxXml}
			disabled={exportingXml}
			aria-label="Exporter vers Rekordbox XML"
		>
			<Icon name={exportingXml ? 'loader' : 'download'} class="h-4 w-4 text-cyan-400 {exportingXml ? 'animate-spin' : ''}" />
		</button>
	</Tooltip>

	{#if onAddRelease}
		<Button variant="primary" size="sm" onclick={onAddRelease}>
			<Icon name="plus" class="mr-1.5 h-4 w-4" />
			{$translate('discovery.addRelease')}
		</Button>
	{:else if onImport}
		<Button variant="primary" size="sm" onclick={onImport}>
			<Icon name="upload" class="mr-1.5 h-4 w-4" />
			{$translate('library.importTracks')}
		</Button>
	{/if}
	<SyncStatusIndicator onclick={onCloudSync} />
	{#if $isDev}
		<Tooltip text={$translate('common.developerTools')} position="bottom" delay={250}>
			<IconButton icon="terminal" iconClass="h-5 w-5" onclick={onDevTools} />
		</Tooltip>
	{/if}
	<Tooltip text={$translate('settings.title')} position="bottom" delay={250}>
		<IconButton icon="settings" iconClass="h-5 w-5" onclick={onSettings} />
	</Tooltip>
</div>
