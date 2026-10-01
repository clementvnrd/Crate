<script lang="ts">
	import type { StatsSummary, TopArtistItem, RekordboxSession } from '$shared/types'
	import Icon from '$lib/components/common/Icon.svelte'
	import { language } from '$lib/stores'
	import { formatNumber } from '$shared/utils/format'
	import { translate } from '$shared/i18n'

	type Props = {
		summary: StatsSummary | null
		topArtists: TopArtistItem[]
		rekordboxSessions: RekordboxSession[]
		isLoading: boolean
	}

	let { summary, topArtists, rekordboxSessions, isLoading }: Props = $props()

	function formatTotalTime(minutes: number, locale: string): { primary: string; secondary: string } {
		if (!minutes || minutes <= 0) {
			return {
				primary: $translate('stats.duration.minutes', { values: { minutes: 0 } }),
				secondary: $translate('stats.kpi.noListening'),
			}
		}
		const hours = Math.floor(minutes / 60)
		const mins = minutes % 60
		const secondary = $translate('stats.kpi.totalMinutes', {
			values: { count: minutes, total: formatNumber(minutes, locale) },
		})
		if (hours > 0) {
			return {
				primary: $translate('stats.duration.hoursMinutes', {
					values: { hours: formatNumber(hours, locale), minutes: mins.toString().padStart(2, '0') },
				}),
				secondary,
			}
		}
		return {
			primary: $translate('stats.duration.minutes', { values: { minutes: mins } }),
			secondary,
		}
	}

	let timeDisplay = $derived(formatTotalTime(summary?.total_minutes ?? 0, $language))
	let totalPlays = $derived(formatNumber(summary?.total_plays ?? 0, $language))
	let uniqueArtistsCount = $derived(formatNumber(topArtists.length, $language))
	let totalRekordboxMs = $derived(rekordboxSessions.reduce((acc, s) => acc + (s.total_played_ms || 0), 0))
	let rekordboxHours = $derived(Math.round(totalRekordboxMs / 3_600_000))
</script>

