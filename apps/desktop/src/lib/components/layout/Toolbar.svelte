<script lang="ts">
	import { toErrorMessage } from '$shared/utils/errors'
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

	// Refresh the duplicate badge only when the number of tracks changes (imports, deletions);
	// edits are covered by the backend `duplicates-updated` event handled in the layout.
	const trackCount = $derived($libraryStore.tracks.length)
	$effect(() => {
		if (trackCount > 0) {
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
			toastStore.success(`Bibliothèque Mixed In Key synchronisée : ${detail} (${res.total} au total)`)
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
			const message = toErrorMessage(err, 'Unknown error')
			toastStore.error(`Échec de l'export Rekordbox : ${message}`)
		} finally {
			exportingXml = false
		}
	}
</script>

<div
	class="flex flex-1 items-center justify-end gap-1.5 rounded-bl-md py-4 pr-3 pl-2 min-[1280px]:gap-2 min-[1280px]:pl-4"
>
	<Tooltip text="Synchronisé en direct avec Mixed In Key (Cliquer pour forcer)" position="bottom" delay={250}>
		<button
			type="button"
			class="group flex items-center gap-2 rounded-lg border border-sky-500/30 bg-surface-2/80 px-2.5 py-1 text-xs shadow-sm transition-all hover:cursor-pointer hover:border-sky-400/60 hover:bg-sky-950/40 active:scale-95"
			onclick={handleSyncMik}
			disabled={syncingMik}
		>
			<span class="hidden min-[1400px]:inline-flex">
				<MixedInKeyLogo variant="full" size="sm" animated={syncingMik} />
			</span>
			<span class="inline-flex min-[1400px]:hidden" aria-label="Mixed In Key">
				<MixedInKeyLogo variant="icon" size="sm" animated={syncingMik} />
			</span>
			<div class="h-3 w-px bg-stroke"></div>
			<Icon
				name="refresh-cw"
				class="h-3 w-3 text-sky-400 {syncingMik
					? 'animate-spin'
					: 'transition-transform duration-300 group-hover:rotate-180'}"
			/>
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
			class="relative flex h-8 w-8 items-center justify-center rounded-lg border shadow-sm transition-all hover:cursor-pointer active:scale-95
			{$duplicateGroupCount > 0
				? 'border-amber-500/40 bg-amber-950/30 text-amber-300 hover:border-amber-400 hover:bg-amber-900/40'
				: 'border-stroke bg-surface-2/80 text-text-secondary hover:border-stroke-strong hover:bg-surface-3 hover:text-text-primary'}"
			onclick={onOpenDuplicates}
			aria-label="Gestion des doublons"
		>
			<Icon name="clone" class="h-4 w-4 {$duplicateGroupCount > 0 ? 'text-amber-400' : 'text-text-secondary'}" />
			{#if $duplicateGroupCount > 0}
				<span
					class="absolute -right-1 -bottom-1 flex h-4 min-w-4 items-center justify-center rounded-full border-2 border-surface-0 bg-red-500 px-1 font-mono text-[10px] font-bold text-white shadow-md"
				>
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
			class="relative flex h-8 w-8 items-center justify-center rounded-lg border shadow-sm transition-all hover:cursor-pointer active:scale-95
			{$upgraderMatchCount > 0
				? 'border-emerald-500/40 bg-emerald-950/30 text-emerald-300 hover:border-emerald-400 hover:bg-emerald-900/40'
				: 'border-stroke bg-surface-2/80 text-text-secondary hover:border-stroke-strong hover:bg-surface-3 hover:text-text-primary'}"
			onclick={onOpenUpgrader}
			aria-label="Beatport Quality Upgrader"
		>
			<Icon name="sparkles" class="h-4 w-4 {$upgraderMatchCount > 0 ? 'text-emerald-400' : 'text-text-secondary'}" />
			{#if $upgraderMatchCount > 0}
				<span
					class="absolute -right-1 -bottom-1 flex h-4 min-w-4 items-center justify-center rounded-full border-2 border-surface-0 bg-emerald-500 px-1 font-mono text-[10px] font-bold text-white shadow-md"
				>
					{$upgraderMatchCount}
				</span>
			{/if}
		</button>
	</Tooltip>

	<!-- Discovery (Découvertes) Button -->
	<Tooltip text={$translate('nav.discovery')} position="bottom" delay={250}>
		<button
			type="button"
			class="relative flex h-8 w-8 items-center justify-center rounded-lg border shadow-sm transition-all hover:cursor-pointer active:scale-95
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
			class="relative flex h-8 w-8 items-center justify-center rounded-lg border shadow-sm transition-all hover:cursor-pointer active:scale-95
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
			class="relative flex h-8 w-8 items-center justify-center rounded-lg border border-stroke bg-surface-2/80 text-text-secondary shadow-sm transition-all hover:cursor-pointer hover:border-cyan-500/50 hover:bg-cyan-950/30 hover:text-cyan-400 active:scale-95 disabled:opacity-50"
			onclick={handleExportRekordboxXml}
			disabled={exportingXml}
			aria-label="Exporter vers Rekordbox XML"
		>
			<Icon
				name={exportingXml ? 'loader' : 'download'}
				class="h-4 w-4 text-cyan-400 {exportingXml ? 'animate-spin' : ''}"
			/>
		</button>
	</Tooltip>

	{#if onAddRelease}
		<Button variant="primary" size="sm" onclick={onAddRelease} aria-label={$translate('discovery.addRelease')}>
			<Icon name="plus" class="h-4 w-4 min-[1280px]:mr-1.5" />
			<span class="hidden min-[1280px]:inline">{$translate('discovery.addRelease')}</span>
		</Button>
	{:else if onImport}
		<Button variant="primary" size="sm" onclick={onImport} aria-label={$translate('library.importTracks')}>
			<Icon name="upload" class="h-4 w-4 min-[1280px]:mr-1.5" />
			<span class="hidden min-[1280px]:inline">{$translate('library.importTracks')}</span>
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
