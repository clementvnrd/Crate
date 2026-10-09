<script lang="ts">
	import { untrack } from 'svelte'
	import type { Recap, RecapPeriod, TimeRange } from '$shared/types'
	import { getRecap } from '$shared/api/stats'
	import { translate } from '$shared/i18n'
	import { language } from '$lib/stores'
	import { formatDate, formatNumber } from '$shared/utils/format'
	import { toErrorMessage } from '$shared/utils/errors'
	import { Button, IconButton, KeyBadge, SegmentedControl, type SegmentOption } from '$lib/components/common'
	import StatsCard from './StatsCard.svelte'
	import { formatChange, formatHour, formatWeekday, recapYearOffset, splitMinutes } from './format'

	type Props = {
		/** The Pulse period: picking "This year" or a previous year opens the recap of that calendar year. */
		range?: TimeRange
		/** Bumped by the Pulse refresh button: reloads the recap. */
		refreshKey?: number
	}

	let { range, refreshKey = 0 }: Props = $props()

	let period = $state<RecapPeriod>('week')
	let offset = $state(0)
	let recap = $state<Recap | null>(null)
	let loading = $state(true)
	let error = $state<string | null>(null)
	let request = 0

	const periodOptions: SegmentOption<RecapPeriod>[] = $derived([
		{ value: 'week', label: $translate('stats.recap.period.week') },
		{ value: 'year', label: $translate('stats.recap.period.year') },
	])

	async function load(which: RecapPeriod, back: number) {
		const current = ++request
		loading = true
		error = null
		try {
			const result = await getRecap(which, back)
			if (current === request) recap = result
		} catch (err) {
			if (current === request) error = toErrorMessage(err, $translate('common.unknownError'))
		} finally {
			if (current === request) loading = false
		}
	}

	// Follows the Pulse period when it names a calendar year; the other periods leave the recap where the owner put it.
	$effect(() => {
		if (!range) return
		const back = recapYearOffset(range)
		if (back === null) return
		untrack(() => {
			period = 'year'
			offset = back
		})
	})

	$effect(() => {
		void refreshKey
		load(period, offset)
	})

	function changePeriod(value: RecapPeriod) {
		period = value
		offset = 0
	}

	function duration(minutes: number): string {
		const { hours, minutes: rest } = splitMinutes(minutes)
		return hours > 0
			? $translate('stats.duration.hoursMinutes', {
					values: { hours: formatNumber(hours, $language), minutes: String(rest).padStart(2, '0') },
				})
			: $translate('stats.duration.minutes', { values: { minutes: rest } })
	}

	function change(current: number, previous: number): string {
		const value = formatChange(current, previous, $language)
		if (value === null) return $translate('stats.recap.noPrevious')
		return $translate(period === 'week' ? 'stats.recap.changeWeek' : 'stats.recap.changeYear', {
			values: { change: value },
		})
	}

	const title = $derived($translate(period === 'week' ? 'stats.recap.titleWeek' : 'stats.recap.titleYear'))
	const subtitle = $derived(
		recap
			? $translate('stats.recap.range', {
					values: {
						start: formatDate(recap.start_date, 'locale', $language),
						end: formatDate(recap.end_date, 'locale', $language),
					},
				})
			: undefined
	)
	const hasPlays = $derived((recap?.total_plays ?? 0) > 0)
</script>

