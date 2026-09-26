<script lang="ts">
	import type { HeatmapCell } from '$shared/types'
	import Icon from '$lib/components/common/Icon.svelte'

	type Props = {
		heatmap: HeatmapCell[]
		isLoading: boolean
	}

	let { heatmap, isLoading }: Props = $props()

	const DAYS = [
		{ id: 1, label: 'Lun', full: 'Lundi' },
		{ id: 2, label: 'Mar', full: 'Mardi' },
		{ id: 3, label: 'Mer', full: 'Mercredi' },
		{ id: 4, label: 'Jeu', full: 'Jeudi' },
		{ id: 5, label: 'Ven', full: 'Vendredi' },
		{ id: 6, label: 'Sam', full: 'Samedi' },
		{ id: 0, label: 'Dim', full: 'Dimanche' },
	]

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
		return found ? found.full : 'Inconnu'
	})

	function getCellIntensityClass(minutes: number, max: number): string {
		if (minutes <= 0) return 'bg-surface-3/40 hover:bg-surface-3 border-transparent'
		const ratio = minutes / max
		if (ratio < 0.25) return 'bg-emerald-500/25 border-emerald-500/30 hover:bg-emerald-500/40 text-emerald-300'
		if (ratio < 0.5) return 'bg-emerald-500/45 border-emerald-500/50 hover:bg-emerald-500/60 text-emerald-200'
		if (ratio < 0.75) return 'bg-emerald-500/70 border-emerald-400 hover:bg-emerald-500/85 text-white'
		return 'bg-emerald-400 border-emerald-300 hover:bg-emerald-300 text-black font-bold shadow-md shadow-emerald-500/20'
	}
</script>

<div class="space-y-4 rounded-2xl border border-stroke/60 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div class="flex items-center gap-2">
			<div class="flex h-7 w-7 items-center justify-center rounded-lg bg-emerald-500/15 text-emerald-400">
				<Icon name="clock" class="h-4 w-4" />
			</div>
			<div>
				<h3 class="text-sm font-bold text-text-primary">Heatmap Hebdomadaire d'Écoute</h3>
				<p class="text-[11px] text-text-tertiary">Matrice horaire 24h × 7 jours</p>
			</div>
		</div>

		{#if peakCell}
			<div
				class="flex items-center gap-1.5 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-3 py-1 text-xs font-medium text-emerald-400"
			>
				<Icon name="flame" class="h-3.5 w-3.5 text-emerald-400" />
				<span>Pic d'écoute : <strong>{peakDayName} à {peakCell.hour_of_day}h00</strong> ({peakCell.minutes} min)</span>
			</div>
		{/if}
	</div>

	{#if isLoading && heatmap.length === 0}
		<div class="bg-surface-3 h-48 w-full animate-pulse rounded-xl"></div>
	{:else}
		<div class="overflow-x-auto pb-2">
			<div class="min-w-[700px] space-y-1.5">
				<!-- Hours Headers (0h, 3h, 6h, 9h, 12h, 15h, 18h, 21h) -->
				<div class="flex items-center pl-10 font-mono text-[10px] text-text-tertiary">
					{#each HOURS as hour (hour)}
						<div class="flex-1 text-center">
							{#if hour % 3 === 0}
								{hour}h
							{/if}
						</div>
					{/each}
				</div>

				<!-- Days Rows -->
				{#each DAYS as day (day.id)}
					<div class="flex items-center gap-2">
						<!-- Day Label -->
						<span class="w-8 text-right font-mono text-[11px] font-bold text-text-tertiary">
							{day.label}
						</span>

						<!-- 24 Hour Cells -->
						<div class="flex flex-1 gap-1">
							{#each HOURS as hour (hour)}
								{@const cell = cellMap.get(`${day.id}_${hour}`)}
								{@const mins = cell?.minutes ?? 0}
								{@const plays = cell?.plays ?? 0}
								<div
									class="group relative flex h-6 flex-1 cursor-pointer items-center justify-center rounded border text-[9px] transition-all {getCellIntensityClass(
										mins,
										maxMinutes
									)}"
									title="{day.full} {hour.toString().padStart(2, '0')}:00 - {(hour + 1)
										.toString()
										.padStart(2, '0')}:00 : {mins} min d'écoute ({plays} titres)"
								>
									<!-- Tooltip on hover -->
									<div
										class="pointer-events-none absolute bottom-full left-1/2 z-30 mb-2 hidden -translate-x-1/2 rounded-lg border border-stroke bg-surface-0/95 px-2.5 py-1.5 text-[11px] font-medium whitespace-nowrap text-text-primary shadow-2xl backdrop-blur-md group-hover:block"
									>
										<div class="font-bold text-emerald-400">{day.full} {hour}h00 - {hour + 1}h00</div>
										<div class="text-text-secondary">{mins} minutes · {plays} titres</div>
									</div>
								</div>
							{/each}
						</div>
					</div>
				{/each}
			</div>

			<!-- Heatmap Scale Legend -->
			<div class="mt-4 flex items-center justify-end gap-2 text-[10px] font-medium text-text-tertiary">
				<span>Moins d'écoute</span>
				<div class="flex items-center gap-1">
					<div class="bg-surface-3/40 h-3 w-3 rounded border border-stroke/40"></div>
					<div class="h-3 w-3 rounded border border-emerald-500/30 bg-emerald-500/25"></div>
					<div class="h-3 w-3 rounded border border-emerald-500/50 bg-emerald-500/50"></div>
					<div class="h-3 w-3 rounded border border-emerald-400 bg-emerald-500/75"></div>
					<div class="h-3 w-3 rounded border border-emerald-300 bg-emerald-400"></div>
				</div>
				<span>Plus d'écoute</span>
			</div>
		</div>
	{/if}
</div>
