<script lang="ts">
	import type { RekordboxSession } from '$shared/types'
	import { translate } from '$shared/i18n'
	import { language } from '$lib/stores'
	import { formatDate, formatDuration } from '$shared/utils/format'
	import { Icon } from '$lib/components/common'
	import StatsCard from './StatsCard.svelte'
	import StatsSetTimelineModal from './StatsSetTimelineModal.svelte'

	type Props = {
		sessions: RekordboxSession[]
		isLoading: boolean
	}

	let { sessions, isLoading }: Props = $props()

	// The set whose timeline is shown; kept after closing so the modal can play its closing transition.
	let openSession = $state<RekordboxSession | null>(null)
	let timelineOpen = $state(false)

	function showTimeline(session: RekordboxSession) {
		openSession = session
		timelineOpen = true
	}

	// Most recent set first, whatever order the backend lists them in.
	const ordered = $derived([...sessions].sort((a, b) => b.started_at.localeCompare(a.started_at)))

	function sessionName(session: RekordboxSession): string {
		return session.session_name?.trim() || $translate('stats.sets.untitled')
	}
</script>

<StatsCard
	headingId="stats-sets-title"
	title={$translate('stats.sets.title')}
	subtitle={$translate('stats.sets.subtitle')}
	icon="headphones"
>
	{#if isLoading && sessions.length === 0}
		<div class="space-y-2" aria-busy="true">
			{#each Array(3) as _, i (i)}
				<div class="h-12 animate-pulse rounded-lg bg-surface-3 motion-reduce:animate-none"></div>
			{/each}
		</div>
	{:else if sessions.length === 0}
		<p class="py-8 text-center text-sm text-text-secondary">{$translate('stats.sets.empty')}</p>
	{:else}
		<ul class="space-y-1">
			{#each ordered as session (session.id)}
				{@const name = sessionName(session)}
				<li>
					<button
						type="button"
						class="flex w-full cursor-pointer items-center gap-3 rounded-lg px-2.5 py-2 text-left transition-colors hover:bg-surface-2 focus-visible:outline-2 focus-visible:outline-brand-primary"
						aria-label={$translate('stats.sets.open', { values: { name } })}
						onclick={() => showTimeline(session)}
					>
						<div class="min-w-0 flex-1">
							<div class="truncate text-xs font-semibold text-text-primary">{name}</div>
							<div class="truncate text-xs text-text-secondary tabular-nums">
								{formatDate(session.started_at, 'locale', $language)} ·
								{$translate('stats.sets.meta', {
									values: { tracks: session.total_tracks, duration: formatDuration(session.total_played_ms) },
								})}
							</div>
						</div>
						<Icon name="chevron-right" class="h-4 w-4 flex-shrink-0 text-text-secondary" />
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</StatsCard>

{#if openSession}
	<StatsSetTimelineModal
		open={timelineOpen}
		session={openSession}
		name={sessionName(openSession)}
		onClose={() => (timelineOpen = false)}
	/>
{/if}
