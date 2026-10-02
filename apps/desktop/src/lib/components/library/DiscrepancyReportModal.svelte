<script lang="ts">
	import type { MissingFile, ReportTrackRef } from '$shared/types'
	import { discrepancyReportStore } from '$shared/stores/discrepancyReport'
	import { translate } from '$shared/i18n'
	import { Button, Icon, Modal, Spinner, Text, Tooltip } from '$lib/components/common'
	import { open as openDialog } from '@tauri-apps/plugin-dialog'
	import { withNativeDialog } from '$shared/utils'
	import DiscrepancyComparisonSection from './DiscrepancyComparisonSection.svelte'

	type Props = {
		open: boolean
		onClose: () => void
	}

	let { open, onClose }: Props = $props()

	// Loaded fresh every time the modal opens: the report is read only, so there is no state to preserve
	// across sessions, and the library may have changed since the last visit.
	$effect(() => {
		if (open) discrepancyReportStore.load()
	})

	const state = $derived($discrepancyReportStore)
	const report = $derived(state.report)

	const isClean = $derived(
		report !== null &&
			report.missing_files.total === 0 &&
			(report.mixed_in_key === null ||
				(report.mixed_in_key.differences.total === 0 &&
					report.mixed_in_key.missing_from_other.total === 0 &&
					report.mixed_in_key.missing_from_crate.total === 0)) &&
			(report.rekordbox === null ||
				(report.rekordbox.differences.total === 0 &&
					report.rekordbox.missing_from_other.total === 0 &&
					report.rekordbox.missing_from_crate.total === 0))
	)

	function trackLabel(track: ReportTrackRef): string {
		const title = track.title || $translate('common.untitled')
		const artist = track.artist || $translate('common.unknownArtist')
		return `${title} — ${artist}`
	}

	function missingReasonLabel(reason: MissingFile['reason']): string {
		return $translate(`discrepancy.missingFiles.reason.${reason}`)
	}

	function missingReasonHint(reason: MissingFile['reason']): string {
		return $translate(`discrepancy.missingFiles.reasonHint.${reason}`)
	}

	async function handleChooseRekordboxXml() {
		const path = await withNativeDialog(() =>
			openDialog({
				multiple: false,
				filters: [{ name: $translate('discrepancy.rekordbox.xmlFilter'), extensions: ['xml'] }],
				title: $translate('discrepancy.rekordbox.xmlDialogTitle'),
			})
		)
		if (path && typeof path === 'string') {
			discrepancyReportStore.load(path)
		}
	}
</script>

