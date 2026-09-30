<script lang="ts">
	import type { HarmonicRelation, RekordboxSession, SessionTimeline, SessionTrack } from '$shared/types'
	import { getRekordboxSessionTimeline } from '$shared/api/stats'
	import { translate } from '$shared/i18n'
	import { language } from '$lib/stores'
	import { formatDate, formatDuration, formatNumber } from '$shared/utils/format'
	import { toErrorMessage } from '$shared/utils/errors'
	import { Button, EnergyBadge, Icon, KeyBadge, Modal, Spinner } from '$lib/components/common'
	import { formatTempoDelta, formatTimeOfDay } from './format'

	type Props = {
		open: boolean
		session: RekordboxSession
		/** Display name of the set (its Rekordbox name, or "Untitled set"). */
		name: string
		onClose: () => void
	}

	let { open, session, name, onClose }: Props = $props()

	let timeline = $state<SessionTimeline | null>(null)
	let loading = $state(true)
	let error = $state<string | null>(null)
	let request = 0

	async function load(sessionId: string) {
		const current = ++request
		loading = true
		error = null
		timeline = null
		try {
			const result = await getRekordboxSessionTimeline(sessionId)
			if (current === request) timeline = result
		} catch (err) {
			if (current === request) error = toErrorMessage(err, $translate('common.unknownError'))
		} finally {
			if (current === request) loading = false
		}
	}

	$effect(() => {
		if (open) load(session.id)
	})

	// The app's global shortcuts take Enter and Space (play/pause) on the window. This modal is not one of the
	// orchestrated modals that switch them off, so keep those keys for the focused control inside it.
	let body: HTMLDivElement | undefined = $state()
	function keepKeysInModal(e: KeyboardEvent) {
		if (!open || !(e.key === 'Enter' || e.code === 'Space')) return
		const dialog = body?.closest('dialog')
		if (dialog && e.target instanceof Node && dialog.contains(e.target)) e.stopPropagation()
	}

	/** The artist, followed by a note when Crate does not know the file of this track. */
	function artistLine(track: SessionTrack): string {
		return track.library_track_id === null
			? `${track.artist} · ${$translate('stats.timeline.notInLibrary')}`
			: track.artist
	}

	function relationLabel(relation: HarmonicRelation): string {
		return $translate(`stats.timeline.relation.${relation}`)
	}

	const subtitle = $derived(
		`${formatDate(session.started_at, 'locale', $language)} · ${formatTimeOfDay(session.started_at, $language)} · ${$translate(
			'stats.sets.meta',
			{ values: { tracks: session.total_tracks, duration: formatDuration(session.total_played_ms) } }
		)}`
	)
</script>

<svelte:window onkeydowncapture={keepKeysInModal} />

