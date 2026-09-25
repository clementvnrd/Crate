<script lang="ts">
	import '../style.css'
	import type { Snippet } from 'svelte'
	import type { Language, Playlist, TagCategory, Tag, TagSelectionState, UsbDevice } from '$shared/types'
	import ToastContainer from '$lib/components/common/ToastContainer.svelte'
	import CrashScreen from '$lib/components/common/CrashScreen.svelte'
	import SplashScreen from '$lib/components/common/SplashScreen.svelte'
	import { AboutDialog, OnboardingWizard } from '$lib/components/onboarding'
	import { WizardTour } from '$lib/components/wizard'
	import { onMount } from 'svelte'
	import { get } from 'svelte/store'
	import { PUBLIC_APP_VERSION } from '$env/static/public'
	import { isDev } from '$lib/stores/app'
	import { settingsStore, hasCompletedOnboarding, hasCompletedWizard } from '$shared/stores/settings'
	import { splashVisible } from '$lib/stores/splash'
	import { useGlobalErrorHandler, hasAudioDrag } from '$lib/hooks'
	import { initializeI18n, translate } from '$shared/i18n'
	import { Sidebar, Toolbar } from '$lib/components/layout'
	import { Player } from '$lib/components/player'
	import { ResizeHandle, Icon, Text } from '$lib/components/common'
	import {
		playlistsStore,
		tagsStore,
		uiStore,
		uiLayoutStore,
		activeView,
		selectedTrackIds,
		selectedReleaseIds,
		visibleDevices,
		computeTagStates,
		releaseCount,
		trackCount,
		sortedReleases,
		displayedTracks,
		pageActions,
		libraryStore,
	} from '$lib/stores'
	import { discoveryPlaylistStore } from '$shared/stores/discoveryPlaylist'
	import { duplicateStore } from '$shared/stores/duplicate'
	import { upgraderStore } from '$shared/stores/upgrader'
	import { listen } from '@tauri-apps/api/event'
	import { setMenuItemEnabled, setOnboardingItemsEnabled } from '$shared/api/app'
	import { computeDiscoveryTagStates } from '$shared/utils/tagComputation'
	import * as standaloneApi from '$shared/api/standalone'
	import { toastStore } from '$shared/stores/toast'
	import { playerStore, recentTracksStore } from '$lib/stores'

	interface Props {
		children: Snippet
	}

	let { children }: Props = $props()
	let i18nReady = $state(false)
	let splashVersion = PUBLIC_APP_VERSION
	let onboardingComplete = $state(false)
	let showOnboarding = $derived(!$splashVisible && !$hasCompletedOnboarding && !onboardingComplete)
	let showAboutDialog = $state(false)
	let showWizardTour = $state(false)

	// =========================================================================
	// Layout State (subscribed from stores)
	// =========================================================================

	let playlists = $state<Playlist[]>([])
	let tagCategories = $state<TagCategory[]>([])
	let devices = $state<UsbDevice[]>([])
	let selectedPlaylistId = $state<string | null>(null)
	let selectedFolderId = $state<string | null>(null)
	let selectedTagIds = $state<string[]>([])
	let sidebarWidth = $state(240)

	// Tag toggle state
	let tagStates = $state<Map<string, TagSelectionState>>(new Map())
	let tagCounts = $state<Map<string, number>>(new Map())

	// =========================================================================
	// Store Subscriptions
	// =========================================================================

	$effect(() => {
		const unsubPlaylists = playlistsStore.subscribe((state) => {
			playlists = state.playlists
		})
		const unsubTags = tagsStore.subscribe((state) => {
			tagCategories = state.categories
		})
		const unsubUI = uiStore.subscribe((state) => {
			selectedPlaylistId = state.selectedPlaylistId
			selectedFolderId = state.selectedFolderId
			selectedTagIds = state.viewFilters[state.activeView].selectedTagIds
		})
		const unsubLayout = uiLayoutStore.subscribe((state) => {
			sidebarWidth = state.sidebarWidth
		})
		const unsubDevices = visibleDevices.subscribe((visibleDevicesList) => {
			devices = visibleDevicesList
		})

		return () => {
			unsubPlaylists()
			unsubTags()
			unsubUI()
			unsubLayout()
			unsubDevices()
		}
	})

	// =========================================================================
	// Effects (migrated from +page.svelte)
	// =========================================================================

	// Compute tag states when selection or tracks/releases change
	$effect(() => {
		if ($activeView === 'discovery') {
			const result = computeDiscoveryTagStates(tagCategories, $sortedReleases, $selectedReleaseIds)
			tagStates = result.states
			tagCounts = result.counts
		} else {
			const result = computeTagStates(tagCategories, $displayedTracks, $selectedTrackIds)
			tagStates = result.states
			tagCounts = result.counts
		}
	})

	// Clear recently toggled tags when selection changes
	let previousSelectedIds = $state<Set<string>>(new Set())
	$effect(() => {
		const currentIds = $activeView === 'discovery' ? $selectedReleaseIds : $selectedTrackIds
		if (currentIds.size !== previousSelectedIds.size || ![...currentIds].every((id) => previousSelectedIds.has(id))) {
			uiStore.clearAllRecentlyToggledTags()
			previousSelectedIds = new Set(currentIds)
		}
	})

	// Clear discovery playlist releases when no playlist is selected
	$effect(() => {
		if (!selectedPlaylistId) {
			discoveryPlaylistStore.clearReleases()
		}
	})

	// Enable/disable Refresh Metadata menu item based on view and selection
	$effect(() => {
		setMenuItemEnabled('refresh_metadata', $activeView === 'discovery' && $selectedReleaseIds.size > 0)
	})

	// Disable menu items during onboarding wizard and sync state to uiStore
	$effect(() => {
		uiStore.setOnboarding(showOnboarding)
		setOnboardingItemsEnabled(!showOnboarding)
	})

	// Clear tree multi-selection when navigation changes
	let prevNavPlaylistId: string | null = null
	let prevNavFolderId: string | null = null
	$effect(() => {
		const pId = selectedPlaylistId
		const fId = selectedFolderId
		if (pId !== prevNavPlaylistId || fId !== prevNavFolderId) {
			prevNavPlaylistId = pId
			prevNavFolderId = fId
			uiLayoutStore.clearSelectedTreeIds()
		}
	})

	// Prune discovery selection to only include visible releases
	$effect(() => {
		if ($activeView !== 'discovery') return
		const visibleIds = new Set($sortedReleases.map((r) => r.id))
		const currentSelection = $selectedReleaseIds
		if (currentSelection.size === 0) return
		const pruned = new Set([...currentSelection].filter((id) => visibleIds.has(id)))
		if (pruned.size < currentSelection.size) {
			uiStore.setSelectedReleases(pruned)
		}
	})

	// Prune library selection to only include visible tracks
	$effect(() => {
		if ($activeView === 'discovery') return
		const visibleIds = new Set($displayedTracks.map((t) => t.id))
		const currentSelection = $selectedTrackIds
		if (currentSelection.size === 0) return
		const pruned = new Set([...currentSelection].filter((id) => visibleIds.has(id)))
		if (pruned.size < currentSelection.size) {
			uiStore.setSelectedTracks(pruned)
		}
	})

	// Auto-start wizard tour after onboarding completes for new users
	$effect(() => {
		if (!$splashVisible && !showOnboarding && onboardingComplete && !$hasCompletedWizard) {
			setTimeout(() => {
				showWizardTour = true
			}, 600)
		}
	})

	// =========================================================================
	// Derived State
	// =========================================================================

	const contextPlaylists = $derived(playlists.filter((p) => p.context === $activeView))

	// =========================================================================
	// Handlers
	// =========================================================================

	function handleSidebarResize(delta: number) {
		uiLayoutStore.setSidebarWidth(sidebarWidth + delta)
	}

	function handlePlaylistItemClick(playlist: Playlist, newSelectedIds: Set<string>, isModifierClick: boolean) {
		uiLayoutStore.setSelectedTreeIds(newSelectedIds)
	}

	function handlePlaylistContextMenu(e: MouseEvent, playlist: Playlist) {
		uiLayoutStore.setContextMenuPlaylistId(playlist.id)
		$pageActions?.getContextMenuOrchestrator()?.openPlaylistMenu(e, playlist, 'tree')
	}

	function handlePlaylistMultiContextMenu(e: MouseEvent, playlists: Playlist[]) {
		uiLayoutStore.clearContextMenuPlaylistId()
		$pageActions?.getContextMenuOrchestrator()?.openPlaylistMenu(e, playlists, 'tree')
	}

	function handleTagContextMenu(e: MouseEvent, tag: Tag, category: TagCategory) {
		$pageActions?.getContextMenuOrchestrator()?.openTagMenu(e, { type: 'tag', tag, category })
	}

	function handleCategoryContextMenu(e: MouseEvent, category: TagCategory) {
		$pageActions?.getContextMenuOrchestrator()?.openTagMenu(e, { type: 'category', category })
	}

	function handleDeviceContextMenu(e: MouseEvent, device: UsbDevice) {
		$pageActions?.getContextMenuOrchestrator()?.openDeviceMenu(e, device)
	}

	// =========================================================================
	// Initialization
	// =========================================================================

	onMount(() => {
		async function init() {
			try {
				const cachedLanguage = localStorage.getItem('crate-language') as Language | null
				await initializeI18n(cachedLanguage)
			} catch (err) {
				console.warn('i18n initialization warning:', err)
			} finally {
				i18nReady = true
			}
			try {
				await settingsStore.load()
			} catch (err) {
				console.warn('settingsStore.load warning:', err)
			}
		}
		init()

		const cleanupErrorHandler = useGlobalErrorHandler()

		const dragoverHandler = (e: DragEvent) => {
			if (hasAudioDrag) {
				e.preventDefault()
			}
			e.stopPropagation()
		}

		const dropHandler = (e: DragEvent) => {
			e.preventDefault()
			e.stopPropagation()
		}

		window.addEventListener('dragover', dragoverHandler)
		window.addEventListener('drop', dropHandler)

		const contextMenuHandler = (e: MouseEvent) => {
			if (!get(isDev)) {
				e.preventDefault()
			}
		}
		document.addEventListener('contextmenu', contextMenuHandler)

		// Listen for menu actions handled at the layout level
		let unlistenMenuAction: (() => void) | null = null
		listen<string>('menu-action', (event) => {
			if (event.payload === 'about' && showOnboarding) {
				showAboutDialog = true
			}
			if (event.payload === 'feature_tour' && !showOnboarding) {
				showWizardTour = true
			}
		}).then((unlisten) => {
			unlistenMenuAction = unlisten
		})

		// Real-time live sync with Mixed In Key database
		let unlistenMikSync: (() => void) | null = null
		listen('mik-database-synced', () => {
			libraryStore.loadTracks()
		}).then((unlisten) => {
			unlistenMikSync = unlisten
		})

		// Real-time duplicate count updates
		let unlistenDuplicates: (() => void) | null = null
		listen('duplicates-updated', () => {
			duplicateStore.loadCount()
		}).then((unlisten) => {
			unlistenDuplicates = unlisten
		})

		// Initial duplicate count load
		duplicateStore.loadCount()

		// Real-time upgrader count updates
		let unlistenUpgrades: (() => void) | null = null
		listen('upgrades-updated', () => {
			upgraderStore.loadCount()
		}).then((unlisten) => {
			unlistenUpgrades = unlisten
		})

		// Initial upgrader count load
		upgraderStore.loadCount()

		// Mixed In Key sync runs in the backend (startup + file watcher) and emits
		// `mik-database-synced`, handled above: no extra sync on focus.

		// Standalone file opener (macOS Open With / Cold Start / Hot Event)
		async function handleOpenFile(filePath: string) {
			try {
				const track = await standaloneApi.readStandaloneTrack(filePath)
				uiStore.setActiveView('player')
				await playerStore.playStandalone(track, track.is_in_library)
				if (!track.is_in_library) {
					await recentTracksStore.addTrack(track)
				}
			} catch (err) {
				console.error('Error opening standalone file:', err)
				toastStore.error(`Impossible d'ouvrir le fichier audio : ${filePath.split('/').pop()}`)
			}
		}

		// Cold start check
		standaloneApi.getStartupFile().then((path) => {
			if (path) {
				handleOpenFile(path)
			}
		})

		// Live open-file listener
		let unlistenOpenFile: (() => void) | null = null
		listen<string>('open-file', (event) => {
			if (event.payload) {
				handleOpenFile(event.payload)
			}
		}).then((unlisten) => {
			unlistenOpenFile = unlisten
		})

		return () => {
			cleanupErrorHandler()
			unlistenMenuAction?.()
			unlistenMikSync?.()
			unlistenDuplicates?.()
			unlistenUpgrades?.()
			unlistenOpenFile?.()
			window.removeEventListener('dragover', dragoverHandler)
			window.removeEventListener('drop', dropHandler)
			document.removeEventListener('contextmenu', contextMenuHandler)
		}
	})