<Modal {open} title={$translate('discrepancy.title')} size="2xl" {onClose}>
	<div class="space-y-5">
		{#if state.loading && report === null}
			<div class="flex items-center gap-2 py-10 text-sm text-text-secondary" aria-busy="true">
				<Spinner />
				{$translate('discrepancy.loading')}
			</div>
		{:else if state.error}
			<div class="flex flex-col items-start gap-3 py-4" role="alert">
				<p class="text-sm text-text-primary">{$translate('discrepancy.loadFailed')}</p>
				<p class="text-xs text-text-secondary">{state.error}</p>
				<Button variant="secondary" size="sm" onclick={() => discrepancyReportStore.retry()}>
					{$translate('common.retry')}
				</Button>
			</div>
		{:else if report}
			<div class="flex items-center justify-between">
				<p class="text-xs text-text-secondary tabular-nums">
					{$translate('discrepancy.subtitle', { values: { count: report.crate_tracks } })}
				</p>
				<Tooltip text={$translate('discrepancy.actions.refreshTitle')} position="bottom" delay={250}>
					<button
						type="button"
						class="inline-flex cursor-pointer items-center gap-1.5 rounded-lg border border-stroke/50 bg-surface-2 px-2.5 py-1 text-xs text-text-secondary transition-colors hover:bg-surface-3 hover:text-text-primary disabled:opacity-50"
						onclick={() => discrepancyReportStore.retry()}
						disabled={state.loading}
					>
						<Icon
							name="refresh-cw"
							class="h-3.5 w-3.5 {state.loading ? 'animate-spin motion-reduce:animate-none' : ''}"
						/>
						{$translate('discrepancy.actions.refresh')}
					</button>
				</Tooltip>
			</div>

			{#if isClean}
				<div class="flex flex-col items-start gap-2 rounded-lg border border-success/30 bg-success/10 p-4">
					<div class="flex items-center gap-2 text-success">
						<Icon name="check" class="h-4 w-4 stroke-[3]" />
						<Text variant="body-1" weight="bold" color="success">{$translate('discrepancy.allClear.title')}</Text>
					</div>
					<p class="text-xs text-text-secondary">{$translate('discrepancy.allClear.body')}</p>
				</div>
			{/if}

			<!-- Missing files -->
			<section aria-labelledby="discrepancy-missing-files-title">
				<h3 id="discrepancy-missing-files-title" class="mb-2 text-sm font-bold text-text-primary">
					{$translate('discrepancy.missingFiles.title')}
					{#if report.missing_files.total > 0}
						<span class="ml-1 font-mono text-xs font-normal text-text-secondary tabular-nums"
							>({report.missing_files.total})</span
						>
					{/if}
				</h3>
				{#if report.missing_files.total === 0}
					<p class="text-xs text-text-secondary">{$translate('discrepancy.missingFiles.empty')}</p>
				{:else}
					<ul class="space-y-1.5">
						{#each report.missing_files.items as entry (entry.track.id ?? entry.track.file_path ?? trackLabel(entry.track))}
							<li
								class="flex items-center justify-between gap-3 rounded-lg border border-stroke/50 bg-surface-2/50 px-2.5 py-1.5"
							>
								<div class="min-w-0">
									<p class="truncate text-xs font-semibold text-text-primary">{trackLabel(entry.track)}</p>
									{#if entry.track.file_path}
										<p class="truncate font-mono text-xs text-text-tertiary">{entry.track.file_path}</p>
									{/if}
								</div>
								<Tooltip text={missingReasonHint(entry.reason)} position="left" delay={250}>
									<span
										class="shrink-0 rounded-full border px-2 py-0.5 text-xs font-semibold {entry.reason === 'missing'
											? 'border-danger/40 bg-danger/15 text-danger'
											: 'border-stroke bg-surface-3 text-text-tertiary'}"
									>
										{missingReasonLabel(entry.reason)}
									</span>
								</Tooltip>
							</li>
						{/each}
					</ul>
					{#if report.missing_files.total > report.missing_files.items.length}
						<p class="mt-1.5 text-xs text-text-tertiary">
							{$translate('discrepancy.missingFiles.truncated', { values: { total: report.missing_files.total } })}
						</p>
					{/if}
				{/if}
			</section>

			<!-- Mixed In Key comparison -->
			<section aria-labelledby="discrepancy-mik-title">
				<h3 id="discrepancy-mik-title" class="mb-2 text-sm font-bold text-text-primary">
					{$translate('discrepancy.mixedInKey.title')}
				</h3>
				{#if report.mixed_in_key === null}
					<p class="text-xs text-text-secondary">{$translate('discrepancy.mixedInKey.unavailable')}</p>
				{:else}
					<DiscrepancyComparisonSection
						toolName={$translate('discrepancy.mixedInKey.title')}
						comparison={report.mixed_in_key}
					/>
				{/if}
			</section>

			<!-- Rekordbox comparison -->
			<section aria-labelledby="discrepancy-rekordbox-title">
				<div class="mb-2 flex items-center justify-between gap-3">
					<h3 id="discrepancy-rekordbox-title" class="text-sm font-bold text-text-primary">
						{$translate('discrepancy.rekordbox.title')}
					</h3>
					{#if report.rekordbox !== null}
						<button
							type="button"
							class="shrink-0 cursor-pointer rounded-lg border border-stroke/50 bg-surface-2 px-2.5 py-1 text-xs text-text-secondary transition-colors hover:bg-surface-3 hover:text-text-primary"
							onclick={handleChooseRekordboxXml}
						>
							{$translate('discrepancy.rekordbox.changeXml')}
						</button>
					{/if}
				</div>
				{#if report.rekordbox === null}
					<div class="flex flex-col items-start gap-2.5 rounded-lg border border-stroke/50 bg-surface-2/40 p-3">
						<p class="text-xs text-text-secondary">{$translate('discrepancy.rekordbox.noXml')}</p>
						<Button variant="secondary" size="sm" onclick={handleChooseRekordboxXml}>
							<Icon name="upload" class="mr-1.5 h-3.5 w-3.5" />
							{$translate('discrepancy.rekordbox.chooseXml')}
						</Button>
					</div>
				{:else}
					<DiscrepancyComparisonSection
						toolName={$translate('discrepancy.rekordbox.title')}
						comparison={report.rekordbox}
					/>
				{/if}
			</section>
		{/if}
	</div>

	{#snippet footer()}
		<Button variant="secondary" size="sm" onclick={onClose}>{$translate('common.close')}</Button>
	{/snippet}
</Modal>