<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
	<!-- Card 1: Listening Time -->
	<div
		class="group relative overflow-hidden rounded-xl border border-pulse-listening/20 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl transition-all duration-300 hover:border-pulse-listening/40 hover:shadow-pulse-listening/10"
	>
		<div
			class="pointer-events-none absolute -top-6 -right-6 h-28 w-28 rounded-full bg-pulse-listening/10 blur-2xl transition-all group-hover:bg-pulse-listening/20"
		></div>
		<div class="flex items-start justify-between">
			<div class="space-y-1">
				<span class="text-xs font-semibold tracking-wider text-text-tertiary uppercase">
					{$translate('stats.kpi.listeningTime')}
				</span>
				<div class="text-2xl font-black tracking-tight text-text-primary">
					{#if isLoading && !summary}
						<span class="inline-block h-7 w-24 animate-pulse rounded bg-surface-3 motion-reduce:animate-none"></span>
					{:else}
						{timeDisplay.primary}
					{/if}
				</div>
				<p class="text-[11px] text-text-secondary">
					{timeDisplay.secondary}
				</p>
			</div>
			<div
				class="flex h-11 w-11 items-center justify-center rounded-xl border border-pulse-listening/30 bg-pulse-listening/15 text-pulse-listening-text shadow-inner"
			>
				<Icon name="clock" class="h-5 w-5 text-pulse-listening-text" />
			</div>
		</div>
	</div>

	<!-- Card 2: Total Plays -->
	<div
		class="group relative overflow-hidden rounded-xl border border-pulse-plays/20 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl transition-all duration-300 hover:border-pulse-plays/40 hover:shadow-pulse-plays/10"
	>
		<div
			class="pointer-events-none absolute -top-6 -right-6 h-28 w-28 rounded-full bg-pulse-plays/10 blur-2xl transition-all group-hover:bg-pulse-plays/20"
		></div>
		<div class="flex items-start justify-between">
			<div class="space-y-1">
				<span class="text-xs font-semibold tracking-wider text-text-tertiary uppercase">
					{$translate('stats.kpi.plays')}
				</span>
				<div class="text-2xl font-black tracking-tight text-text-primary">
					{#if isLoading && !summary}
						<span class="inline-block h-7 w-20 animate-pulse rounded bg-surface-3 motion-reduce:animate-none"></span>
					{:else}
						{totalPlays}
					{/if}
				</div>
				<p class="text-[11px] text-text-secondary">{$translate('stats.kpi.playsHint')}</p>
			</div>
			<div
				class="flex h-11 w-11 items-center justify-center rounded-xl border border-pulse-plays/30 bg-pulse-plays/15 text-pulse-plays-text shadow-inner"
			>
				<Icon name="music-note" class="h-5 w-5 text-pulse-plays-text" />
			</div>
		</div>
	</div>

	<!-- Card 3: Unique Artists -->
	<div
		class="group relative overflow-hidden rounded-xl border border-pulse-artists/20 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl transition-all duration-300 hover:border-pulse-artists/40 hover:shadow-pulse-artists/10"
	>
		<div
			class="pointer-events-none absolute -top-6 -right-6 h-28 w-28 rounded-full bg-pulse-artists/10 blur-2xl transition-all group-hover:bg-pulse-artists/20"
		></div>
		<div class="flex items-start justify-between">
			<div class="space-y-1">
				<span class="text-xs font-semibold tracking-wider text-text-tertiary uppercase">
					{$translate('stats.kpi.artists')}
				</span>
				<div class="text-2xl font-black tracking-tight text-text-primary">
					{#if isLoading && !summary}
						<span class="inline-block h-7 w-16 animate-pulse rounded bg-surface-3 motion-reduce:animate-none"></span>
					{:else}
						{uniqueArtistsCount}
					{/if}
				</div>
				<p class="text-[11px] text-text-secondary">{$translate('stats.kpi.artistsHint')}</p>
			</div>
			<div
				class="flex h-11 w-11 items-center justify-center rounded-xl border border-pulse-artists/30 bg-pulse-artists/15 text-pulse-artists-text shadow-inner"
			>
				<Icon name="user" class="h-5 w-5 text-pulse-artists-text" />
			</div>
		</div>
	</div>

	<!-- Card 4: Rekordbox DJ Sessions -->
	<div
		class="group relative overflow-hidden rounded-xl border border-pulse-sessions/20 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl transition-all duration-300 hover:border-pulse-sessions/40 hover:shadow-pulse-sessions/10"
	>
		<div
			class="pointer-events-none absolute -top-6 -right-6 h-28 w-28 rounded-full bg-pulse-sessions/10 blur-2xl transition-all group-hover:bg-pulse-sessions/20"
		></div>
		<div class="flex items-start justify-between">
			<div class="space-y-1">
				<span class="text-xs font-semibold tracking-wider text-text-tertiary uppercase">
					{$translate('stats.kpi.rekordboxSessions')}
				</span>
				<div class="text-2xl font-black tracking-tight text-text-primary">
					{#if isLoading && !summary}
						<span class="inline-block h-7 w-16 animate-pulse rounded bg-surface-3 motion-reduce:animate-none"></span>
					{:else}
						{formatNumber(rekordboxSessions.length, $language)}
					{/if}
				</div>
				<p class="text-[11px] text-text-secondary">
					{rekordboxHours > 0
						? $translate('stats.kpi.mixHours', { values: { hours: formatNumber(rekordboxHours, $language) } })
						: $translate('stats.kpi.mixSets')}
				</p>
			</div>
			<div
				class="flex h-11 w-11 items-center justify-center rounded-xl border border-pulse-sessions/30 bg-pulse-sessions/15 text-pulse-sessions-text shadow-inner"
			>
				<Icon name="activity" class="h-5 w-5 text-pulse-sessions-text" />
			</div>
		</div>
	</div>
</div>
