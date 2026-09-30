<script lang="ts">
	import { fade } from 'svelte/transition'
	import type { Track, TrackColor } from '$shared/types'
	import {
		formatDurationCompact,
		formatBpm,
		formatKey,
		formatBitrate,
		formatDate,
		formatCamelotKey,
		getTrackDisplayName,
		getTrackDisplayArtist,
	} from '$shared/utils'
	import { TagChip } from '$lib/components/tags'
	import Icon from '$lib/components/common/Icon.svelte'
	import { AlbumArt, AlbumArtModal, EnergyBadge, KeyBadge, Spinner, Text, Tooltip } from '$lib/components/common'
	import { missingTrackIds, dragStore, isDraggingTag, keyNotationFormat, language, dateFormat } from '$lib/stores'
	import { displaySettingsStore, type ColumnVisibility } from '$shared/stores/displaySettings'
	import { translate } from '$shared/i18n'
	import { DRAG_THRESHOLD, getDistance } from '$shared/utils/drag'
	import TrackColorCell from './TrackColorCell.svelte'

	type Props = {
		track: Track
		selected?: boolean
		playing?: boolean
		analyzing?: boolean
		dragTrackIds?: string[]
		categoryColors?: Map<string, string | null>
		categorySortOrders?: Map<string, number>
		onclick?: (e: MouseEvent) => void
		ondblclick?: (e: MouseEvent) => void
		oncontextmenu?: (e: MouseEvent) => void
		onColorChange?: (color: TrackColor | null) => void
		onCancelAnalysis?: () => void
	}

	let {
		track,
		selected = false,
		playing = false,
		analyzing = false,
		dragTrackIds = [],
		categoryColors,
		categorySortOrders,
		onclick,
		ondblclick,
		oncontextmenu,
		onColorChange,
		onCancelAnalysis,
	}: Props = $props()

	let showArtworkModal = $state(false)
	let isHoveringColorCell = $state(false)
	let isTagDragHovered = $state(false)

	// Clear hover when tag drag ends
	$effect(() => {
		if (!$isDraggingTag) isTagDragHovered = false
	})

	// Track pointer state for drag detection
	let pointerStartPos: { x: number; y: number } | null = null
	let isDragStarted = false

	function handlePointerDown(e: PointerEvent) {
		// Only handle primary button (left click)
		if (e.button !== 0) return

		// Don't start drag on interactive elements
		const target = e.target as HTMLElement
		if (target.closest('button, [role="button"]')) return

		pointerStartPos = { x: e.clientX, y: e.clientY }
		isDragStarted = false
	}

	function handlePointerMove(e: PointerEvent) {
		if (!pointerStartPos) return

		const distance = getDistance(pointerStartPos.x, pointerStartPos.y, e.clientX, e.clientY)

		// Start drag if threshold exceeded
		if (!isDragStarted && distance >= DRAG_THRESHOLD) {
			isDragStarted = true

			// Determine which tracks to drag
			const trackIds = selected && dragTrackIds.length > 0 ? dragTrackIds : [track.id]

			// Start the drag via the store
			dragStore.startTrackDrag(trackIds, e.clientX, e.clientY)
		}
	}

	function handlePointerUp() {
		pointerStartPos = null
		isDragStarted = false
	}

	function handleArtworkClick() {
		if (track.artwork_path) {
			showArtworkModal = true
		}
	}

	const isMissing = $derived($missingTrackIds.has(track.id))

	const columnWidthMap: Record<keyof ColumnVisibility, string> = {
		color: '24px',
		artwork: '40px',
		title: 'minmax(140px, 1.3fr)',
		artist: 'minmax(110px, 1fr)',
		album: 'minmax(110px, 1fr)',
		bpm: '65px',
		key: '55px',
		energy: '65px',
		format: '60px',
		bitrate: '75px',
		duration_ms: '60px',
		tags: 'minmax(110px, 1fr)',
		date_added: '85px',
		rating: '70px',
		genre: '85px',
		label: '85px',
		year: '55px',
		file_size: '70px',
		sample_rate: '70px',
		file_path: 'minmax(140px, 1.5fr)',
	}

	const columnOrder: (keyof ColumnVisibility)[] = [
		'color',
		'artwork',
		'title',
		'artist',
		'album',
		'bpm',
		'key',
		'energy',
		'format',
		'bitrate',
		'duration_ms',
		'tags',
		'date_added',
		'rating',
		'genre',
		'label',
		'year',
		'file_size',
		'sample_rate',
		'file_path',
	]

	const activeColumns = $derived(columnOrder.filter((col) => $displaySettingsStore.columns[col] ?? false))

	const gridTemplateColumns = $derived(activeColumns.map((col) => columnWidthMap[col]).join(' '))
