<script lang="ts">
	import type { HarmonicRelation } from '$shared/types'
	import { setModeStore, setModeTrackCount, setModeIsEmpty } from '$shared/stores/setMode'
	import { translate } from '$shared/i18n'
	import { language } from '$lib/stores'
	import { formatDuration, formatNumber } from '$shared/utils/format'
	import { formatTempoDelta } from '$lib/components/stats/format'
	import {
		Button,
		EnergyBadge,
		Icon,
		IconButton,
		Input,
		KeyBadge,
		Modal,
		Spinner,
		Text,
		Tooltip,
	} from '$lib/components/common'

	type Props = {
		open: boolean
		trackIds: string[]
		onClose: () => void
	}

	let { open, trackIds, onClose }: Props = $props()

	// Seed the store once per opening — `trackIds` is a one-time snapshot of the selection the
	// modal was opened with, not a live binding.
	$effect(() => {
		if (open) setModeStore.open(trackIds)
	})

	function handleClose() {
		setModeStore.close()
		onClose()
	}

	function relationLabel(relation: HarmonicRelation): string {
		return $translate(`stats.timeline.relation.${relation}`)
	}

	async function handleExport() {
		await setModeStore.exportXml()
	}
</script>

<Modal {open} title={$translate('setMode.title')} size="2xl" onClose={handleClose}>
	<div class="space-y-4">
		<label class="block space-y-1">
			<Text as="span" size="xs" weight="medium" color="secondary" class="block">
				{$translate('setMode.nameLabel')}
			</Text>
			<Input
				value={$setModeStore.setName}
				placeholder={$translate('setMode.defaultName')}
				oninput={(e) => setModeStore.setSetName((e.target as HTMLInputElement).value)}
			/>
		</label>

		{#if $setModeIsEmpty}
			<p class="py-8 text-center text-sm text-text-secondary">{$translate('setMode.empty')}</p>
		{:else if $setModeStore.loading}
			<div class="flex items-center gap-2 py-8 text-sm text-text-secondary" aria-busy="true">
				<Spinner />
				{$translate('common.loading')}
			</div>
		{:else if $setModeStore.error && !$setModeStore.analysis}
			<div class="flex flex-col items-start gap-3 py-4" role="alert">
				<p class="text-sm text-text-primary">{$translate('setMode.loadFailed')}</p>
				<p class="text-xs text-text-secondary">{$setModeStore.error}</p>
			</div>
		{:else if $setModeStore.analysis}
			{@const analysis = $setModeStore.analysis}
			<!-- How the set mixes, at a glance -->
			<dl class="grid grid-cols-2 gap-3 sm:grid-cols-4">
				<div class="rounded-lg bg-surface-2 p-3">
					<dt class="text-xs text-text-secondary">{$translate('setMode.stats.tracks')}</dt>
					<dd class="mt-1 text-lg font-semibold text-text-primary tabular-nums">
						{formatNumber(analysis.entries.length, $language)}
					</dd>
				</div>
				<div class="rounded-lg bg-surface-2 p-3">
					<dt class="text-xs text-text-secondary">{$translate('setMode.stats.harmonic')}</dt>
					<dd class="mt-1 text-lg font-semibold text-text-primary tabular-nums">
						{formatNumber(analysis.harmonic_transitions, $language)}
					</dd>
				</div>
				<div class="rounded-lg bg-surface-2 p-3">
					<dt class="text-xs text-text-secondary">{$translate('setMode.stats.clashing')}</dt>
					<dd class="mt-1 text-lg font-semibold text-text-primary tabular-nums">
						{formatNumber(analysis.clashing_transitions, $language)}
					</dd>
				</div>
				<div class="rounded-lg bg-surface-2 p-3">
					<dt class="text-xs text-text-secondary">{$translate('setMode.stats.energyJumps')}</dt>
					<dd class="mt-1 text-lg font-semibold text-text-primary tabular-nums">
						{formatNumber(analysis.energy_jumps, $language)}
					</dd>
				</div>
			</dl>

			<div class="flex items-center justify-between">
				<p class="text-xs text-text-secondary tabular-nums">
					{$translate('setMode.duration', { values: { duration: formatDuration(analysis.total_duration_ms) } })}
				</p>
				<Tooltip text={$translate('setMode.autoOrderHint')}>
					<Button
						variant="secondary"
						size="sm"
						disabled={$setModeTrackCount < 2 || $setModeStore.refreshing}
						onclick={() => setModeStore.applySuggestedOrder()}
					>
						<Icon name="shuffle" class="mr-1.5 h-3.5 w-3.5" />
						{$translate('setMode.autoOrder')}
					</Button>
				</Tooltip>
			</div>

			<ul
				class="divide-y divide-stroke-subtle {$setModeStore.refreshing ? 'opacity-60' : ''}"
				aria-busy={$setModeStore.refreshing}
			>
				{#each analysis.entries as entry (entry.track_id)}
					{@const index = entry.position - 1}
					{@const transition = entry.from_previous}
					<li class="flex items-center gap-3 py-2">
						<div class="flex flex-col">
							<IconButton
								ariaLabel={$translate('setMode.moveUp', { values: { title: entry.title } })}
								icon="chevron-down"
								iconClass="rotate-180"
								size="sm"
								disabled={index === 0 || $setModeStore.refreshing}
								onclick={() => setModeStore.moveUp(index)}
							/>
							<IconButton
								ariaLabel={$translate('setMode.moveDown', { values: { title: entry.title } })}
								icon="chevron-down"
								size="sm"
								disabled={index === analysis.entries.length - 1 || $setModeStore.refreshing}
								onclick={() => setModeStore.moveDown(index)}
							/>
						</div>

						<span class="w-6 text-right text-xs text-text-secondary tabular-nums">{entry.position}</span>

						<div class="min-w-0 flex-1">
							<div class="truncate font-semibold text-text-primary">{entry.title}</div>
							<div class="truncate text-xs text-text-secondary">{entry.artist}</div>
							{#if transition}
								<div class="mt-0.5 flex items-center gap-1.5 text-xs text-text-secondary">
									{#if transition.harmonic === 'clash'}
										<Icon name="alert-triangle" class="h-3.5 w-3.5 flex-shrink-0 text-warning" />
									{/if}
									{#if transition.energy_jump}
										<Icon name="flame" class="h-3.5 w-3.5 flex-shrink-0 text-warning" />
									{/if}
									<span>{relationLabel(transition.harmonic)}</span>
									{#if transition.bpm_delta_percent !== null}
										<span class="tabular-nums">
											{$translate('setMode.tempo', {
												values: { delta: formatTempoDelta(transition.bpm_delta_percent, $language) },
											})}
										</span>
									{/if}
								</div>
							{:else}
								<div class="mt-0.5 text-xs text-text-secondary">{$translate('setMode.opening')}</div>
							{/if}
						</div>

						<span class="w-14 text-right text-xs text-text-primary tabular-nums">
							{entry.bpm === null ? '—' : Math.round(entry.bpm)}
						</span>

						{#if entry.key}
							<KeyBadge value={entry.key} variant="tag-wide" />
						{:else}
							<span class="w-10 text-center text-text-secondary">—</span>
						{/if}

						{#if entry.energy !== null}
							<EnergyBadge energy={entry.energy} size="sm" />
						{:else}
							<span class="w-8 text-center text-text-secondary">—</span>
						{/if}

						<IconButton
							ariaLabel={$translate('setMode.removeTrack', { values: { title: entry.title } })}
							icon="close"
							size="sm"
							onclick={() => setModeStore.removeTrack(entry.track_id)}
						/>
					</li>
				{/each}
			</ul>
		{/if}
	</div>

	{#snippet footer()}
		<Button variant="secondary" size="sm" onclick={handleClose}>{$translate('common.close')}</Button>
		<Button variant="primary" size="sm" disabled={$setModeIsEmpty || $setModeStore.exporting} onclick={handleExport}>
			{#if $setModeStore.exporting}
				<Spinner color="current" class="mr-1.5 h-3.5 w-3.5" />
			{/if}
			{$translate('setMode.exportXml')}
		</Button>
	{/snippet}
</Modal>
