<script lang="ts">
	import { toErrorMessage } from '$shared/utils/errors'
	import { onMount } from 'svelte'
	import { listen } from '@tauri-apps/api/event'
	import { Button, IconButton, Tooltip, MixedInKeyLogo, Spinner } from '$lib/components/common'
	import Icon from '$lib/components/common/Icon.svelte'
	import { SyncStatusIndicator } from '$lib/components/cloud-sync'
	import { isDev, libraryStore, language } from '$lib/stores'
	import { duplicateStore, duplicateGroupCount, duplicateTrackCount } from '$shared/stores/duplicate'
	import { upgraderStore, upgraderMatchCount } from '$shared/stores/upgrader'
	import { toastStore } from '$shared/stores/toast'
	import { translate } from '$shared/i18n'
	import * as libraryApi from '$shared/api/library'
	import * as exportApi from '$shared/api/export'
	import { save } from '@tauri-apps/plugin-dialog'
	import { withNativeDialog } from '$shared/utils'
	import { formatNumber } from '$shared/utils/format'

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
		onOpenDiscrepancyReport?: () => void
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
		onOpenDiscrepancyReport,
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
			if (res.added > 0) parts.push($translate('library.toast.mikSyncAdded', { values: { count: res.added } }))
			if (res.updated > 0) parts.push($translate('library.toast.mikSyncUpdated', { values: { count: res.updated } }))
			if (res.removed > 0) parts.push($translate('library.toast.mikSyncRemoved', { values: { count: res.removed } }))
			const detail = parts.length > 0 ? parts.join(', ') : $translate('library.toast.mikSyncUpToDate')
			toastStore.success(
				$translate('library.toast.mikSyncSuccess', {
					values: { detail, total: formatNumber(res.total, $language) },
				})
			)
		} catch (err) {
			console.error('MIK DB Sync error:', err)
			toastStore.error($translate('library.toast.mikSyncError'))
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
			toastStore.success($translate('export.toast.rekordboxXmlSuccess', { values: { count } }))
		} catch (err) {
			const message = toErrorMessage(err, $translate('common.unknownError'))
			toastStore.error($translate('export.toast.rekordboxXmlFailed', { values: { error: message } }))
		} finally {
			exportingXml = false
		}
	}
</script>

<div
	class="flex flex-1 items-center justify-end gap-1.5 rounded-bl-md py-4 pr-3 pl-2 min-[1280px]:gap-2 min-[1280px]:pl-4"
