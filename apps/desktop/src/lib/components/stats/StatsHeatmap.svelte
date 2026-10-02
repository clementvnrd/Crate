<script lang="ts">
	import type { HeatmapCell } from '$shared/types'
	import { translate } from '$shared/i18n'
	import { language } from '$lib/stores'
	import { formatNumber } from '$shared/utils/format'
	import { Icon, Tooltip } from '$lib/components/common'
	import { heatmapCellForKey } from './heatmapKeys'

	type Props = {
		heatmap: HeatmapCell[]
		isLoading: boolean
	}

	let { heatmap, isLoading }: Props = $props()

	// Rows run Monday to Sunday; ids follow the backend's day_of_week (0 = Sunday)
	const DAY_IDS = [1, 2, 3, 4, 5, 6, 0]

	// 1 January 2023 was a Sunday, so this date falls on the weekday `id`
	const weekdayDate = (id: number) => new Date(2023, 0, 1 + id)

	// Day names come from the app language, not from hand-written labels. They are row labels and the start of a
	// tooltip line, so they start with a capital and drop the abbreviation dot ("lun." -> "Lun", "lundi" -> "Lundi").
	const asLabel = (name: string, locale: string) =>
		name.charAt(0).toLocaleUpperCase(locale) + name.slice(1).replace(/\.$/, '')

	let DAYS = $derived.by(() => {
		const short = new Intl.DateTimeFormat($language, { weekday: 'short' })
		const long = new Intl.DateTimeFormat($language, { weekday: 'long' })
		return DAY_IDS.map((id) => ({
			id,
			label: asLabel(short.format(weekdayDate(id)), $language),
			full: asLabel(long.format(weekdayDate(id)), $language),
		}))
	})

	const HOURS = Array.from({ length: 24 }, (_, i) => i)

	// Map cells by "day_hour" key
	let cellMap = $derived.by(() => {
		// eslint-disable-next-line svelte/prefer-svelte-reactivity -- lookup rebuilt inside $derived, never mutated afterwards
		const map = new Map<string, HeatmapCell>()
		for (const cell of heatmap) {
			map.set(`${cell.day_of_week}_${cell.hour_of_day}`, cell)
		}
		return map
	})

	let maxMinutes = $derived(Math.max(...heatmap.map((c) => c.minutes), 1))

	// Find the peak cell
	let peakCell = $derived.by(() => {
		if (heatmap.length === 0) return null
		let top = heatmap[0]
		for (const cell of heatmap) {
			if (cell.minutes > top.minutes) {
				top = cell
			}
		}
		return top.minutes > 0 ? top : null
	})

	let peakDayName = $derived.by(() => {
		if (!peakCell) return ''
		const found = DAYS.find((d) => d.id === peakCell.day_of_week)
		return found ? found.full : $translate('common.unknown')
	})

	const hourLabel = (hour: number) => `${hour.toString().padStart(2, '0')}:00`

	function cellDescription(dayName: string, hour: number, minutes: number, plays: number): string {
		return $translate('stats.heatmap.cell', {
			values: { day: dayName, from: hourLabel(hour), to: hourLabel(hour + 1), minutes, plays },
		})
	}

	// Keyboard: the grid is one tab stop; arrows move the active cell (heatmapKeys.ts), whose tooltip opens and which is
	// announced through aria-activedescendant. Only for keyboard focus: a heatmap focused by a click leaves the arrows
	// to the global seek and volume shortcuts, like the Player's recent files.
	const uid = $props.id()
	const cellId = (row: number, col: number) => `${uid}-cell-${row}-${col}`

	let gridEl: HTMLDivElement | undefined = $state()
	let activeRow = $state(0)
	let activeCol = $state(0)
	let keyboardActive = $state(false)

	function handleGridFocus() {
		keyboardActive = !!gridEl?.matches(':focus-visible')
	}

	function handleGridKeydown(e: KeyboardEvent) {
		if (e.target !== e.currentTarget || !gridEl?.matches(':focus-visible')) return
		const next = heatmapCellForKey(e, { row: activeRow, col: activeCol }, DAYS.length, HOURS.length)
		if (!next) return
		e.preventDefault()
		e.stopPropagation()
		keyboardActive = true
		activeRow = next.row
		activeCol = next.col
		document.getElementById(cellId(next.row, next.col))?.scrollIntoView({ block: 'nearest', inline: 'nearest' })
	}

	function getCellIntensityClass(minutes: number, max: number): string {
		if (minutes <= 0) return 'bg-surface-3/40 hover:bg-surface-3 border-transparent'
		const ratio = minutes / max
		if (ratio < 0.25) return 'bg-pulse-listening/25 border-pulse-listening/30 hover:bg-pulse-listening/40'
		if (ratio < 0.5) return 'bg-pulse-listening/45 border-pulse-listening/50 hover:bg-pulse-listening/60'
		if (ratio < 0.75) return 'bg-pulse-listening/70 border-pulse-listening-bright hover:bg-pulse-listening/85'
		return 'bg-pulse-listening-bright border-pulse-listening-brighter hover:bg-pulse-listening-brighter shadow-md shadow-pulse-listening/20'
	}
</script>

