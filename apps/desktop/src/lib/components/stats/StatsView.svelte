<script lang="ts">
	import { onMount, onDestroy } from 'svelte'
	import type { TimeRange } from '$shared/types'
	import {
		statsStore,
		statsSummary,
		topTracks,
		topArtists,
		harmonicStats,
		bpmStats,
		listeningHeatmap,
		spotifyAuth,
		spotifyNowPlaying,
		rekordboxDetected,
		rekordboxSessions,
		mikDetected,
		statsSelectedRange,
		isStatsLoading,
	} from '$shared/stores/stats'
	import { translate } from '$shared/i18n'
	import { Button, Icon, SegmentedControl, Spinner, Tooltip, type SegmentOption } from '$lib/components/common'
	import { save } from '@tauri-apps/plugin-dialog'
	import { exportListeningHistory } from '$shared/api/stats'
	import { toastStore } from '$shared/stores/toast'
	import { withNativeDialog } from '$shared/utils'
	import { toErrorMessage } from '$shared/utils/errors'
	import StatsKpiCards from './StatsKpiCards.svelte'
	import StatsSourceBar from './StatsSourceBar.svelte'
	import StatsTopTracks from './StatsTopTracks.svelte'
	import StatsTopArtists from './StatsTopArtists.svelte'
	import StatsHarmonicWheel from './StatsHarmonicWheel.svelte'
	import StatsBpmDistribution from './StatsBpmDistribution.svelte'
	import StatsHeatmap from './StatsHeatmap.svelte'
	import StatsIntegrations from './StatsIntegrations.svelte'
	import StatsRecap from './StatsRecap.svelte'
	import StatsFunnel from './StatsFunnel.svelte'
	import StatsSets from './StatsSets.svelte'
	import { historyExportTarget } from './historyExport'

	const TIME_RANGES: TimeRange[] = ['today', '7d', '30d', 'year', 'all']

	const rangeOptions: SegmentOption<TimeRange>[] = $derived(
		TIME_RANGES.map((range) => ({ value: range, label: $translate(`stats.range.${range}`) }))
	)

	let refreshTimer: NodeJS.Timeout | null = null

	onMount(() => {
		statsStore.refreshAll()
		// Poll Spotify now playing every 10 seconds
		refreshTimer = setInterval(() => {
			statsStore.refreshNowPlaying()
		}, 10000)
	})

	onDestroy(() => {
		if (refreshTimer) clearInterval(refreshTimer)
	})

	function handleRangeChange(range: TimeRange) {
		statsStore.setRange(range)
	}

	// Bumped by the refresh button so the recap and the funnel, which load their own data, reload too.
	let refreshKey = $state(0)

	function handleRefresh() {
		statsStore.refreshAll()
		refreshKey += 1
	}

	// History export: the native save dialog picks the file, its extension picks the format (CSV or JSON).
	let exportingHistory = $state(false)
	async function handleExportHistory() {
		if (exportingHistory) return
		const path = await withNativeDialog(() =>
			save({
				defaultPath: 'crate-listening-history.csv',
				filters: [
					{ name: 'CSV', extensions: ['csv'] },
					{ name: 'JSON', extensions: ['json'] },
				],
			})
		)
		if (!path) return
		const target = historyExportTarget(path)
		exportingHistory = true
		try {
			const count = await exportListeningHistory(target.format, target.path)
			toastStore.success($translate('stats.export.success', { values: { count } }))
		} catch (err) {
			const message = toErrorMessage(err, $translate('common.unknownError'))
			toastStore.error($translate('stats.export.failed', { values: { error: message } }))
		} finally {
			exportingHistory = false
		}
	}
</script>