</script>

<div
	role="row"
	tabindex="0"
	data-track-row
	data-track-id={track.id}
	class="group relative grid items-center gap-2 border-b border-stroke-subtle px-3 py-1.5 text-sm transition-colors select-none {selected
		? 'bg-brand-muted text-text-primary'
		: isTagDragHovered
			? 'bg-brand-muted/50 text-text-primary ring-1 ring-brand-primary/50 ring-inset'
			: 'text-text-secondary hover:bg-surface-2 hover:text-text-primary'} {isMissing ? 'opacity-60' : ''}"
	style="grid-template-columns: {gridTemplateColumns};"
	{onclick}
	{ondblclick}
	{oncontextmenu}
	onpointerdown={handlePointerDown}
	onpointermove={handlePointerMove}
	onpointerup={handlePointerUp}
	onpointerenter={() => $isDraggingTag && (isTagDragHovered = true)}
	onpointerleave={() => (isTagDragHovered = false)}
	onkeydown={(e) => {
		if (e.key === 'Enter' || e.key === ' ') {
			// Handled here only: the global Space shortcut must not also toggle playback
			e.preventDefault()
			e.stopPropagation()
			ondblclick?.(e as unknown as MouseEvent)
		}
	}}
>
	{#each activeColumns as col (col)}
		{#if col === 'color'}
			<!-- Color Tag -->
			<div
				class="relative flex h-6 w-6 items-center justify-center"
				onmouseenter={() => (isHoveringColorCell = true)}
				onmouseleave={() => (isHoveringColorCell = false)}
			>
				{#if analyzing}
					{#if isHoveringColorCell && onCancelAnalysis}
						<Tooltip text={$translate('contextMenu.stopAnalysis')} position="right">
							<div transition:fade={{ duration: 150 }}>
								<button
									type="button"
									class="flex h-5 w-5 cursor-pointer items-center justify-center rounded transition-colors hover:bg-red-500/20"
									aria-label={$translate('contextMenu.stopAnalysis')}
									onclick={(e) => {
										e.stopPropagation()
										onCancelAnalysis?.()
									}}
								>
									<Icon name="x" class="h-3 w-3 text-red-500" />
								</button>
							</div>
						</Tooltip>
					{:else}
						<div class="absolute inset-0 flex items-center justify-center" transition:fade={{ duration: 150 }}>
							<Spinner class="h-3 w-3" />
						</div>
					{/if}
				{:else}
					<div transition:fade={{ duration: 150 }}>
						<TrackColorCell color={track.color} onselect={onColorChange} />
					</div>
				{/if}
			</div>
		{:else if col === 'artwork'}
			<!-- Artwork -->
			<div class="flex justify-center">
				<AlbumArt
					artworkPath={track.artwork_path}
					size="xs"
					onclick={handleArtworkClick}
					class={track.artwork_path ? 'cursor-zoom-in' : ''}
				/>
			</div>
		{:else if col === 'title'}
			<!-- Title -->
			<div class="flex items-center truncate font-medium {playing ? 'text-brand-primary' : 'text-text-primary'}">
				{#if isMissing}
					<span class="mr-1.5 flex-shrink-0" title={$translate('library.fileNotFound')}>
						<Icon name="warning" class="h-3.5 w-3.5 text-red-500" />
					</span>
				{/if}
				<span class="truncate">{getTrackDisplayName(track)}</span>
			</div>
		{:else if col === 'artist'}
			<!-- Artist -->
			<div class="truncate text-text-secondary">
				{getTrackDisplayArtist(track)}
			</div>
		{:else if col === 'album'}
			<!-- Album -->
			<div class="truncate text-text-secondary">
				{track.album || '-'}
			</div>
		{:else if col === 'bpm'}
			<!-- BPM -->
			<div class="text-text-secondary tabular-nums">
				{formatBpm(track.bpm)}
			</div>
		{:else if col === 'key'}
			<!-- Key (Authentic MIK Camelot Wheel vs Neutral Grey for non-MIK) -->
			<div class="flex items-center">
				<KeyBadge
					value={track.key}
					variant="cell"
					label={track.key
						? $keyNotationFormat === 'camelot'
							? formatCamelotKey(track.key, $displaySettingsStore.camelotZeroPadding)
							: formatKey(track.key, $keyNotationFormat)
						: undefined}
					analysis={track.analysis_source === 'mixed_in_key' ? 'mik' : 'other'}
				/>
			</div>
		{:else if col === 'energy'}
			<!-- Energy (Mixed In Key Energy Level) -->
			<div class="flex items-center">
				{#if track.analysis_source === 'mixed_in_key' && track.energy}
					<EnergyBadge energy={track.energy} size="sm" />
				{:else}
					<span class="text-xs text-text-tertiary select-none">-</span>
				{/if}
			</div>
		{:else if col === 'format'}
			<!-- Format / File Type (MP3, FLAC, WAV, AIFF, M4A) -->
			<div class="flex items-center">
				{#if track.format}
					<span
						class="rounded border border-stroke bg-surface-2 px-1.5 py-0.5 font-mono text-[10px] font-bold tracking-wider text-text-secondary uppercase"
					>
						{track.format}
					</span>
				{:else}
					<span class="text-xs text-text-tertiary">-</span>
				{/if}
			</div>
		{:else if col === 'bitrate'}
			<!-- Bitrate (Normalized kbps) -->
			<div class="truncate font-mono text-xs text-text-secondary">
				<span>{formatBitrate(track.bitrate, track.format, track.sample_rate)}</span>
			</div>
		{:else if col === 'duration_ms'}
			<!-- Duration -->
			<div class="text-text-secondary tabular-nums">
				{formatDurationCompact(track.duration_ms)}
			</div>
		{:else if col === 'tags'}
			<!-- Tags -->
			<div class="flex h-6 items-center gap-1 overflow-hidden">
				{#each track.tags
					.toSorted((a, b) => {
						const orderA = categorySortOrders?.get(a.category_id) ?? 0
						const orderB = categorySortOrders?.get(b.category_id) ?? 0
						if (orderA !== orderB) return orderA - orderB
						return a.name.localeCompare(b.name)
					})
					.slice(0, 3) as tag (tag.id)}
					<TagChip {tag} size="sm" color={categoryColors?.get(tag.category_id)} />
				{/each}
				{#if track.tags.length > 3}
					<Text variant="caption">+{track.tags.length - 3}</Text>
				{/if}
			</div>
		{:else if col === 'date_added'}
			<!-- Date Added -->
			<div class="truncate text-xs text-text-secondary">
				{track.date_added ? formatDate(track.date_added, $dateFormat, $language) : '-'}
			</div>
		{:else if col === 'rating'}
			<!-- Rating -->
			<div class="truncate text-xs text-text-secondary">
				{track.rating ? '★'.repeat(track.rating) : '-'}
			</div>
		{:else if col === 'genre'}
			<!-- Genre -->
			<div class="truncate text-xs text-text-secondary">
				{track.genre || '-'}
			</div>
		{:else if col === 'label'}
			<!-- Label -->
			<div class="truncate text-xs text-text-secondary">
				{track.label || '-'}
			</div>
		{:else if col === 'year'}
			<!-- Year -->
			<div class="truncate text-xs text-text-secondary tabular-nums">
				{track.year || '-'}
			</div>
		{:else if col === 'file_size'}
			<!-- File Size -->
			<div class="truncate font-mono text-xs text-text-secondary">-</div>
		{:else if col === 'sample_rate'}
			<!-- Sample Rate -->
			<div class="truncate font-mono text-xs text-text-secondary">
				{track.sample_rate ? `${Math.round(track.sample_rate / 1000)} kHz` : '-'}
			</div>
		{:else if col === 'file_path'}
			<!-- File Path -->
			<div class="truncate font-mono text-xs text-text-tertiary" title={track.file_path}>
				{track.file_path}
			</div>
		{/if}
	{/each}
</div>

{#if showArtworkModal}
	<AlbumArtModal
		open={showArtworkModal}
		artworkPath={track.artwork_path}
		trackTitle={getTrackDisplayName(track)}
		onClose={() => (showArtworkModal = false)}
	/>
{/if}