<div class="space-y-4 rounded-xl border border-stroke/60 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div class="flex items-center gap-2">
			<div class="flex h-7 w-7 items-center justify-center rounded-lg bg-pulse-listening/15 text-pulse-listening-text">
				<Icon name="clock" class="h-4 w-4" />
			</div>
			<div>
				<h3 class="text-sm font-bold text-text-primary">{$translate('stats.heatmap.title')}</h3>
				<p class="text-[11px] text-text-tertiary">{$translate('stats.heatmap.subtitle')}</p>
			</div>
		</div>

		{#if peakCell}
			<div
				class="flex items-center gap-1.5 rounded-full border border-pulse-listening/30 bg-pulse-listening/10 px-3 py-1 text-xs font-medium text-pulse-listening-text"
			>
				<Icon name="flame" class="h-3.5 w-3.5 text-pulse-listening-text" />
				<span
					>{$translate('stats.heatmap.peak')}
					<strong
						>{$translate('stats.heatmap.peakAt', { values: { day: peakDayName, hour: peakCell.hour_of_day } })}</strong
					>
					({$translate('stats.duration.minutes', {
						values: { minutes: formatNumber(peakCell.minutes, $language) },
					})})</span
				>
			</div>
		{/if}
	</div>

	{#if isLoading && heatmap.length === 0}
		<div class="h-48 w-full animate-pulse rounded-xl bg-surface-3 motion-reduce:animate-none"></div>
	{:else}
		<div class="overflow-x-auto pb-2">
			<div
				bind:this={gridEl}
				class="group/heatmap min-w-[700px] space-y-1.5 focus-visible:outline-none"
				role="grid"
				tabindex="0"
				aria-label={$translate('stats.heatmap.title')}
				aria-activedescendant={cellId(activeRow, activeCol)}
				onfocus={handleGridFocus}
				onblur={() => (keyboardActive = false)}
				onkeydown={handleGridKeydown}
			>
				<!-- Hours Headers (0h, 3h, 6h, 9h, 12h, 15h, 18h, 21h): every cell names its own hours -->
				<div class="flex items-center pl-10 font-mono text-[10px] text-text-tertiary" aria-hidden="true">
					{#each HOURS as hour (hour)}
						<div class="flex-1 text-center">
							{#if hour % 3 === 0}
								{$translate('stats.heatmap.hourTick', { values: { hour } })}
							{/if}
						</div>
					{/each}
				</div>

				<!-- Days Rows -->
				{#each DAYS as day, row (day.id)}
					<div class="flex items-center gap-2" role="row">
						<!-- Day Label -->
						<span class="w-8 text-right font-mono text-[11px] font-bold text-text-tertiary" role="rowheader">
							{day.label}
						</span>

						<!-- 24 Hour Cells -->
						<div class="flex flex-1 gap-1">
							{#each HOURS as hour (hour)}
								{@const cell = cellMap.get(`${day.id}_${hour}`)}
								{@const mins = cell?.minutes ?? 0}
								{@const plays = cell?.plays ?? 0}
								{@const description = cellDescription(day.full, hour, mins, plays)}
								<!-- Same bubble as before (instant, two lines); the cell is described for screen readers -->
								{@const active = row === activeRow && hour === activeCol}
								<Tooltip
									text={description}
									wrapperClass="flex flex-1"
									wrapperRole="presentation"
									open={keyboardActive && active}
									fade={false}
									bubbleClass="rounded-lg border border-stroke bg-surface-0/95 px-2.5 py-1.5 text-[11px] font-medium whitespace-nowrap text-text-primary shadow-2xl backdrop-blur-md"
								>
									<div
										id={cellId(row, hour)}
										role="gridcell"
										aria-label={description}
										aria-selected={active}
										title={description}
										class="flex h-6 flex-1 items-center justify-center rounded border text-[9px] transition-all {getCellIntensityClass(
											mins,
											maxMinutes
										)} {active
											? 'group-focus-visible/heatmap:outline-2 group-focus-visible/heatmap:outline-offset-1 group-focus-visible/heatmap:outline-brand-primary'
											: ''}"
									></div>
									{#snippet content()}
										<div class="font-bold text-pulse-listening-text">
											{$translate('stats.heatmap.tooltipRange', {
												values: { day: day.full, from: hour, to: hour + 1 },
											})}
										</div>
										<div class="text-text-secondary">
											{$translate('stats.heatmap.tooltipDetail', { values: { minutes: mins, plays } })}
										</div>
									{/snippet}
								</Tooltip>
							{/each}
						</div>
					</div>
				{/each}
			</div>

			<!-- Heatmap Scale Legend -->
			<div class="mt-4 flex items-center justify-end gap-2 text-[10px] font-medium text-text-tertiary">
				<span>{$translate('stats.heatmap.less')}</span>
				<div class="flex items-center gap-1">
					<div class="h-3 w-3 rounded border border-stroke/40 bg-surface-3/40"></div>
					<div class="h-3 w-3 rounded border border-pulse-listening/30 bg-pulse-listening/25"></div>
					<div class="h-3 w-3 rounded border border-pulse-listening/50 bg-pulse-listening/50"></div>
					<div class="h-3 w-3 rounded border border-pulse-listening-bright bg-pulse-listening/75"></div>
					<div class="h-3 w-3 rounded border border-pulse-listening-brighter bg-pulse-listening-bright"></div>
				</div>
				<span>{$translate('stats.heatmap.more')}</span>
			</div>
		</div>
	{/if}
</div>