<div class="flex h-full w-full flex-col overflow-y-auto bg-surface-0 pb-32">
	<!-- Top Sticky Glass Header -->
	<div
		class="sticky top-0 z-20 flex flex-wrap items-center justify-between gap-4 border-b border-stroke/80 bg-surface-1/90 px-6 py-3.5 backdrop-blur-xl"
	>
		<div class="flex items-center gap-3">
			<div
				class="flex h-9 w-9 items-center justify-center rounded-xl border border-brand-primary/30 bg-brand-primary/15 text-brand-primary shadow-sm"
			>
				<Icon name="chart" class="h-5 w-5 text-brand-primary" />
			</div>
			<div>
				<div class="flex items-center gap-2">
					<h1 class="text-base font-bold tracking-tight text-text-primary">{$translate('stats.header.title')}</h1>
					<span
						class="rounded-full border border-brand-primary/40 bg-brand-primary/20 px-2 py-0.5 font-mono text-[10px] font-bold tracking-wider text-brand-primary uppercase"
					>
						{$translate('stats.header.live')}
					</span>
				</div>
				<p class="text-[11px] text-text-tertiary">
					{$translate('stats.header.subtitle')}
				</p>
			</div>
		</div>

		<!-- Time Range Segmented Control & Refresh. A zero basis (`flex-1 min-w-0`) keeps the header on one line: the
		     period bar shrinks and scrolls instead of the group wrapping under the title (CRA-173). -->
		<div class="flex min-w-0 flex-1 items-center justify-end gap-3">
			<!-- Now Playing Spotify Live Pill (if active) -->
			{#if $spotifyNowPlaying?.is_playing}
				<div
					class="hidden shrink-0 animate-pulse items-center gap-2 rounded-full border border-source-spotify/40 bg-source-spotify/10 px-3 py-1 text-xs font-medium text-source-spotify-text shadow-sm motion-reduce:animate-none md:flex"
				>
					<span class="h-2 w-2 rounded-full bg-source-spotify"></span>
					<span class="font-bold">{$translate('stats.header.liveSpotify')}</span>
					<span class="max-w-[140px] truncate text-text-primary">
						{$spotifyNowPlaying.title}
					</span>
				</div>
			{/if}

			<!-- Period (Crate control: stays on the neutral look, DESIGN.md Pulse family). Scrollable: when the presets do
			     not fit, arrows and edge fades reveal the rest (DESIGN.md "Scroll affordance"). -->
			<SegmentedControl
				variant="boxed"
				scrollable
				ariaLabel={$translate('stats.range.label')}
				options={rangeOptions}
				value={$statsSelectedRange}
				onchange={handleRangeChange}
			/>

			<!-- Export the whole listening history (CSV or JSON). Icon only below 1280 px, so the header keeps one line at
			     1000 px as it did before the button existed. -->
			<Tooltip text={$translate('stats.export.hint')} position="bottom" wrapperClass="inline-flex shrink-0">
				<Button
					variant="secondary"
					size="sm"
					class="h-8"
					onclick={handleExportHistory}
					disabled={exportingHistory}
					aria-label={$translate('stats.export.button')}
				>
					{#if exportingHistory}
						<Spinner icon="loader" color="current" class="h-4 w-4" />
					{:else}
						<Icon name="download" class="h-4 w-4" />
					{/if}
					<span class="ml-1.5 hidden xl:inline">{$translate('stats.export.button')}</span>
				</Button>
			</Tooltip>

			<!-- Refresh Button -->
			<button
				type="button"
				class="flex h-8 w-8 shrink-0 cursor-pointer items-center justify-center rounded-xl border border-stroke bg-surface-2 text-text-secondary shadow-sm transition-all hover:border-stroke-strong hover:bg-surface-3 hover:text-text-primary active:scale-95"
				onclick={handleRefresh}
				title={$translate('stats.header.refresh')}
				disabled={$isStatsLoading}
			>
				{#if $isStatsLoading}
					<Spinner icon="refresh-cw" color="current" class="h-4 w-4 text-brand-primary" />
				{:else}
					<Icon name="refresh-cw" class="h-4 w-4" />
				{/if}
			</button>
		</div>
	</div>

	<!-- Main Content Body -->
	<div class="mx-auto w-full max-w-7xl space-y-6 p-6">
		<!-- 1. Hero KPI Cards -->
		<StatsKpiCards
			summary={$statsSummary}
			topArtists={$topArtists}
			rekordboxSessions={$rekordboxSessions}
			isLoading={$isStatsLoading}
		/>

		<!-- 2. Multi-Sources Breakdown Bar -->
		<StatsSourceBar summary={$statsSummary} />

		<!-- 3. Top Morceaux & Top Artistes (2 Columns side-by-side) -->
		<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
			<StatsTopTracks tracks={$topTracks} isLoading={$isStatsLoading} />
			<StatsTopArtists artists={$topArtists} isLoading={$isStatsLoading} />
		</div>

		<!-- 4. DJ & Musical Analytics (Harmonic Camelot Wheel & BPM Distribution) -->
		<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
			<StatsHarmonicWheel harmonicStats={$harmonicStats} isLoading={$isStatsLoading} />
			<StatsBpmDistribution bpmStats={$bpmStats} isLoading={$isStatsLoading} />
		</div>

		<!-- 5. Weekly 24h x 7d Heatmap Matrix -->
		<StatsHeatmap heatmap={$listeningHeatmap} isLoading={$isStatsLoading} />

		<!-- 6. Recap of a week or a year -->
		<StatsRecap {refreshKey} />

		<!-- 7. Rekordbox sets (timeline of each) and the discovery funnel -->
		<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
			<StatsSets sessions={$rekordboxSessions} isLoading={$isStatsLoading} />
			<StatsFunnel range={$statsSelectedRange} {refreshKey} />
		</div>

		<!-- 8. Cloud & Files Integrations Banner (Spotify, Rekordbox & Mixed In Key) -->
		<StatsIntegrations
			spotifyAuth={$spotifyAuth}
			spotifyNowPlaying={$spotifyNowPlaying}
			rekordboxDetected={$rekordboxDetected}
			rekordboxSessions={$rekordboxSessions}
			mikDetected={$mikDetected}
		/>
	</div>
</div>
