<script lang="ts">
	import type { FieldDifference, ReportComparison, ReportTrackRef } from '$shared/types'
	import { translate } from '$shared/i18n'
	import { formatBpm } from '$shared/utils/format'
	import { KeyBadge, Text } from '$lib/components/common'

	type Props = {
		/** "Mixed In Key" or "Rekordbox": used both as the section title and in "In Crate, not in {tool}". */
		toolName: string
		comparison: ReportComparison
	}

	let { toolName, comparison }: Props = $props()

	function trackLabel(track: ReportTrackRef): string {
		const title = track.title || $translate('common.untitled')
		const artist = track.artist || $translate('common.unknownArtist')
		return `${title} — ${artist}`
	}

	/** A value already formatted by the backend (Camelot key, "128.00", an integer…), shown per field. */
	function fieldValue(field: string, value: string | null): string {
		if (value === null) return '—'
		if (field === 'bpm') {
			const parsed = Number.parseFloat(value)
			return Number.isNaN(parsed) ? value : `${formatBpm(parsed)} BPM`
		}
		if (field === 'energy') return `${value}/10`
		if (field === 'cues') return $translate('discrepancy.field.cues') + ': ' + value
		return value
	}

	function fieldNote(diff: FieldDifference): string | null {
		return diff.note ? $translate(`discrepancy.field.note.${diff.note}`) : null
	}
</script>

<div class="space-y-4">
	<!-- Matched count -->
	<p class="text-xs text-text-secondary tabular-nums">
		{$translate('discrepancy.comparison.matched', { values: { count: comparison.matched } })}
	</p>

	<!-- Field-by-field differences -->
	<div>
		<Text variant="body-2" weight="bold" class="mb-2 text-text-primary">
			{$translate('discrepancy.comparison.differencesTitle')}
			{#if comparison.differences.total > 0}
				<span class="ml-1 font-mono text-xs font-normal text-text-secondary tabular-nums"
					>({comparison.differences.total})</span
				>
			{/if}
		</Text>
		{#if comparison.differences.total === 0}
			<p class="text-xs text-text-secondary">{$translate('discrepancy.comparison.differencesEmpty')}</p>
		{:else}
			<ul class="space-y-2">
				{#each comparison.differences.items as diff (diff.track.id ?? diff.track.file_path ?? trackLabel(diff.track))}
					<li class="rounded-lg border border-stroke/60 bg-surface-2/60 p-2.5">
						<Text variant="caption" weight="bold" class="truncate text-text-primary">
							{trackLabel(diff.track)}
						</Text>
						<ul class="mt-1.5 flex flex-wrap gap-1.5">
							{#each diff.fields as field (field.field)}
								<li
									class="inline-flex items-center gap-1 rounded border border-stroke/50 bg-surface-1 px-1.5 py-0.5 font-mono text-xs text-text-secondary"
								>
									<span class="font-semibold text-text-tertiary">{$translate(`discrepancy.field.${field.field}`)}:</span
									>
									{#if field.field === 'key'}
										<KeyBadge value={field.crate_value} variant="tag" />
										<span aria-hidden="true">→</span>
										<KeyBadge value={field.other_value} variant="tag" />
									{:else}
										<span class="text-text-primary">{fieldValue(field.field, field.crate_value)}</span>
										<span aria-hidden="true">→</span>
										<span class="text-text-primary">{fieldValue(field.field, field.other_value)}</span>
									{/if}
									{#if fieldNote(field)}
										<span class="text-warning">({fieldNote(field)})</span>
									{/if}
								</li>
							{/each}
						</ul>
					</li>
				{/each}
			</ul>
			{#if comparison.differences.total > comparison.differences.items.length}
				<p class="mt-1.5 text-xs text-text-tertiary">
					{$translate('discrepancy.comparison.truncated', { values: { total: comparison.differences.total } })}
				</p>
			{/if}
		{/if}
	</div>

	<!-- In Crate, not in the other tool -->
	<div>
		<Text variant="body-2" weight="bold" class="mb-2 text-text-primary">
			{$translate('discrepancy.comparison.missingFromOther', { values: { tool: toolName } })}
			{#if comparison.missing_from_other.total > 0}
				<span class="ml-1 font-mono text-xs font-normal text-text-secondary tabular-nums"
					>({comparison.missing_from_other.total})</span
				>
			{/if}
		</Text>
		{#if comparison.missing_from_other.total === 0}
			<p class="text-xs text-text-secondary">
				{$translate('discrepancy.comparison.missingFromOtherEmpty', { values: { tool: toolName } })}
			</p>
		{:else}
			<ul class="space-y-1">
				{#each comparison.missing_from_other.items as track (track.id ?? track.file_path ?? trackLabel(track))}
					<li
						class="truncate rounded-lg border border-stroke/50 bg-surface-2/40 px-2.5 py-1.5 text-xs text-text-primary"
					>
						{trackLabel(track)}
					</li>
				{/each}
			</ul>
			{#if comparison.missing_from_other.total > comparison.missing_from_other.items.length}
				<p class="mt-1.5 text-xs text-text-tertiary">
					{$translate('discrepancy.comparison.truncated', { values: { total: comparison.missing_from_other.total } })}
				</p>
			{/if}
		{/if}
	</div>

	<!-- In the other tool, not in Crate (import candidates) -->
	<div>
		<Text variant="body-2" weight="bold" class="mb-2 text-text-primary">
			{$translate('discrepancy.comparison.missingFromCrate', { values: { tool: toolName } })}
			{#if comparison.missing_from_crate.total > 0}
				<span class="ml-1 font-mono text-xs font-normal text-text-secondary tabular-nums"
					>({comparison.missing_from_crate.total})</span
				>
			{/if}
		</Text>
		{#if comparison.missing_from_crate.total === 0}
			<p class="text-xs text-text-secondary">
				{$translate('discrepancy.comparison.missingFromCrateEmpty', { values: { tool: toolName } })}
			</p>
		{:else}
			<p class="mb-1.5 text-xs text-text-tertiary">
				{$translate('discrepancy.comparison.missingFromCrateHint', { values: { tool: toolName } })}
			</p>
			<ul class="space-y-1">
				{#each comparison.missing_from_crate.items as track, index (track.id ?? track.file_path ?? `${trackLabel(track)}-${index}`)}
					<li
						class="truncate rounded-lg border border-stroke/50 bg-surface-2/40 px-2.5 py-1.5 text-xs text-text-primary"
					>
						{trackLabel(track)}
					</li>
				{/each}
			</ul>
			{#if comparison.missing_from_crate.total > comparison.missing_from_crate.items.length}
				<p class="mt-1.5 text-xs text-text-tertiary">
					{$translate('discrepancy.comparison.truncated', { values: { total: comparison.missing_from_crate.total } })}
				</p>
			{/if}
		{/if}
	</div>
</div>