<StatsCard headingId="stats-recap-title" {title} {subtitle} icon="star">
	{#snippet actions()}
		<SegmentedControl
			variant="boxed"
			unselectedTone="secondary"
			ariaLabel={$translate('stats.recap.periodLabel')}
			options={periodOptions}
			value={period}
			onchange={changePeriod}
		/>
		<IconButton
			icon="chevron-right"
			iconClass="h-4 w-4 rotate-180"
			ariaLabel={$translate('stats.recap.previous')}
			title={$translate('stats.recap.previous')}
			onclick={() => (offset += 1)}
		/>
		<IconButton
			icon="chevron-right"
			iconClass="h-4 w-4"
			ariaLabel={$translate('stats.recap.next')}
			title={$translate('stats.recap.next')}
			disabled={offset === 0}
			onclick={() => (offset = Math.max(0, offset - 1))}
		/>
	{/snippet}

	{#if loading && !recap}
		<div class="grid grid-cols-2 gap-3 lg:grid-cols-4" aria-busy="true">
			{#each Array(4) as _, i (i)}
				<div class="h-20 animate-pulse rounded-lg bg-surface-3 motion-reduce:animate-none"></div>
			{/each}
		</div>
	{:else if error}
		<div class="flex flex-col items-start gap-3 py-4" role="alert">
			<p class="text-sm text-text-primary">{$translate('stats.recap.loadFailed')}</p>
			<p class="text-xs text-text-secondary">{error}</p>
			<Button variant="secondary" size="sm" onclick={() => load(period, offset)}>
				{$translate('stats.retry')}
			</Button>
		</div>
	{:else if recap && !hasPlays}
		<p class="py-8 text-center text-sm text-text-secondary">{$translate('stats.recap.empty')}</p>
	{:else if recap}
		<div class="space-y-5" aria-busy={loading}>
			<!-- Totals of the period, each with its change against the period before -->
			<dl class="grid grid-cols-2 gap-3 lg:grid-cols-4">
				<div class="min-w-0 rounded-lg bg-surface-2 p-3">
					<dt class="text-xs font-semibold tracking-wider text-text-secondary uppercase">
						{$translate('stats.recap.plays')}
					</dt>
					<dd class="mt-1 text-xl font-semibold text-text-primary tabular-nums">
						{formatNumber(recap.total_plays, $language)}
					</dd>
					<dd class="truncate text-xs text-text-secondary">{change(recap.total_plays, recap.previous_plays)}</dd>
				</div>
				<div class="min-w-0 rounded-lg bg-surface-2 p-3">
					<dt class="text-xs font-semibold tracking-wider text-text-secondary uppercase">
						{$translate('stats.recap.listeningTime')}
					</dt>
					<dd class="mt-1 text-xl font-semibold text-text-primary tabular-nums">{duration(recap.total_minutes)}</dd>
					<dd class="truncate text-xs text-text-secondary">
						{change(recap.total_minutes, recap.previous_minutes)}
					</dd>
				</div>
				<div class="min-w-0 rounded-lg bg-surface-2 p-3">
					<dt class="text-xs font-semibold tracking-wider text-text-secondary uppercase">
						{$translate('stats.recap.uniqueTracks')}
					</dt>
					<dd class="mt-1 text-xl font-semibold text-text-primary tabular-nums">
						{formatNumber(recap.unique_tracks, $language)}
					</dd>
					<dd class="truncate text-xs text-text-secondary">
						{$translate('stats.recap.uniqueArtists', { values: { count: recap.unique_artists } })}
					</dd>
				</div>
				<div class="min-w-0 rounded-lg bg-surface-2 p-3">
					<dt class="text-xs font-semibold tracking-wider text-text-secondary uppercase">
						{$translate('stats.recap.newTracks')}
					</dt>
					<dd class="mt-1 text-xl font-semibold text-text-primary tabular-nums">
						{formatNumber(recap.new_tracks, $language)}
					</dd>
					<dd class="truncate text-xs text-text-secondary">{$translate('stats.recap.newTracksHint')}</dd>
				</div>
			</dl>

			<div class="grid grid-cols-1 gap-5 lg:grid-cols-3">
				<!-- Top tracks -->
				<div class="min-w-0">
					<h4 class="mb-2 text-xs font-semibold tracking-wider text-text-secondary uppercase">
						{$translate('stats.recap.topTracks')}
					</h4>
					<ol class="space-y-1">
						{#each recap.top_tracks as track, index (track.title + track.artist + index)}
							<li class="flex items-center gap-3 py-1">
								<span class="w-4 flex-shrink-0 text-right text-xs text-text-secondary tabular-nums">{index + 1}</span>
								<div class="min-w-0 flex-1">
									<div class="truncate text-xs font-semibold text-text-primary">{track.title}</div>
									<div class="truncate text-xs text-text-secondary">{track.artist}</div>
								</div>
								<span class="flex-shrink-0 text-xs text-text-secondary tabular-nums">
									{$translate('stats.plays', { values: { count: track.plays } })}
								</span>
							</li>
						{/each}
					</ol>
				</div>

				<!-- Top artists -->
				<div class="min-w-0">
					<h4 class="mb-2 text-xs font-semibold tracking-wider text-text-secondary uppercase">
						{$translate('stats.recap.topArtists')}
					</h4>
					<ol class="space-y-1">
						{#each recap.top_artists as artist, index (artist.artist + index)}
							<li class="flex items-center gap-3 py-1">
								<span class="w-4 flex-shrink-0 text-right text-xs text-text-secondary tabular-nums">{index + 1}</span>
								<div class="min-w-0 flex-1 truncate text-xs font-semibold text-text-primary">{artist.artist}</div>
								<span class="flex-shrink-0 text-xs text-text-secondary tabular-nums">
									{$translate('stats.plays', { values: { count: artist.plays } })}
								</span>
							</li>
						{/each}
					</ol>
				</div>

				<!-- Highlights: busiest day, peak hour, favourite day, top keys -->
				<div class="min-w-0">
					<h4 class="mb-2 text-xs font-semibold tracking-wider text-text-secondary uppercase">
						{$translate('stats.recap.highlights')}
					</h4>
					<dl class="space-y-2 text-xs">
						<div class="flex items-baseline justify-between gap-3">
							<dt class="text-text-secondary">{$translate('stats.recap.busiestDay')}</dt>
							<dd class="text-right text-text-primary tabular-nums">
								{#if recap.busiest_day}
									{formatDate(recap.busiest_day.date, 'locale', $language)} ·
									{$translate('stats.plays', { values: { count: recap.busiest_day.plays } })}
								{:else}
									{$translate('stats.recap.none')}
								{/if}
							</dd>
						</div>
						<div class="flex items-baseline justify-between gap-3">
							<dt class="text-text-secondary">{$translate('stats.recap.peakHour')}</dt>
							<dd class="text-text-primary tabular-nums">
								{recap.peak_hour === null ? $translate('stats.recap.none') : formatHour(recap.peak_hour, $language)}
							</dd>
						</div>
						<div class="flex items-baseline justify-between gap-3">
							<dt class="text-text-secondary">{$translate('stats.recap.peakWeekday')}</dt>
							<dd class="text-text-primary first-letter:uppercase">
								{recap.peak_weekday === null
									? $translate('stats.recap.none')
									: formatWeekday(recap.peak_weekday, $language)}
							</dd>
						</div>
						<div class="flex items-center justify-between gap-3">
							<dt class="text-text-secondary">{$translate('stats.recap.topKeys')}</dt>
							<dd class="flex items-center gap-1.5">
								{#each recap.top_keys as key (key.key)}
									<KeyBadge value={key.key} variant="tag-wide" />
								{:else}
									<span class="text-text-primary">{$translate('stats.recap.none')}</span>
								{/each}
							</dd>
						</div>
					</dl>
				</div>
			</div>
		</div>
	{/if}
</StatsCard>