</script>

<SplashScreen show={$splashVisible} version={splashVersion} />

{#if showOnboarding}
	<OnboardingWizard
		onComplete={() => {
			onboardingComplete = true
			settingsStore.completeOnboarding()
		}}
	/>
	<AboutDialog open={showAboutDialog} onClose={() => (showAboutDialog = false)} />
{/if}

<div class="flex h-screen w-screen flex-col overflow-hidden bg-surface-0 text-text-primary">
	{#if i18nReady}
		<div
			class="flex h-full flex-col transition-opacity duration-300"
			style="opacity: {$splashVisible || showOnboarding ? 0 : 1}; pointer-events: {$splashVisible || showOnboarding
				? 'none'
				: 'auto'}"
		>
			<!-- Unified Header: Logo + Toolbar -->
			<div class="relative flex rounded-br bg-surface-1">
				<div class="flex flex-shrink-0 items-center justify-center gap-2" style="width: {sidebarWidth}px">
					<div
						class="h-6 w-6 bg-brand-primary"
						style="-webkit-mask-image: url('/crate-logo.svg'); -webkit-mask-size: contain; -webkit-mask-repeat: no-repeat; -webkit-mask-position: center; mask-image: url('/crate-logo.svg'); mask-size: contain; mask-repeat: no-repeat; mask-position: center;"
					></div>
					<Text variant="header-1" as="span" weight="bold">Crate</Text>
					<span
						class="rounded bg-brand-primary/20 border border-brand-primary/40 px-1.5 py-0.5 text-[11px] font-mono font-bold text-brand-primary tracking-wide"
						title="App Build Number"
					>
						Build 57
					</span>
					{#if $isDev}
						<span class="rounded bg-amber-500/20 px-1.5 py-0.5 text-xs font-medium text-amber-500">DEV</span>
					{/if}
				</div>

				<!-- Segmented control (absolutely centered in full window) -->
				<div class="pointer-events-none absolute inset-0 flex items-center justify-center">
					<div
						id="wizard-view-switcher"
						class="pointer-events-auto relative inline-grid grid-cols-3 items-center rounded-lg bg-surface-2 p-0.5"
					>
						<div
							class="absolute top-0.5 bottom-0.5 left-0.5 w-[calc(33.333%-2px)] rounded-md bg-surface-0 shadow-sm transition-transform duration-200 ease-out motion-reduce:transition-none"
							style="transform: translateX({$activeView === 'player' ? '0%' : $activeView === 'library' ? '100%' : '200%'})"
						></div>
						<button
							type="button"
							class="relative z-10 flex items-center justify-center gap-1.5 rounded-md px-3 py-1 text-center text-xs font-medium transition-colors {$activeView ===
							'player'
								? 'text-cyan-400 font-semibold'
								: 'text-text-tertiary hover:cursor-pointer hover:text-text-secondary'}"
							onclick={() => $pageActions?.handleViewChange('player')}
						>
							<Icon name="disc" class="h-3 w-3 {$activeView === 'player' ? 'text-cyan-400' : 'text-text-tertiary'}" />
							<span>Player</span>
						</button>
						<button
							type="button"
							class="relative z-10 rounded-md px-3 py-1 text-center text-xs font-medium transition-colors {$activeView ===
							'library'
								? 'text-text-primary font-semibold'
								: 'text-text-tertiary hover:cursor-pointer hover:text-text-secondary'}"
							onclick={() => $pageActions?.handleViewChange('library')}
						>
							{$translate('nav.library')}
						</button>
						<button
							type="button"
							class="relative z-10 flex items-center justify-center gap-1.5 rounded-md px-3 py-1 text-center text-xs font-medium transition-colors {$activeView ===
							'beatport'
								? 'text-emerald-500 dark:text-emerald-400 font-semibold'
								: 'text-text-tertiary hover:cursor-pointer hover:text-text-secondary'}"
							onclick={() => $pageActions?.handleViewChange('beatport')}
						>
							<Icon name="beatport" class="h-3 w-3 {$activeView === 'beatport' ? 'text-emerald-500 dark:text-emerald-400' : 'text-text-tertiary'}" />
							<span>Beatport</span>
						</button>
					</div>
				</div>

				<Toolbar
					activeView={$activeView}
					onViewChange={(v) => $pageActions?.handleViewChange(v)}
					onImport={$activeView === 'library' ? () => $pageActions?.trackController.handleImport() : undefined}
					onAddRelease={$activeView === 'discovery' ? () => $pageActions?.openAddReleaseModal() : undefined}
					onSettings={() => $pageActions?.getModalOrchestrator()?.openSettingsModal()}
					onCloudSync={() => $pageActions?.getModalOrchestrator()?.openSettingsModal('cloudSync')}
					onDevTools={() => $pageActions?.handleToggleDevTools()}
					onOpenDuplicates={() => $pageActions?.getModalOrchestrator()?.openDuplicateManagerModal()}
					onOpenUpgrader={() => $pageActions?.getModalOrchestrator()?.openBeatportUpgraderModal()}
				/>
			</div>

			<div class="relative flex flex-1 overflow-hidden bg-surface-1">
				{#if $activeView !== 'beatport' && $activeView !== 'player' && $activeView !== 'stats'}
					<!-- Left: Sidebar -->
					<div class="flex-shrink-0" style="width: {sidebarWidth}px">
						<Sidebar
							playlists={contextPlaylists}
							{tagCategories}
							{devices}
							{selectedPlaylistId}
							{selectedFolderId}
							contextMenuPlaylistId={$uiLayoutStore.contextMenuPlaylistId}
							{selectedTagIds}
							selectedTrackIds={$activeView === 'discovery' ? $selectedReleaseIds : $selectedTrackIds}
							selectedTreeIds={$uiLayoutStore.selectedTreeIds}
							{tagStates}
							{tagCounts}
							trackCount={$activeView === 'discovery' ? $releaseCount : $trackCount}
							showHeader={false}
							onLibraryClick={() => {
								uiLayoutStore.clearSelectedTreeIds()
								$pageActions?.playlistController.handleLibraryClick()
							}}
							onPlaylistSelect={(p) => $pageActions?.playlistController.handlePlaylistSelect(p)}
							onPlaylistItemClick={handlePlaylistItemClick}
							onPlaylistContextMenu={handlePlaylistContextMenu}
							onPlaylistMultiContextMenu={handlePlaylistMultiContextMenu}
							onPlaylistTreeContextMenu={(e) => $pageActions?.getContextMenuOrchestrator()?.openPlaylistTreeMenu(e)}
							onDeviceContextMenu={handleDeviceContextMenu}
							onCancelExport={() => $pageActions?.exportController.handleExportCancel()}
							onTagSelect={(tagId) => $pageActions?.tagController.selectTag(tagId)}
							onTagToggle={(tagId, state) => $pageActions?.tagController.toggleTagOnTracks(tagId, state)}
							onTagContextMenu={handleTagContextMenu}
							onCategoryContextMenu={handleCategoryContextMenu}
							onCreatePlaylist={() => $pageActions?.playlistController.handleCreatePlaylist()}
							onCreateSmartPlaylist={() => $pageActions?.playlistController.handleCreateSmartPlaylist($activeView)}
							onCreateFolder={() => $pageActions?.playlistController.handleCreateFolder()}
							onCreateCategory={() => $pageActions?.getModalOrchestrator()?.openCreateCategoryModal()}
							onCreateTag={(categoryId) => $pageActions?.getModalOrchestrator()?.openCreateTagModal(categoryId)}
							onTagsWhitespaceContextMenu={(e) => $pageActions?.getContextMenuOrchestrator()?.openTagsSidebarMenu(e)}
							onTracksDrop={(playlistId, trackIds) =>
								$pageActions?.trackController.handleTracksDropOnPlaylist(playlistId, trackIds)}
							onPlaylistMove={(playlistId, targetFolderId) =>
								$pageActions?.playlistController.handlePlaylistDragMove(playlistId, targetFolderId)}
						/>
					</div>

					<ResizeHandle onResize={handleSidebarResize} />
				{/if}

				<!-- Right: Main Content -->
				<div class="flex flex-1 overflow-hidden {$activeView !== 'beatport' && $activeView !== 'player' && $activeView !== 'stats' ? 'rounded-tl-md border-t border-l border-stroke' : ''}">
					{@render children()}
				</div>
			</div>

			<Player
				onNext={() => $pageActions?.playNextTrack()}
				onPrevious={() => $pageActions?.playPreviousTrack()}
				onLocateTrack={() => $pageActions?.locatePlayingTrack()}
			/>
		</div>
	{/if}
</div>

{#if showWizardTour}
	<WizardTour
		onComplete={() => {
			showWizardTour = false
			settingsStore.completeWizard()
		}}
		onSkip={() => {
			showWizardTour = false
			settingsStore.completeWizard()
		}}
	/>
{/if}

<ToastContainer />
<CrashScreen />
