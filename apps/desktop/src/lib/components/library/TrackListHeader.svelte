<script lang="ts">
	import type { SortConfig, TrackSortField } from '$shared/types'
	import { getNextSortConfig } from '$shared/utils'
	import { translate } from '$shared/i18n'
	import Icon from '$lib/components/common/Icon.svelte'
	import { displaySettingsStore, type ColumnVisibility } from '$shared/stores/displaySettings'
	import { fade } from 'svelte/transition'

	type Props = {
		sortConfig: SortConfig
		onSort?: (config: SortConfig) => void
	}

	let { sortConfig, onSort }: Props = $props()

	type ExtendedTrackSortField = TrackSortField | 'tags'

	type ColumnDef = {
		key: keyof ColumnVisibility
		field: ExtendedTrackSortField | null
		labelKey: string
		/** Accessible name of a sort button whose column shows no label (colour). */
		nameKey?: string
		width: string
	}

	const allColumns: ColumnDef[] = [
		{ key: 'color', field: 'color', labelKey: '', nameKey: 'library.columns.color', width: '24px' },
		{ key: 'artwork', field: null, labelKey: '', width: '40px' },
		{ key: 'title', field: 'title', labelKey: 'library.columns.title', width: 'minmax(140px, 1.3fr)' },
		{ key: 'artist', field: 'artist', labelKey: 'library.columns.artist', width: 'minmax(110px, 1fr)' },
		{ key: 'album', field: 'album', labelKey: 'library.columns.album', width: 'minmax(110px, 1fr)' },
		{ key: 'bpm', field: 'bpm', labelKey: 'library.columns.bpm', width: '65px' },
		{ key: 'key', field: 'key', labelKey: 'library.columns.key', width: '55px' },
		{ key: 'energy', field: 'energy', labelKey: 'library.columns.energy', width: '65px' },
		{ key: 'format', field: 'format', labelKey: 'library.columns.format', width: '60px' },
		{ key: 'bitrate', field: 'bitrate', labelKey: 'library.columns.bitrate', width: '75px' },
		{ key: 'duration_ms', field: 'duration_ms', labelKey: 'library.columns.time', width: '60px' },
		{ key: 'tags', field: 'tags', labelKey: 'library.columns.tags', width: 'minmax(110px, 1fr)' },
		{ key: 'date_added', field: 'date_added', labelKey: 'library.columns.dateAdded', width: '85px' },
		{ key: 'rating', field: 'rating', labelKey: 'library.columns.rating', width: '70px' },
		{ key: 'genre', field: 'genre', labelKey: 'library.columns.genre', width: '85px' },
		{ key: 'label', field: 'label', labelKey: 'library.columns.label', width: '85px' },
		{ key: 'year', field: 'year', labelKey: 'library.columns.year', width: '55px' },
		{ key: 'file_size', field: null, labelKey: 'library.columns.fileSize', width: '70px' },
		{ key: 'sample_rate', field: 'sample_rate', labelKey: 'library.columns.sampleRate', width: '70px' },
		{ key: 'file_path', field: 'file_path', labelKey: 'library.columns.filePath', width: 'minmax(140px, 1.5fr)' },
	]

	const activeColumns = $derived(allColumns.filter((col) => $displaySettingsStore.columns[col.key] ?? false))

	const gridTemplateColumns = $derived(activeColumns.map((col) => col.width).join(' '))

	function handleSort(field: ExtendedTrackSortField) {
		if (field !== 'tags') {
			const newConfig = getNextSortConfig(sortConfig, field)
			onSort?.(newConfig)
		}
	}
</script>

<div
	class="sticky top-0 z-10 grid justify-items-start gap-2 border-b border-stroke bg-surface-1/50 px-3 py-2 text-xs font-medium tracking-wider text-text-tertiary uppercase backdrop-blur-sm"
	style="grid-template-columns: {gridTemplateColumns};"
>
	{#each activeColumns as column (column.key)}
		{#if column.field}
			<button
				type="button"
				class="w-full truncate text-left transition-colors hover:text-text-secondary"
				aria-label={column.nameKey ? $translate(column.nameKey) : undefined}
				onclick={() => column.field && handleSort(column.field)}
			>
				{column.labelKey ? $translate(column.labelKey) : ''}
				{#if column.field && column.field !== 'tags' && sortConfig.field === column.field}
					<span class="inline-block" transition:fade={{ duration: 50 }}>
						<Icon
							name="chevron-down"
							class="ml-1 inline-block h-3 w-3 align-middle transition-transform {sortConfig.direction === 'asc'
								? 'rotate-180'
								: ''}"
						/>
					</span>
				{/if}
			</button>
		{:else}
			<div>{column.labelKey ? $translate(column.labelKey) : ''}</div>
		{/if}
	{/each}
</div>