<Modal {open} title={name} size="2xl" {onClose}>
	<div bind:this={body} class="space-y-4">
		<p class="text-xs text-text-secondary tabular-nums">{subtitle}</p>

		{#if loading}
			<div class="flex items-center gap-2 py-8 text-sm text-text-secondary" aria-busy="true">
				<Spinner />
				{$translate('common.loading')}
			</div>
		{:else if error}
			<div class="flex flex-col items-start gap-3 py-4" role="alert">
				<p class="text-sm text-text-primary">{$translate('stats.timeline.loadFailed')}</p>
				<p class="text-xs text-text-secondary">{error}</p>
				<Button variant="secondary" size="sm" onclick={() => load(session.id)}>
					{$translate('stats.retry')}
				</Button>
			</div>
		{:else if timeline && timeline.tracks.length === 0}
			<p class="py-8 text-center text-sm text-text-secondary">{$translate('stats.timeline.empty')}</p>
		{:else if timeline}
			<!-- How the set mixes, at a glance -->
			<dl class="grid grid-cols-2 gap-3 sm:grid-cols-4">
				<div class="rounded-lg bg-surface-2 p-3">
					<dt class="text-xs text-text-secondary">{$translate('stats.timeline.tracks')}</dt>
					<dd class="mt-1 text-lg font-semibold text-text-primary tabular-nums">
						{formatNumber(timeline.tracks.length, $language)}
					</dd>
				</div>
				<div class="rounded-lg bg-surface-2 p-3">
					<dt class="text-xs text-text-secondary">{$translate('stats.timeline.harmonic')}</dt>
					<dd class="mt-1 text-lg font-semibold text-text-primary tabular-nums">
						{formatNumber(timeline.harmonic_transitions, $language)}
					</dd>
				</div>
				<div class="rounded-lg bg-surface-2 p-3">
					<dt class="text-xs text-text-secondary">{$translate('stats.timeline.clashing')}</dt>
					<dd class="mt-1 text-lg font-semibold text-text-primary tabular-nums">
						{formatNumber(timeline.clashing_transitions, $language)}
					</dd>
				</div>
				<div class="rounded-lg bg-surface-2 p-3">
					<dt class="text-xs text-text-secondary">{$translate('stats.timeline.unknown')}</dt>
					<dd class="mt-1 text-lg font-semibold text-text-primary tabular-nums">
						{formatNumber(timeline.unknown_transitions, $language)}
					</dd>
				</div>
			</dl>

			<!-- The tracklist in play order, with how each track mixes out of the previous one -->
			<table class="w-full table-fixed text-xs">
				<thead>
					<tr class="border-b border-stroke-subtle text-left text-text-secondary">
						<th scope="col" class="w-8 py-1.5 pr-2 text-right font-medium">
							{$translate('stats.timeline.columns.position')}
						</th>
						<th scope="col" class="w-20 px-2 py-1.5 font-medium">{$translate('stats.timeline.columns.time')}</th>
						<th scope="col" class="px-2 py-1.5 font-medium">{$translate('stats.timeline.columns.track')}</th>
						<th scope="col" class="w-14 px-2 py-1.5 text-right font-medium">
							{$translate('stats.timeline.columns.bpm')}
						</th>
						<th scope="col" class="w-16 px-2 py-1.5 font-medium">{$translate('stats.timeline.columns.key')}</th>
						<th scope="col" class="w-16 px-2 py-1.5 font-medium">
							{$translate('stats.timeline.columns.energy')}
						</th>
						<th scope="col" class="w-48 py-1.5 pl-2 font-medium">
							{$translate('stats.timeline.columns.transition')}
						</th>
					</tr>
				</thead>
				<tbody>
					{#each timeline.tracks as track (track.position)}
						{@const transition = track.from_previous}
						<tr class="border-b border-stroke-subtle last:border-b-0">
							<td class="py-1.5 pr-2 text-right text-text-secondary tabular-nums">{track.position}</td>
							<td class="px-2 py-1.5 text-text-secondary tabular-nums">
								{formatTimeOfDay(track.played_at, $language)}
							</td>
							<td class="min-w-0 px-2 py-1.5">
								<div class="truncate font-semibold text-text-primary">{track.title}</div>
								<div class="truncate text-text-secondary">
									{artistLine(track)}
								</div>
							</td>
							<td class="px-2 py-1.5 text-right text-text-primary tabular-nums">
								{track.bpm === null ? '—' : Math.round(track.bpm)}
							</td>
							<td class="px-2 py-1.5">
								{#if track.key}
									<KeyBadge value={track.key} variant="tag-wide" />
								{:else}
									<span class="text-text-secondary">—</span>
								{/if}
							</td>
							<td class="px-2 py-1.5">
								{#if track.energy !== null}
									<EnergyBadge energy={track.energy} size="sm" />
								{:else}
									<span class="text-text-secondary">—</span>
								{/if}
							</td>
							<td class="py-1.5 pl-2">
								{#if transition}
									<div class="flex items-center gap-1.5 text-text-primary">
										{#if transition.harmonic === 'clash'}
											<Icon name="alert-triangle" class="h-3.5 w-3.5 flex-shrink-0 text-warning" />
										{/if}
										<span class="truncate">{relationLabel(transition.harmonic)}</span>
									</div>
									{#if transition.bpm_delta_percent !== null}
										<div class="text-text-secondary tabular-nums">
											{$translate('stats.timeline.tempo', {
												values: { delta: formatTempoDelta(transition.bpm_delta_percent, $language) },
											})}
										</div>
									{/if}
								{:else}
									<span class="text-text-secondary">{$translate('stats.timeline.opening')}</span>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
	</div>

	{#snippet footer()}
		<Button variant="secondary" size="sm" onclick={onClose}>{$translate('common.close')}</Button>
	{/snippet}
</Modal>
