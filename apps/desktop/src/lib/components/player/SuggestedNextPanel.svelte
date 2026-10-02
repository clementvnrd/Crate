<script lang="ts">
	import { get } from 'svelte/store'
	import { revealItemInDir } from '@tauri-apps/plugin-opener'
	import type { NextTrackSuggestion, Track } from '$shared/types'
	import { translate } from '$shared/i18n'
	import { toastStore } from '$shared/stores/toast'
	import { formatBpm, formatDate, formatKey, getArtworkUrl } from '$shared/utils'
	import { formatTempoDelta } from '$lib/components/stats/format'
	import { isClash, toUtcIso } from './suggestionFormat'
	import {
		playerStore,
		appDataDir,
		language,
		suggestionsStore,
		nextTrackSuggestions,
		suggestionsLoading,
		suggestionsError,
	} from '$lib/stores'
	import { Button, Icon, KeyBadge, Spinner, Tooltip } from '$lib/components/common'

	type Props = {
		/** Library track id to suggest from; `null` when there is nothing eligible (see `hasTrack`). */
		trackId: string | null
		/** Whether a track is shown in the Hero at all, to tell "nothing playing" from "playing, not in library". */
		hasTrack: boolean
		/** Title of the Hero track, for the "not in library" hint. */
		trackTitle?: string | null
		/** Reuses the Hero's own "Add to library" action so this panel never duplicates the import logic. */
		onImportToLibrary?: () => void
		importing?: boolean
	}

	let { trackId, hasTrack, trackTitle = null, onImportToLibrary, importing = false }: Props = $props()

	// Reload whenever the eligible track changes (including back to none, which clears the list).
	$effect(() => {
		if (trackId) {
			suggestionsStore.load(trackId)
		} else {
			suggestionsStore.clear()
		}
	})

	function relationLabel(relation: NextTrackSuggestion['harmonic']): string {
		return $translate(`stats.timeline.relation.${relation}`)
	}

	function historyTooltip(suggestion: NextTrackSuggestion): string {
		const count = suggestion.times_played_after
		if (!suggestion.last_played_after) {
			return $translate('player.suggestions.historyTooltipNoDate', { values: { count } })
		}
		const date = formatDate(toUtcIso(suggestion.last_played_after), 'locale', $language)
		return $translate('player.suggestions.historyTooltipWithDate', { values: { count, date } })
	}

	async function handlePlay(track: Track) {
		await playerStore.play(track)
		const state = get(playerStore)
		if (state.error) {
			toastStore.error(
				$translate('player.suggestions.playFailed', {
					values: { title: track.title || $translate('player.trackFallback') },
				})
			)
			suggestionsStore.removeSuggestion(track.id)
		}
	}

	async function handleReveal(filePath: string) {
		try {
			await revealItemInDir(filePath)
		} catch (err) {
			console.error('Reveal in finder error:', err)
			toastStore.error($translate('player.toast.revealFailed'))
		}
	}

	function handleRetry() {
		if (trackId) suggestionsStore.load(trackId)
	}
</script>

