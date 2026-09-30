<script lang="ts">
	import type { DiscoveryFunnel, FunnelStages, TimeRange } from '$shared/types'
	import { getDiscoveryFunnel } from '$shared/api/discovery'
	import { translate } from '$shared/i18n'
	import { language } from '$lib/stores'
	import { formatNumber } from '$shared/utils/format'
	import { toErrorMessage } from '$shared/utils/errors'
	import { Button } from '$lib/components/common'
	import StatsCard from './StatsCard.svelte'
	import { discoverySourceName, formatShare, funnelSince } from './format'

	type Props = {
		/** The Pulse period: the funnel counts the releases added in it. */
		range: TimeRange
		/** Bumped by the Pulse refresh button: reloads the funnel. */
		refreshKey?: number
	}

	let { range, refreshKey = 0 }: Props = $props()

	let funnel = $state<DiscoveryFunnel | null>(null)
	let loading = $state(true)
	let error = $state<string | null>(null)
	let request = 0

	async function load(since: string | undefined) {
		const current = ++request
		loading = true
		error = null
		try {
			const result = await getDiscoveryFunnel(since)
			if (current === request) funnel = result
		} catch (err) {
			if (current === request) error = toErrorMessage(err, $translate('common.unknownError'))
		} finally {
			if (current === request) loading = false
		}
	}

	$effect(() => {
		void refreshKey
		load(funnelSince(range))
	})

	type Stage = { key: keyof FunnelStages; label: string }
	const stages: Stage[] = $derived([
		{ key: 'discovered', label: $translate('stats.funnel.stage.discovered') },
		{ key: 'in_library', label: $translate('stats.funnel.stage.inLibrary') },
		{ key: 'played_in_set', label: $translate('stats.funnel.stage.playedInSet') },
	])

	const total = $derived(funnel?.total ?? { discovered: 0, in_library: 0, played_in_set: 0 })
	const subtitle = $derived($translate(range === 'all' ? 'stats.funnel.scopeAll' : 'stats.funnel.scopeRange'))

	function sourceLabel(source: string): string {
		return discoverySourceName(source) ?? $translate('discovery.sourceOther')
	}

	/** Width of a stage bar: its share of the discoveries, never thinner than a visible sliver when non-zero. */
	function barWidth(value: number): string {
		if (total.discovered <= 0 || value <= 0) return '0%'
		return `${Math.max(2, Math.round((value / total.discovered) * 100))}%`
	}
</script>

<StatsCard headingId="stats-funnel-title" title={$translate('stats.funnel.title')} {subtitle} icon="filter">
	{#if loading && !funnel}
		<div class="space-y-3" aria-busy="true">
			{#each Array(3) as _, i (i)}
				<div class="h-9 animate-pulse rounded-lg bg-surface-3 motion-reduce:animate-none"></div>
			{/each}
		</div>
	{:else if error}
		<div class="flex flex-col items-start gap-3 py-4" role="alert">
			<p class="text-sm text-text-primary">{$translate('stats.funnel.loadFailed')}</p>
			<p class="text-xs text-text-secondary">{error}</p>
			<Button variant="secondary" size="sm" onclick={() => load(funnelSince(range))}>
				{$translate('stats.retry')}
			</Button>
		</div>
	{:else if total.discovered === 0}
		<p class="py-8 text-center text-sm text-text-secondary">{$translate('stats.funnel.empty')}</p>
	{:else}
		<div class="space-y-5" aria-busy={loading}>
			<!-- The three stages, each bar sized by its share of the discoveries -->
			<ol class="space-y-3">
				{#each stages as stage (stage.key)}
					{@const value = total[stage.key]}
					<li class="min-w-0">
						<div class="mb-1 flex items-baseline justify-between gap-3 text-xs">
							<span class="truncate font-semibold text-text-primary">{stage.label}</span>
							<span class="flex-shrink-0 text-text-secondary tabular-nums">
								{$translate('stats.funnel.releases', { values: { count: value } })}
								{#if stage.key !== 'discovered'}
									· {$translate('stats.funnel.share', {
										values: { percent: formatShare(value, total.discovered, $language) },
									})}
								{/if}
							</span>
						</div>
						<div class="h-2 overflow-hidden rounded-full bg-surface-3" aria-hidden="true">
							<div class="h-full rounded-full bg-brand-primary" style="width: {barWidth(value)}"></div>
						</div>
					</li>
				{/each}
			</ol>

			<!-- By source -->
			{#if funnel && funnel.by_source.length > 0}
				<table class="w-full table-fixed text-xs">
					<caption class="mb-2 text-left text-xs font-semibold tracking-wider text-text-secondary uppercase">
						{$translate('stats.funnel.bySource')}
					</caption>
					<thead>
						<tr class="border-b border-stroke-subtle text-text-secondary">
							<th scope="col" class="w-2/5 py-1.5 text-left font-medium">{$translate('stats.funnel.source')}</th>
							{#each stages as stage (stage.key)}
								<th scope="col" class="truncate py-1.5 text-right font-medium" title={stage.label}>{stage.label}</th>
							{/each}
						</tr>
					</thead>
					<tbody>
						{#each funnel.by_source as source (source.source_type)}
							<tr class="border-b border-stroke-subtle last:border-b-0">
								<th scope="row" class="truncate py-1.5 text-left font-medium text-text-primary">
									{sourceLabel(source.source_type)}
								</th>
								{#each stages as stage (stage.key)}
									<td class="py-1.5 text-right text-text-primary tabular-nums">
										{formatNumber(source.stages[stage.key], $language)}
									</td>
								{/each}
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}

			<p class="text-xs text-text-secondary">{$translate('stats.funnel.note')}</p>
		</div>
	{/if}
</StatsCard>
