<script lang="ts">
	import type { MoveStatus, OrganisationBatch, OrganisationRule } from '$shared/types'
	import {
		organisationStore,
		organisationPlan,
		isPlanningOrganisation,
		organisationPlanError,
		isApplyingOrganisation,
		organisationApplyResult,
		organisationBatches,
		isLoadingOrganisationBatches,
		undoingOrganisationBatchId,
	} from '$shared/stores/organisation'
	import { translate } from '$shared/i18n'
	import { language } from '$lib/stores'
	import { toastStore } from '$shared/stores/toast'
	import { formatDate, formatNumber } from '$shared/utils/format'
	import { withNativeDialog } from '$shared/utils'
	import { open as openNativeDialog } from '@tauri-apps/plugin-dialog'
	import { Button, ConfirmModal, Icon, IconButton, Input, Modal, Spinner, Text } from '$lib/components/common'

	// Assisted physical organisation (CRA-130): the owner picks a folder/naming rule, previews every
	// planned move (a dry run that touches nothing), and only then can apply it. Applying always sends
	// back the exact plan that was previewed (defect C6: never move a file without a shown preview),
	// and every applied run can be undone from the "Past runs" list below.

	type Props = {
		open: boolean
		onClose: () => void
	}

	let { open: isOpen, onClose }: Props = $props()

	let destinationRoot = $state('')
	let template = $state('{artist}/{album}/{artist} - {title}')
	let selectedPreset = $state('artistAlbumTrack')

	$effect(() => {
		if (isOpen) organisationStore.loadBatches()
	})

	type Preset = { value: string; template: string }
	function presets(): (Preset & { label: string })[] {
		return [
			{ value: 'artistAlbumTrack', template: '{artist}/{album}/{artist} - {title}' },
			{ value: 'artistAlbumTitle', template: '{artist}/{album}/{title}' },
			{ value: 'genreArtistTitle', template: '{genre}/{artist}/{title}' },
		].map((preset) => ({ ...preset, label: $translate(`organisation.rule.presets.${preset.value}`) }))
	}

	function selectPreset(preset: Preset) {
		selectedPreset = preset.value
		template = preset.template
	}

	function handleTemplateInput() {
		selectedPreset = ''
	}

	async function handleBrowse() {
		const selected = await withNativeDialog(() =>
			openNativeDialog({ directory: true, multiple: false, title: $translate('organisation.rule.browseTitle') })
		)
		if (selected && typeof selected === 'string') destinationRoot = selected
	}

	const rule = $derived<OrganisationRule>({
		destination_root: destinationRoot.trim(),
		template: template.trim(),
	})
	const canRunDryRun = $derived(destinationRoot.trim() !== '' && template.trim() !== '' && !$isPlanningOrganisation)

	async function handleRunDryRun() {
		await organisationStore.planDryRun(rule)
	}

	// Applying always goes through this confirmation: the owner sees how many files move and must
	// tick, every time, that tools like Rekordbox will lose track of them.
	let showApplyConfirm = $state(false)
	let applyAcknowledged = $state(false)

	function openApplyConfirm() {
		if (!$organisationPlan || $organisationPlan.to_move === 0) return
		applyAcknowledged = false
		showApplyConfirm = true
	}

	function cancelApplyConfirm() {
		showApplyConfirm = false
	}

	async function confirmApply(checked: boolean) {
		if (!checked) {
			toastStore.error($translate('organisation.toast.mustAcknowledge'))
			return
		}
		showApplyConfirm = false
		await organisationStore.apply(rule, true)
	}

	let undoTarget = $state<OrganisationBatch | null>(null)

	function openUndoConfirm(batch: OrganisationBatch) {
		undoTarget = batch
	}

	function cancelUndoConfirm() {
		undoTarget = null
	}

	async function confirmUndo() {
		if (!undoTarget) return
		const batchId = undoTarget.batch_id
		undoTarget = null
		await organisationStore.undo(batchId)
	}

	function statusMeta(status: MoveStatus): { label: string; classes: string } {
		switch (status) {
			case 'move':
				return {
					label: $translate('organisation.status.move'),
					classes: 'border-brand-primary/40 bg-brand-muted text-brand-primary',
				}
			case 'already_in_place':
				return {
					label: $translate('organisation.status.alreadyInPlace'),
					classes: 'border-success/30 bg-success/10 text-success',
				}
			case 'source_missing':
				return {
					label: $translate('organisation.status.sourceMissing'),
					classes: 'border-danger/30 bg-danger/10 text-danger',
				}
			case 'target_exists':
				return {
					label: $translate('organisation.status.targetExists'),
					classes: 'border-warning/30 bg-warning/10 text-warning',
				}
			case 'cross_volume':
				return {
					label: $translate('organisation.status.crossVolume'),
					classes: 'border-warning/30 bg-warning/10 text-warning',
				}
		}
	}