<div class="flex h-full min-h-0 flex-col">
	<div
		class="z-20 w-full flex-shrink-0 border-y border-stroke-subtle bg-surface-0/90 px-6 py-2 shadow-xs backdrop-blur-md"
	>
		<div class="flex items-center gap-2.5 pb-2">
			<Icon name="sparkles" class="h-4 w-4 text-cyan-600 dark:text-cyan-400" />
			<h2 class="text-sm font-bold text-text-primary">{$translate('player.suggestions.title')}</h2>
		</div>
		<p class="border-t border-stroke-subtle pt-2 text-[11px] text-text-tertiary">
			{$translate('player.suggestions.subtitle')}
		</p>
	</div>

	<div class="min-h-0 flex-1 overflow-y-auto scroll-smooth px-6 py-3">
		{#if !hasTrack}
			<div class="flex flex-col items-center justify-center py-12 text-center">
				<div
					class="mb-2.5 flex h-12 w-12 items-center justify-center rounded-2xl border border-stroke-subtle bg-surface-2 text-cyan-600 shadow-inner dark:text-cyan-400/60"
				>
					<Icon name="sparkles" class="h-6 w-6" />
				</div>
				<h3 class="text-sm font-semibold text-text-primary">{$translate('player.suggestions.noTrackTitle')}</h3>
				<p class="mt-1 max-w-sm text-xs text-text-tertiary">{$translate('player.suggestions.noTrackHint')}</p>
			</div>
		{:else if !trackId}
			<div class="flex flex-col items-center justify-center py-12 text-center">
				<div
					class="mb-2.5 flex h-12 w-12 items-center justify-center rounded-2xl border border-stroke-subtle bg-surface-2 text-amber-600 shadow-inner dark:text-amber-300"
				>
					<Icon name="hard-drive" class="h-6 w-6" />
				</div>
				<h3 class="text-sm font-semibold text-text-primary">
					{$translate('player.suggestions.notInLibraryTitle')}
				</h3>
				<p class="mt-1 max-w-sm text-xs text-text-tertiary">
					{$translate('player.suggestions.notInLibraryHint', {
						values: { title: trackTitle || $translate('player.trackFallback') },
					})}
				</p>
				{#if onImportToLibrary}
					<Button variant="primary" size="sm" class="mt-3" onclick={onImportToLibrary} disabled={importing}>
						<Icon
							name={importing ? 'loader' : 'plus'}
							class="mr-1.5 h-3.5 w-3.5 {importing ? 'animate-spin motion-reduce:animate-none' : ''}"
						/>
						{importing ? $translate('player.hero.adding') : $translate('player.hero.addToLibrary')}
					</Button>
				{/if}
			</div>
		{:else if $suggestionsLoading}
			<div class="flex h-32 items-center justify-center gap-2 text-text-tertiary">
				<Spinner icon="loader" class="h-4 w-4" color="current" />
				<span class="text-xs">{$translate('player.suggestions.loading')}</span>
			</div>
		{:else if $suggestionsError}
			<div class="flex flex-col items-center justify-center py-12 text-center">
				<div
					class="mb-2.5 flex h-12 w-12 items-center justify-center rounded-2xl border border-stroke-subtle bg-surface-2 text-red-500 shadow-inner"
				>
					<Icon name="alert-triangle" class="h-6 w-6" />
				</div>
				<h3 class="text-sm font-semibold text-text-primary">{$translate('player.suggestions.errorTitle')}</h3>
				<p class="mt-1 max-w-sm text-xs text-text-tertiary">{$suggestionsError}</p>
				<Button variant="secondary" size="sm" class="mt-3" onclick={handleRetry}>
					{$translate('player.suggestions.retry')}
				</Button>
			</div>
		{:else if $nextTrackSuggestions.length === 0}
			<div class="flex flex-col items-center justify-center py-12 text-center">
				<div
					class="mb-2.5 flex h-12 w-12 items-center justify-center rounded-2xl border border-stroke-subtle bg-surface-2 text-cyan-600 shadow-inner dark:text-cyan-400/60"
				>
					<Icon name="sparkles" class="h-6 w-6" />
				</div>
				<h3 class="text-sm font-semibold text-text-primary">{$translate('player.suggestions.emptyTitle')}</h3>
				<p class="mt-1 max-w-sm text-xs text-text-tertiary">{$translate('player.suggestions.emptyHint')}</p>
			</div>
		{:else}
			<div
				class="grid grid-cols-[28px_minmax(0,1fr)_28px] items-center gap-3 border-b border-stroke-subtle px-3 pb-1 text-[10px] font-semibold tracking-wider text-text-tertiary uppercase"
			>
				<span>#</span>
				<span>{$translate('player.columns.titleArtist')}</span>
				<span class="text-right">{$translate('player.columns.actions')}</span>
			</div>
			<div class="flex flex-col divide-y divide-stroke-subtle/50 py-1">
				{#each $nextTrackSuggestions as suggestion, index (suggestion.track.id)}
					{@const track = suggestion.track}
					{@const artUrl = track.artwork_path ? getArtworkUrl(track.artwork_path, $appDataDir) : null}
					{@const playLabel = $translate('player.suggestions.play', {
						values: { title: track.title || $translate('player.trackFallback') },
					})}
					<div class="group flex items-center gap-2 rounded-lg px-3 py-1.5 transition-colors hover:bg-surface-1">
						<button
							type="button"
							class="group/row grid flex-1 cursor-pointer grid-cols-[20px_32px_minmax(120px,2fr)_150px_130px] items-center gap-3 text-left text-xs"
							onclick={() => handlePlay(track)}
							title={playLabel}
							aria-label={playLabel}
						>
							<span class="flex items-center font-mono text-text-tertiary">
								<span class="group-hover/row:hidden">{index + 1}</span>
								<Icon
									name="play"
									class="hidden h-3.5 w-3.5 text-cyan-600 group-hover/row:block dark:text-cyan-400"
									fill
								/>
							</span>

							<span
								class="h-8 w-8 flex-shrink-0 overflow-hidden rounded-lg border border-stroke-subtle bg-surface-2/60"
							>
								{#if artUrl}
									<img src={artUrl} alt="" class="h-full w-full object-cover" />
								{:else}
									<span class="flex h-full w-full items-center justify-center text-text-tertiary opacity-40">
										<Icon name="music-note" class="h-3.5 w-3.5" />
									</span>
								{/if}
							</span>

							<span class="min-w-0">
								<span
									class="block truncate font-semibold text-text-primary transition-colors group-hover/row:text-cyan-600 dark:group-hover/row:text-cyan-400"
								>
									{track.title || track.file_path.split('/').pop() || $translate('player.unknownTrack')}
								</span>
								<span class="block truncate text-[11px] text-text-secondary">
									{track.artist || $translate('common.unknownArtist')}
								</span>
							</span>

							<span class="flex min-w-0 flex-col gap-0.5">
								<span class="flex items-center gap-1">
									{#if isClash(suggestion.harmonic)}
										<Icon name="alert-triangle" class="h-3 w-3 flex-shrink-0 text-warning" />
									{/if}
									{#if track.key}
										<KeyBadge value={track.key} label={formatKey(track.key, 'camelot')} variant="pill-xs" />
									{/if}
									<span class="truncate text-[11px] text-text-secondary">{relationLabel(suggestion.harmonic)}</span>
								</span>
								{#if track.bpm}
									<span class="font-mono text-[10px] text-text-tertiary tabular-nums">
										{formatBpm(track.bpm)} BPM
										{#if suggestion.bpm_delta_percent !== null}
											·
											{$translate('stats.timeline.tempo', {
												values: { delta: formatTempoDelta(suggestion.bpm_delta_percent, $language) },
											})}
										{/if}
									</span>
								{/if}
							</span>

							<span>
								{#if suggestion.source === 'history'}
									<Tooltip text={historyTooltip(suggestion)} position="top">
										<span
											class="inline-flex items-center gap-1 rounded-full border border-cyan-500/20 bg-cyan-500/10 px-2 py-0.5 text-[10px] font-bold text-cyan-600 dark:text-cyan-300"
										>
											<Icon name="clock" class="h-2.5 w-2.5" />
											{$translate('player.suggestions.badgeHistory', {
												values: { count: suggestion.times_played_after },
											})}
										</span>
									</Tooltip>
								{:else}
									<Tooltip text={$translate('player.suggestions.compatibleTooltip')} position="top">
										<span
											class="inline-flex items-center gap-1 rounded-full border border-stroke-subtle bg-surface-2 px-2 py-0.5 text-[10px] font-medium text-text-tertiary"
										>
											<Icon name="sparkles" class="h-2.5 w-2.5" />
											{$translate('player.suggestions.badgeCompatible')}
										</span>
									</Tooltip>
								{/if}
							</span>
						</button>

						<Tooltip text={$translate('player.recent.reveal')} position="top">
							<button
								type="button"
								class="cursor-pointer rounded-full p-1 text-text-tertiary opacity-0 transition-opacity group-hover:opacity-100 hover:bg-surface-2 hover:text-text-primary has-focus-visible:opacity-100"
								aria-label={$translate('player.recent.reveal')}
								onclick={() => handleReveal(track.file_path)}
							>
								<Icon name="folder-open" class="h-3 w-3" />
							</button>
						</Tooltip>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>