>
	<Tooltip text={$translate('nav.toolbar.mikSync')} position="bottom" delay={250}>
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
			{#if syncingMik}
				<Spinner icon="refresh-cw" color="current" class="h-3 w-3 text-sky-400" />
			{:else}
				<Icon name="refresh-cw" class="h-3 w-3 text-sky-400 transition-transform duration-300 group-hover:rotate-180" />
			{/if}
		</button>
	</Tooltip>

	<!-- Duplicate Killer Button with Badge -->
	<Tooltip
		text={$duplicateGroupCount > 0
			? $translate('nav.toolbar.duplicatesFound', {
					values: { groups: $duplicateGroupCount, tracks: $duplicateTrackCount },
				})
			: $translate('nav.toolbar.duplicatesIdle')}
		position="bottom"
		delay={250}
	>
		<button
			type="button"
			class="relative flex h-8 w-8 items-center justify-center rounded-lg border shadow-sm transition-all hover:cursor-pointer active:scale-95
			{$duplicateGroupCount > 0
				? 'border-warning-tint/40 bg-warning-wash/30 text-warning-text hover:border-warning-text hover:bg-warning-wash-hover/40'
				: 'border-stroke bg-surface-2/80 text-text-secondary hover:border-stroke-strong hover:bg-surface-3 hover:text-text-primary'}"
			onclick={onOpenDuplicates}
			aria-label={$translate('nav.toolbar.duplicates')}
		>
			<Icon name="clone" class="h-4 w-4 {$duplicateGroupCount > 0 ? 'text-warning-text' : 'text-text-secondary'}" />
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
			? $translate('nav.toolbar.upgraderFound', { values: { count: $upgraderMatchCount } })
			: $translate('nav.toolbar.upgraderIdle')}
		position="bottom"
		delay={250}
	>
		<button
			type="button"
			class="relative flex h-8 w-8 items-center justify-center rounded-lg border border-stroke bg-surface-2/80 text-text-secondary shadow-sm transition-all hover:cursor-pointer hover:border-stroke-strong hover:bg-surface-3 hover:text-text-primary active:scale-95"
			onclick={onOpenUpgrader}
			aria-label="Beatport Quality Upgrader"
		>
			<Icon name="sparkles" class="h-4 w-4 text-text-secondary" />
			{#if $upgraderMatchCount > 0}
				<span
					class="absolute -right-1 -bottom-1 flex h-4 min-w-4 items-center justify-center rounded-full border-2 border-surface-0 bg-beatport-tint px-1 font-mono text-[10px] font-bold text-white shadow-md"
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
	<Tooltip text={$translate('nav.toolbar.statsTooltip')} position="bottom" delay={250}>
		<button
			type="button"
			class="relative flex h-8 w-8 items-center justify-center rounded-lg border shadow-sm transition-all hover:cursor-pointer active:scale-95
			{activeView === 'stats'
				? 'border-brand-primary/60 bg-brand-primary/20 text-brand-primary'
				: 'border-stroke bg-surface-2/80 text-text-secondary hover:border-stroke-strong hover:bg-surface-3 hover:text-text-primary'}"
			onclick={() => onViewChange?.('stats')}
			aria-label={$translate('nav.toolbar.stats')}
		>
			<Icon name="chart" class="h-4 w-4 {activeView === 'stats' ? 'text-brand-primary' : 'text-text-secondary'}" />
		</button>
	</Tooltip>

	<!-- Rekordbox XML Export Button -->
	<Tooltip text={$translate('nav.toolbar.rekordboxXmlTooltip')} position="bottom" delay={250}>
		<button
			type="button"
			class="relative flex h-8 w-8 items-center justify-center rounded-lg border border-stroke bg-surface-2/80 text-text-secondary shadow-sm transition-all hover:cursor-pointer hover:border-stroke-strong hover:bg-surface-3 hover:text-text-primary active:scale-95 disabled:opacity-50"
			onclick={handleExportRekordboxXml}
			disabled={exportingXml}
			aria-label={$translate('nav.toolbar.rekordboxXml')}
		>
			{#if exportingXml}
				<Spinner icon="loader" color="current" class="h-4 w-4 text-text-secondary" />
			{:else}
				<Icon name="download" class="h-4 w-4 text-text-secondary" />
			{/if}
		</button>
	</Tooltip>

	<!-- Discrepancy Report Button -->
	<Tooltip text={$translate('nav.toolbar.discrepancyReportTooltip')} position="bottom" delay={250}>
		<button
			type="button"
			class="relative flex h-8 w-8 items-center justify-center rounded-lg border border-stroke bg-surface-2/80 text-text-secondary shadow-sm transition-colors hover:cursor-pointer hover:border-stroke-strong hover:bg-surface-3 hover:text-text-primary active:scale-95"
			onclick={onOpenDiscrepancyReport}
			aria-label={$translate('nav.toolbar.discrepancyReport')}
		>
			<Icon name="clipboard" class="h-4 w-4 text-text-secondary" />
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
			<IconButton
				icon="terminal"
				iconClass="h-5 w-5"
				ariaLabel={$translate('common.developerTools')}
				onclick={onDevTools}
			/>
		</Tooltip>
	{/if}
	<Tooltip text={$translate('settings.title')} position="bottom" delay={250}>
		<IconButton icon="settings" iconClass="h-5 w-5" ariaLabel={$translate('settings.title')} onclick={onSettings} />
	</Tooltip>
</div>