</script>

<Modal open={isOpen} {onClose} size="4xl" flush>
	<div class="flex min-h-0 flex-1 flex-col">
		<div class="flex items-center justify-between border-b border-stroke px-6 py-4">
			<Text variant="header-1" weight="semibold">{$translate('organisation.title')}</Text>
			<IconButton icon="x" ariaLabel={$translate('common.close')} onclick={onClose} />
		</div>

		<div class="min-h-0 flex-1 space-y-6 overflow-y-auto p-6">
			<Text variant="body-2" color="secondary">{$translate('organisation.subtitle')}</Text>

			<!-- Rule configuration -->
			<section class="space-y-3 rounded-lg border border-stroke bg-surface-2/50 p-4">
				<Text variant="header-3">{$translate('organisation.rule.heading')}</Text>

				<label class="block">
					<Text as="span" variant="caption" weight="medium" class="mb-1 block">
						{$translate('organisation.rule.destination')}
					</Text>
					<div class="flex gap-2">
						<Input
							bind:value={destinationRoot}
							placeholder={$translate('organisation.rule.destinationPlaceholder')}
							class="flex-1"
						/>
						<Button variant="secondary" onclick={handleBrowse}>
							<Icon name="folder" class="mr-1.5 h-4 w-4" />
							{$translate('organisation.rule.browse')}
						</Button>
					</div>
				</label>

				<div>
					<Text as="span" variant="caption" weight="medium" class="mb-1 block">
						{$translate('organisation.rule.presets.heading')}
					</Text>
					<div class="flex flex-wrap gap-2">
						{#each presets() as preset (preset.value)}
							<button
								type="button"
								class="rounded-md border px-3 py-1.5 text-xs transition-colors hover:cursor-pointer {selectedPreset ===
								preset.value
									? 'border-brand-primary bg-brand-muted text-brand-primary'
									: 'border-stroke text-text-secondary hover:border-text-tertiary hover:text-text-primary'}"
								onclick={() => selectPreset(preset)}
							>
								{preset.label}
							</button>
						{/each}
					</div>
				</div>

				<label class="block">
					<Text as="span" variant="caption" weight="medium" class="mb-1 block">
						{$translate('organisation.rule.template')}
					</Text>
					<Input bind:value={template} oninput={handleTemplateInput} class="font-mono" />
					<Text variant="caption" color="tertiary" class="mt-1 block">
						{$translate('organisation.rule.templateHelp')}
					</Text>
				</label>

				<div class="flex justify-end">
					<Button variant="primary" onclick={handleRunDryRun} disabled={!canRunDryRun}>
						{#if $isPlanningOrganisation}
							<Spinner class="mr-2 h-4 w-4" color="current" />
							{$translate('organisation.rule.computing')}
						{:else}
							{$translate('organisation.rule.runDryRun')}
						{/if}
					</Button>
				</div>
			</section>

			{#if $organisationPlanError}
				<div class="flex items-start gap-2 rounded-md border border-danger/30 bg-danger/10 p-3" role="alert">
					<Icon name="alert-triangle" class="mt-0.5 h-4 w-4 flex-shrink-0 text-danger" />
					<div>
						<Text variant="body-2" weight="semibold" color="danger">{$translate('organisation.planError')}</Text>
						<Text variant="caption" color="secondary">{$organisationPlanError}</Text>
					</div>
				</div>
			{/if}

			{#if $organisationPlan}
				{@const plan = $organisationPlan}
				<section class="space-y-4">
					<dl class="grid grid-cols-3 gap-3">
						<div class="rounded-lg bg-surface-2 p-3">
							<dt class="text-xs text-text-secondary">{$translate('organisation.summary.toMove')}</dt>
							<dd class="mt-1 text-lg font-semibold text-text-primary tabular-nums">
								{formatNumber(plan.to_move, $language)}
							</dd>
						</div>
						<div class="rounded-lg bg-surface-2 p-3">
							<dt class="text-xs text-text-secondary">{$translate('organisation.summary.alreadyInPlace')}</dt>
							<dd class="mt-1 text-lg font-semibold text-text-primary tabular-nums">
								{formatNumber(plan.already_in_place, $language)}
							</dd>
						</div>
						<div class="rounded-lg bg-surface-2 p-3">
							<dt class="text-xs text-text-secondary">{$translate('organisation.summary.blocked')}</dt>
							<dd
								class="mt-1 text-lg font-semibold tabular-nums {plan.blocked > 0
									? 'text-warning'
									: 'text-text-primary'}"
							>
								{formatNumber(plan.blocked, $language)}
							</dd>
						</div>
					</dl>

					{#if plan.to_move === 0 && plan.blocked === 0}
						<div class="flex flex-col items-center gap-3 py-10 text-center">
							<div
								class="flex h-12 w-12 items-center justify-center rounded-lg border border-success/30 bg-success/10 text-success"
							>
								<Icon name="check" class="h-6 w-6" />
							</div>
							<Text variant="header-2">{$translate('organisation.empty.title')}</Text>
							<Text variant="body-2" color="secondary">{$translate('organisation.empty.body')}</Text>
						</div>
					{:else}
						{#if plan.blocked > 0}
							<div class="flex items-start gap-2 rounded-md border border-warning/30 bg-warning/10 p-3">
								<Icon name="alert-triangle" class="mt-0.5 h-4 w-4 flex-shrink-0 text-warning" />
								<Text variant="body-2" color="warning">
									{$translate('organisation.warning.blocked', { values: { count: plan.blocked } })}
								</Text>
							</div>
						{/if}

						<div class="overflow-hidden rounded-lg border border-stroke">
							<div class="max-h-96 overflow-y-auto">
								<table class="w-full table-fixed text-xs">
									<thead class="sticky top-0 bg-surface-2">
										<tr class="border-b border-stroke-subtle text-left text-text-secondary">
											<th scope="col" class="w-1/5 px-3 py-2 font-medium">{$translate('organisation.table.track')}</th>
											<th scope="col" class="px-3 py-2 font-medium">{$translate('organisation.table.from')}</th>
											<th scope="col" class="px-3 py-2 font-medium">{$translate('organisation.table.to')}</th>
											<th scope="col" class="w-32 px-3 py-2 font-medium">{$translate('organisation.table.status')}</th>
										</tr>
									</thead>
									<tbody>
										{#each plan.moves as move (move.track_id)}
											{@const meta = statusMeta(move.status)}
											<tr class="border-b border-stroke-subtle last:border-b-0">
												<td class="min-w-0 px-3 py-2">
													<div class="truncate font-medium text-text-primary">{move.title}</div>
													<div class="truncate text-text-secondary">{move.artist}</div>
												</td>
												<td class="min-w-0 truncate px-3 py-2 font-mono text-text-secondary" title={move.from}>
													{move.from}
												</td>
												<td class="min-w-0 truncate px-3 py-2 font-mono text-text-secondary" title={move.to}>
													{move.to}
												</td>
												<td class="px-3 py-2">
													<span
														class="inline-flex items-center rounded border px-2 py-0.5 text-xs font-medium {meta.classes}"
													>
														{meta.label}
													</span>
												</td>
											</tr>
										{/each}
									</tbody>
								</table>
							</div>
						</div>

						<div
							class="flex items-center justify-between gap-3 rounded-md border border-stroke-subtle bg-surface-2/40 p-3"
						>
							<Text variant="caption" color="secondary">{$translate('organisation.warning.externalTools')}</Text>
							<Button
								variant="primary"
								onclick={openApplyConfirm}
								disabled={plan.to_move === 0 || $isApplyingOrganisation}
							>
								{#if $isApplyingOrganisation}
									<Spinner class="mr-2 h-4 w-4" color="current" />
									{$translate('organisation.apply.applying')}
								{:else}
									{$translate('organisation.apply.button', { values: { count: plan.to_move } })}
								{/if}
							</Button>
						</div>
					{/if}

					{#if $organisationApplyResult && $organisationApplyResult.failed.length > 0}
						<div class="space-y-1 rounded-md border border-danger/30 bg-danger/10 p-3" role="alert">
							<Text variant="body-2" weight="semibold" color="danger">
								{$translate('organisation.result.partialFailureTitle', {
									values: { count: $organisationApplyResult.failed.length },
								})}
							</Text>
							<ul class="list-disc space-y-0.5 pl-5">
								{#each $organisationApplyResult.failed as failure (failure.track_id)}
									<li class="text-text-secondary">
										<span class="font-mono">{failure.track_id}</span> — {failure.reason}
									</li>
								{/each}
							</ul>
						</div>
					{/if}
				</section>
			{/if}

			<!-- Past runs: every applied batch can be undone, one file at a time if something blocks it. -->
			<section class="space-y-3 border-t border-stroke-subtle pt-5">
				<Text variant="header-3">{$translate('organisation.batches.heading')}</Text>

				{#if $isLoadingOrganisationBatches && $organisationBatches.length === 0}
					<div class="flex items-center gap-2 text-sm text-text-secondary" aria-busy="true">
						<Spinner />
						{$translate('common.loading')}
					</div>
				{:else if $organisationBatches.length === 0}
					<Text variant="body-2" color="secondary">{$translate('organisation.batches.empty')}</Text>
				{:else}
					<ul class="divide-y divide-stroke-subtle rounded-lg border border-stroke">
						{#each $organisationBatches as batch (batch.batch_id)}
							{@const fullyUndone = batch.undone >= batch.files}
							<li class="flex items-center justify-between gap-3 px-4 py-3">
								<div>
									<Text variant="body-2" weight="medium">
										{$translate('organisation.batches.summary', { values: { files: batch.files } })}
									</Text>
									<Text variant="caption" color="secondary" class="tabular-nums">
										{formatDate(batch.moved_at, 'locale', $language)}
										{#if batch.undone > 0}
											· {$translate('organisation.batches.undoneCount', { values: { count: batch.undone } })}
										{/if}
									</Text>
								</div>
								<Button
									variant="secondary"
									size="sm"
									onclick={() => openUndoConfirm(batch)}
									disabled={fullyUndone || $undoingOrganisationBatchId === batch.batch_id}
								>
									{#if $undoingOrganisationBatchId === batch.batch_id}
										<Spinner class="mr-1.5 h-3.5 w-3.5" />
										{$translate('organisation.batches.undoing')}
									{:else if fullyUndone}
										{$translate('organisation.batches.fullyUndone')}
									{:else}
										{$translate('organisation.batches.undo')}
									{/if}
								</Button>
							</li>
						{/each}
					</ul>
				{/if}
			</section>
		</div>
	</div>
</Modal>

<ConfirmModal
	open={showApplyConfirm}
	title={$translate('modals.confirm.applyOrganisationTitle')}
	message={$translate('modals.confirm.applyOrganisationMessage', {
		values: { count: $organisationPlan?.to_move ?? 0 },
	})}
	warnings={[$translate('modals.confirm.applyOrganisationWarning')]}
	checkboxLabel={$translate('modals.confirm.applyOrganisationCheckbox')}
	bind:checkboxChecked={applyAcknowledged}
	confirmLabel={$translate('organisation.apply.button', { values: { count: $organisationPlan?.to_move ?? 0 } })}
	destructive
	onConfirm={confirmApply}
	onCancel={cancelApplyConfirm}
/>

<ConfirmModal
	open={undoTarget !== null}
	title={$translate('modals.confirm.undoOrganisationTitle')}
	message={$translate('modals.confirm.undoOrganisationMessage', { values: { count: undoTarget?.files ?? 0 } })}
	confirmLabel={$translate('organisation.batches.undo')}
	onConfirm={confirmUndo}
	onCancel={cancelUndoConfirm}
/>
