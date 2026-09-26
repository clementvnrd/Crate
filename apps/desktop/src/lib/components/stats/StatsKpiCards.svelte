<script lang="ts">
	import type { StatsSummary, TopArtistItem, RekordboxSession } from '$shared/types'
	import Icon from '$lib/components/common/Icon.svelte'

	type Props = {
		summary: StatsSummary | null
		topArtists: TopArtistItem[]
		rekordboxSessions: RekordboxSession[]
		isLoading: boolean
	}

	let { summary, topArtists, rekordboxSessions, isLoading }: Props = $props()

	function formatTotalTime(minutes: number): { primary: string; secondary: string } {
		if (!minutes || minutes <= 0) return { primary: '0 min', secondary: "0 h d'écoute" }
		const hours = Math.floor(minutes / 60)
		const mins = minutes % 60
		if (hours > 0) {
			return {
				primary: `${hours}h ${mins.toString().padStart(2, '0')}m`,
				secondary: `${minutes.toLocaleString()} minutes au total`,
			}
		}
		return {
			primary: `${mins} min`,
			secondary: `${minutes} minutes au total`,
		}
	}

	let timeDisplay = $derived(formatTotalTime(summary?.total_minutes ?? 0))
	let totalPlays = $derived((summary?.total_plays ?? 0).toLocaleString())
	let uniqueArtistsCount = $derived(topArtists.length > 0 ? topArtists.length.toLocaleString() : '0')
	let totalRekordboxMs = $derived(rekordboxSessions.reduce((acc, s) => acc + (s.total_played_ms || 0), 0))
	let rekordboxHours = $derived(Math.round(totalRekordboxMs / 3_600_000))
</script>

<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
	<!-- Card 1: Listening Time -->
	<div
		class="group relative overflow-hidden rounded-2xl border border-emerald-500/20 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl transition-all duration-300 hover:-translate-y-0.5 hover:border-emerald-500/40 hover:shadow-emerald-500/10"
	>
		<div
			class="pointer-events-none absolute -top-6 -right-6 h-28 w-28 rounded-full bg-emerald-500/10 blur-2xl transition-all group-hover:bg-emerald-500/20"
		></div>
		<div class="flex items-start justify-between">
			<div class="space-y-1">
				<span class="text-xs font-semibold tracking-wider text-text-tertiary uppercase"> Temps d'Écoute </span>
				<div class="text-2xl font-black tracking-tight text-text-primary">
					{#if isLoading && !summary}
						<span class="inline-block h-7 w-24 animate-pulse rounded bg-surface-3"></span>
					{:else}
						{timeDisplay.primary}
					{/if}
				</div>
				<p class="text-[11px] text-text-secondary">
					{timeDisplay.secondary}
				</p>
			</div>
			<div
				class="flex h-11 w-11 items-center justify-center rounded-xl border border-emerald-500/30 bg-emerald-500/15 text-emerald-400 shadow-inner transition-transform duration-300 group-hover:scale-110"
			>
				<Icon name="clock" class="h-5 w-5 text-emerald-400" />
			</div>
		</div>
	</div>

	<!-- Card 2: Total Plays -->
	<div
		class="group relative overflow-hidden rounded-2xl border border-purple-500/20 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl transition-all duration-300 hover:-translate-y-0.5 hover:border-purple-500/40 hover:shadow-purple-500/10"
	>
		<div
			class="pointer-events-none absolute -top-6 -right-6 h-28 w-28 rounded-full bg-purple-500/10 blur-2xl transition-all group-hover:bg-purple-500/20"
		></div>
		<div class="flex items-start justify-between">
			<div class="space-y-1">
				<span class="text-xs font-semibold tracking-wider text-text-tertiary uppercase"> Titres Joués </span>
				<div class="text-2xl font-black tracking-tight text-text-primary">
					{#if isLoading && !summary}
						<span class="inline-block h-7 w-20 animate-pulse rounded bg-surface-3"></span>
					{:else}
						{totalPlays}
					{/if}
				</div>
				<p class="text-[11px] text-text-secondary">Écoutes cumulées</p>
			</div>
			<div
				class="flex h-11 w-11 items-center justify-center rounded-xl border border-purple-500/30 bg-purple-500/15 text-purple-400 shadow-inner transition-transform duration-300 group-hover:scale-110"
			>
				<Icon name="music-note" class="h-5 w-5 text-purple-400" />
			</div>
		</div>
	</div>

	<!-- Card 3: Unique Artists -->
	<div
		class="group relative overflow-hidden rounded-2xl border border-pink-500/20 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl transition-all duration-300 hover:-translate-y-0.5 hover:border-pink-500/40 hover:shadow-pink-500/10"
	>
		<div
			class="pointer-events-none absolute -top-6 -right-6 h-28 w-28 rounded-full bg-pink-500/10 blur-2xl transition-all group-hover:bg-pink-500/20"
		></div>
		<div class="flex items-start justify-between">
			<div class="space-y-1">
				<span class="text-xs font-semibold tracking-wider text-text-tertiary uppercase"> Artistes Découverts </span>
				<div class="text-2xl font-black tracking-tight text-text-primary">
					{#if isLoading && !summary}
						<span class="inline-block h-7 w-16 animate-pulse rounded bg-surface-3"></span>
					{:else}
						{uniqueArtistsCount}
					{/if}
				</div>
				<p class="text-[11px] text-text-secondary">Créateurs & producteurs</p>
			</div>
			<div
				class="flex h-11 w-11 items-center justify-center rounded-xl border border-pink-500/30 bg-pink-500/15 text-pink-400 shadow-inner transition-transform duration-300 group-hover:scale-110"
			>
				<Icon name="user" class="h-5 w-5 text-pink-400" />
			</div>
		</div>
	</div>

	<!-- Card 4: Rekordbox DJ Sessions -->
	<div
		class="group relative overflow-hidden rounded-2xl border border-amber-500/20 bg-surface-1/70 p-5 shadow-lg backdrop-blur-xl transition-all duration-300 hover:-translate-y-0.5 hover:border-amber-500/40 hover:shadow-amber-500/10"
	>
		<div
			class="pointer-events-none absolute -top-6 -right-6 h-28 w-28 rounded-full bg-amber-500/10 blur-2xl transition-all group-hover:bg-amber-500/20"
		></div>
		<div class="flex items-start justify-between">
			<div class="space-y-1">
				<span class="text-xs font-semibold tracking-wider text-text-tertiary uppercase"> Sessions DJ Rekordbox </span>
				<div class="text-2xl font-black tracking-tight text-text-primary">
					{#if isLoading && !summary}
						<span class="inline-block h-7 w-16 animate-pulse rounded bg-surface-3"></span>
					{:else}
						{rekordboxSessions.length}
					{/if}
				</div>
				<p class="text-[11px] text-text-secondary">
					{rekordboxHours > 0 ? `${rekordboxHours}h de mix enregistrées` : 'Mix & Live sets'}
				</p>
			</div>
			<div
				class="flex h-11 w-11 items-center justify-center rounded-xl border border-amber-500/30 bg-amber-500/15 text-amber-400 shadow-inner transition-transform duration-300 group-hover:scale-110"
			>
				<Icon name="activity" class="h-5 w-5 text-amber-400" />
			</div>
		</div>
	</div>
</div>
