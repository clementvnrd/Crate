<script lang="ts">
	import type { Track, UpgradeMatch } from '$shared/types'
	import {
		upgraderStore,
		upgraderMatches,
		upgraderMatchCount,
		isUpgraderLoading,
		isUpgrading,
		upgradeProgress,
		selectedUpgradeCount,
	} from '$shared/stores/upgrader'
	import {
		playerStore,
		currentTrack,
		isPlaying,
		playbackPosition,
		playbackSource,
		beatportTrack as currentBeatportTrack,
	} from '$shared/stores/player'
	import { beatportStore } from '$shared/stores/beatport'
	import { pageActions } from '$lib/stores'
	import { translate } from '$shared/i18n'
	import { libraryStore } from '$lib/stores/library'
	import { formatBitrate, formatDurationCompact } from '$shared/utils/format'
	import { Button, Icon, KeyBadge, Text } from '$lib/components/common'
	import Modal from '$lib/components/common/Modal.svelte'

	type Props = {
		open: boolean
		onClose: () => void
	}

	let { open, onClose }: Props = $props()

	const isBeatportConnected = $derived(!!$beatportStore.auth.token && $beatportStore.auth.is_authenticated)
	const isAuthRequired = $derived(
		!isBeatportConnected ||
			($upgraderStore.error !== null &&
				($upgraderStore.error.toLowerCase().includes('authentification') ||
					$upgraderStore.error.toLowerCase().includes('beatport') ||
					$upgraderStore.error.includes('401') ||
					$upgraderStore.error.includes('403')))
	)

	function handleConnectBeatport() {
		onClose()
		$pageActions?.handleViewChange('beatport')
		if (!$beatportStore.auth.is_authenticated) {
			beatportStore.openLoginModal()
		}
	}

	// Scan upgrades when modal is opened
	$effect(() => {
		if (open) {
			upgraderStore.load()
		}
	})

	function convertToLocalTrack(match: UpgradeMatch): Track {
		return {
			id: match.track_id,
			file_path: match.file_path,
			file_hash: null,
			title: match.title,
			artist: match.artist,
			album: match.album,
			year: null,
			genre: null,
			label: null,
			catalog_number: null,
			duration_ms: match.current_duration_ms,
			bpm: match.current_bpm,
			key: match.current_key,
			energy: match.current_energy,
			bitrate: match.current_bitrate,
			sample_rate: match.current_sample_rate,
			format: match.current_format,
			analysis_source: null,
			waveform_data: null,
			rating: 0,
			play_count: 0,
			date_added: '',
			date_modified: '',
			last_played: null,
			rekordbox_id: null,
			artwork_path: match.current_artwork_path,
			artwork_source: null,
			color: null,
			library_root_id: null,
			relative_path: null,
			tags: [],
		}
	}

	function handleTogglePlayLocal(match: UpgradeMatch) {
		const isThisLocalPlaying = $playbackSource === 'library' && $currentTrack?.id === match.track_id && $isPlaying

		if (isThisLocalPlaying) {
			playerStore.pause()
			return
		}
		if ($playbackSource === 'library' && $currentTrack?.id === match.track_id && !$isPlaying) {
			playerStore.resume()
			return
		}

		const trackObj = convertToLocalTrack(match)
		playerStore.play(trackObj)
	}

	function handleTogglePlayBeatport(match: UpgradeMatch) {
		const isThisBeatportPlaying =
			$playbackSource === 'beatport' &&
			$currentBeatportTrack?.id?.toString() === match.beatport_track.id.toString() &&
			$isPlaying

		if (isThisBeatportPlaying) {
			playerStore.pause()
			return
		}
		if (
			$playbackSource === 'beatport' &&
			$currentBeatportTrack?.id?.toString() === match.beatport_track.id.toString() &&
			!$isPlaying
		) {
			playerStore.resume()
			return
		}

		playerStore.playBeatport(match.beatport_track)
	}

	async function handleWaveformClickLocal(e: MouseEvent, match: UpgradeMatch) {
		e.stopPropagation()
		const target = e.currentTarget as HTMLElement
		const rect = target.getBoundingClientRect()
		const clickX = e.clientX - rect.left
		const ratio = Math.max(0, Math.min(1, clickX / rect.width))
		const duration = match.current_duration_ms || 180000
		const targetMs = Math.round(ratio * duration)

		if ($playbackSource === 'library' && $currentTrack?.id === match.track_id) {
			if (!$isPlaying) {
				await playerStore.resume()
			}
			await playerStore.seek(targetMs)
		} else {
			const trackObj = convertToLocalTrack(match)
			await playerStore.play(trackObj)
			await playerStore.seek(targetMs)
		}
	}

	async function handleExecuteUpgrade() {
		if ($selectedUpgradeCount === 0 || $isUpgrading || isAuthRequired) return
		await upgraderStore.executeSelected(async () => {
			await libraryStore.reloadWithCurrentFilter()
		})
	}

	// eslint-disable-next-line svelte/prefer-svelte-reactivity -- memo cache, not UI state
	const waveformCache = new Map<string, number[]>()

	function getBarsForTrack(id: string): number[] {
		const cached = waveformCache.get(id)
		if (cached) return cached

		let hash = 0
		const seed = id || 'crate-waveform-seed'
		for (let i = 0; i < seed.length; i++) {
			hash = (hash << 5) - hash + seed.charCodeAt(i)
			hash |= 0
		}

		const count = 48
		const bars: number[] = []
		for (let i = 0; i < count; i++) {
			const x = (i / count) * Math.PI
			const envelope = Math.sin(x) * 0.45 + 0.5
			const pseudoRand = Math.abs(Math.sin((hash + i * 19.87) * 43758.5453))
			const height = Math.max(0.18, Math.min(0.96, envelope * (0.3 + pseudoRand * 0.7)))
			bars.push(height)
		}

		waveformCache.set(id, bars)
		return bars
	}
</script>

<Modal {open} {onClose} size="4xl" flush>
	<div class="flex h-[85vh] min-h-0 flex-col overflow-hidden bg-surface-1">
		<!-- Header -->
		<div class="flex flex-shrink-0 items-center justify-between border-b border-stroke bg-surface-2/60 px-6 py-4">
			<div class="flex items-center gap-3">
				<!-- Sparkles Upgrader Icon -->
				<div
					class="flex h-9 w-9 items-center justify-center rounded-xl border border-emerald-500/30 bg-gradient-to-br from-emerald-500/20 via-teal-500/15 to-emerald-600/10 text-emerald-400 shadow-sm"
				>
					<Icon name="sparkles" class="h-5 w-5 stroke-[2.2]" />
				</div>
				<div>
					<div class="flex items-center gap-2.5">
						<Text variant="header-1" weight="bold">Beatport Quality Upgrader</Text>
						{#if $upgraderMatchCount > 0}
							<span
								class="rounded-full border border-emerald-500/30 bg-emerald-500/15 px-2.5 py-0.5 text-xs font-semibold text-emerald-400"
							>
								{$translate('upgrader.header.upgradableCount', { values: { count: $upgraderMatchCount } })}
							</span>
						{/if}
					</div>
				</div>
			</div>

			<!-- Header Actions -->
			<div class="flex items-center gap-2.5">
				{#if $upgraderMatchCount > 0}
					<button
						type="button"
						class="cursor-pointer rounded-lg border border-stroke/50 bg-surface-2 px-3 py-1.5 text-xs text-text-secondary transition-colors hover:bg-surface-3 hover:text-text-primary"
						onclick={() => upgraderStore.selectAll()}
					>
						{$translate('upgrader.actions.selectAll')}
					</button>
					<button
						type="button"
						class="cursor-pointer rounded-lg border border-stroke/50 bg-surface-2 px-3 py-1.5 text-xs text-text-secondary transition-colors hover:bg-surface-3 hover:text-text-primary"
						onclick={() => upgraderStore.deselectAll()}
					>
						{$translate('upgrader.actions.deselectAll')}
					</button>
				{/if}

				<button
					type="button"
					class="inline-flex cursor-pointer items-center gap-1.5 rounded-lg border border-stroke/50 bg-surface-2 px-3 py-1.5 text-xs text-text-secondary transition-colors hover:bg-surface-3 hover:text-text-primary disabled:opacity-50"
					onclick={() => upgraderStore.load()}
					disabled={$isUpgraderLoading}
					title={$translate('upgrader.actions.refreshTitle')}
				>
					<Icon
						name="refresh-cw"
						class="h-3.5 w-3.5 {$isUpgraderLoading ? 'animate-spin text-emerald-400 motion-reduce:animate-none' : ''}"
					/>
					<span>{$translate('upgrader.actions.refresh')}</span>
				</button>

				<button
					type="button"
					class="inline-flex cursor-pointer items-center gap-1 rounded-lg border border-stroke/50 bg-surface-2 px-3 py-1.5 text-xs text-text-secondary transition-colors hover:bg-surface-3 hover:text-text-primary"
					onclick={onClose}
					title={$translate('common.close')}
				>
					<Icon name="x" class="h-3.5 w-3.5" />
					<span>{$translate('common.close')}</span>
				</button>
			</div>
		</div>

		<!-- Body Content -->
		<div class="min-h-0 flex-1 space-y-4 overflow-y-auto p-6">
			{#if isAuthRequired && !$isUpgraderLoading}
				<div
					class="mx-auto my-6 flex max-w-lg flex-col items-center justify-center gap-5 rounded-2xl border border-emerald-500/30 bg-gradient-to-b from-emerald-950/30 via-surface-2 to-surface-1 p-8 text-center shadow-lg"
				>
					<div
						class="flex h-16 w-16 items-center justify-center rounded-2xl border border-emerald-500/30 bg-emerald-500/20 text-emerald-400 shadow-md"
					>
						<Icon name="beatport" class="h-8 w-8 fill-current" />
					</div>
					<div class="space-y-1.5">
						<Text variant="header-2" weight="bold" class="text-text-primary">{$translate('upgrader.auth.title')}</Text>
						<Text variant="body-2" class="text-text-secondary">
							{$translate('upgrader.auth.body')}
						</Text>
					</div>
					<Button
						variant="primary"
						class="flex items-center gap-2 bg-emerald-600 px-5 py-2.5 text-xs font-semibold text-white shadow-lg shadow-emerald-950/50 hover:bg-emerald-500"
						onclick={handleConnectBeatport}
					>
						<Icon name="beatport" class="h-4 w-4 fill-current" />
						<span>{$translate('upgrader.auth.connect')}</span>
					</Button>
				</div>
			{:else if $isUpgraderLoading && $upgraderMatches.length === 0}
				<div class="flex h-80 flex-col items-center justify-center gap-4 px-4 text-center">
					<div class="relative flex h-16 w-16 items-center justify-center">
						<div
							class="absolute inset-0 animate-ping rounded-full border-3 border-emerald-500/20 motion-reduce:animate-none"
						></div>
						<div
							class="h-14 w-14 animate-spin rounded-full border-3 border-emerald-500 border-t-transparent shadow-lg shadow-emerald-500/20 motion-reduce:animate-none"
						></div>
						<Icon name="sparkles" class="absolute h-6 w-6 animate-pulse text-emerald-400 motion-reduce:animate-none" />
					</div>
					<div class="space-y-1">
						<Text variant="header-2" weight="bold" class="text-text-primary"
							>{$translate('upgrader.scanning.title')}</Text
						>
						<Text variant="body-2" class="mx-auto max-w-md text-text-secondary">
							{$translate('upgrader.scanning.body')}
						</Text>
					</div>
				</div>
			{:else if $upgraderMatches.length === 0}
				<div class="flex h-80 flex-col items-center justify-center gap-4 px-4 text-center">
					<div
						class="flex h-16 w-16 items-center justify-center rounded-2xl border border-emerald-500/30 bg-emerald-500/20 text-emerald-400 shadow-lg"
					>
						<Icon name="check" class="h-8 w-8 stroke-[3]" />
					</div>
					<div>
						<Text variant="header-2" weight="bold" class="mb-1 text-text-primary">
							{$translate('upgrader.empty.title')}
						</Text>
						<Text variant="body-1" class="mx-auto max-w-md text-text-secondary">
							{$translate('upgrader.empty.body')}
						</Text>
					</div>
				</div>
			{:else}
				{#if $isUpgraderLoading}
					<div
						class="mb-2 flex animate-pulse items-center justify-center gap-2.5 rounded-lg border border-emerald-500/30 bg-emerald-950/20 px-3.5 py-2 text-xs font-medium text-emerald-400 motion-reduce:animate-none"
					>
						<Icon name="refresh-cw" class="h-3.5 w-3.5 animate-spin motion-reduce:animate-none" />
						<span>{$translate('upgrader.refreshing')}</span>
					</div>
				{/if}
				{#each $upgraderMatches as match (match.track_id)}
					{@const isSelected = $upgraderStore.selectedMatchTrackIds.has(match.track_id)}
					{@const isLocalCurrent = $playbackSource === 'library' && $currentTrack?.id === match.track_id}
					{@const isLocalPlaying = isLocalCurrent && $isPlaying}
					{@const isBpCurrent =
						$playbackSource === 'beatport' &&
						$currentBeatportTrack?.id?.toString() === match.beatport_track.id.toString()}
					{@const isBpPlaying = isBpCurrent && $isPlaying}
					{@const localProgress = isLocalCurrent
						? Math.min(100, Math.max(0, ($playbackPosition / (match.current_duration_ms || 1)) * 100))
						: 0}

					<div
						class="rounded-xl border bg-surface-2/70 p-4 shadow-sm transition-all hover:border-stroke-strong
						{isSelected ? 'border-emerald-500/40 bg-emerald-950/10' : 'border-stroke'}"
					>
						<!-- Match Header: Track Title + Confidence Score + Ignore Button -->
						<div class="mb-3 flex items-center justify-between border-b border-stroke/40 pb-3">
							<div class="flex min-w-0 items-center gap-3">
								<Text variant="body-2" weight="bold" class="truncate text-text-primary">
									{match.artist} — {match.title}
								</Text>

								<!-- Confidence Score Badge -->
								<span
									class="inline-flex shrink-0 items-center gap-1 rounded-md border px-2 py-0.5 font-mono text-[11px] font-bold
									{match.confidence_score >= 90
										? 'border-emerald-500/30 bg-emerald-500/15 text-emerald-400'
										: 'border-teal-500/30 bg-teal-500/15 text-teal-300'}"
									title={$translate('upgrader.card.scoreTitle', {
										values: {
											score: match.confidence_score,
											titleScore: match.score_breakdown.title_score,
											artistScore: match.score_breakdown.artist_score,
											durationScore: match.score_breakdown.duration_score,
											bpmScore: match.score_breakdown.bpm_score,
											keyScore: match.score_breakdown.key_score,
										},
									})}
								>
									<Icon name="sparkles" class="h-3 w-3" />
									{$translate('upgrader.card.match', { values: { score: match.confidence_score } })}
								</span>
							</div>

							<button
								type="button"
								class="shrink-0 cursor-pointer rounded-lg border border-stroke/50 bg-surface-2 px-3 py-1.5 text-xs text-text-secondary transition-colors hover:bg-surface-3 hover:text-text-primary"
								onclick={() => upgraderStore.ignoreMatch(match)}
								title={$translate('upgrader.card.ignoreTitle')}
							>
								{$translate('upgrader.card.ignore')}
							</button>
						</div>

						<!-- Side-by-Side Comparison Grid (3 columns) -->
						<div class="grid grid-cols-1 items-center gap-3.5 lg:grid-cols-12">
							<!-- Left Column: Current MP3 -->
							<div class="space-y-2.5 rounded-xl border border-stroke/60 bg-surface-1/80 p-3.5 lg:col-span-5">
								<div class="flex items-center justify-between">
									<span
										class="flex items-center gap-1.5 text-[11px] font-bold tracking-wider text-amber-400/90 uppercase"
									>
										<span class="inline-block h-2 w-2 rounded-full bg-amber-400"></span>
										{$translate('upgrader.card.current')}
									</span>
									<span class="font-mono text-xs text-text-secondary tabular-nums">
										{formatDurationCompact(match.current_duration_ms)}
									</span>
								</div>

								<div class="flex items-start gap-3">
									<!-- Local Play Button -->
									<button
										type="button"
										class="group relative flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border transition-all hover:cursor-pointer
										{isLocalPlaying
											? 'border-[#00E5FF] bg-[#00E5FF] text-black shadow-md shadow-[#00E5FF]/40'
											: 'border-stroke bg-surface-2 text-text-primary hover:border-[#00E5FF]/60 hover:bg-[#00E5FF]/15 hover:text-[#00E5FF]'}"
										onclick={() => handleTogglePlayLocal(match)}
										title={isLocalPlaying ? $translate('upgrader.card.pause') : $translate('upgrader.card.playLocal')}
									>
										{#if isLocalPlaying}
											<Icon name="pause" class="h-4 w-4" fill />
										{:else}
											<Icon name="play" class="ml-0.5 h-4 w-4 transition-transform group-hover:scale-110" fill />
										{/if}
									</button>

									<div class="min-w-0 flex-1">
										<Text variant="body-2" weight="bold" class="truncate text-text-primary">
											{match.title}
										</Text>
										<Text variant="caption" class="truncate text-text-secondary">
											{match.artist}
											{match.album ? `• ${match.album}` : ''}
										</Text>
									</div>
								</div>

								<!-- Interactive Local Waveform -->
								<div
									role="button"
									tabindex="0"
									class="group/wave relative flex h-8 w-full cursor-pointer items-center justify-between gap-[2px] rounded-lg border border-stroke/40 bg-surface-0/70 px-2 py-1 transition-all select-none hover:border-[#00E5FF]/40 hover:bg-surface-0"
									onclick={(e) => handleWaveformClickLocal(e, match)}
									onkeydown={(e) => {
										if (e.key === 'Enter' || e.key === ' ') {
											handleTogglePlayLocal(match)
										}
									}}
									title={$translate('upgrader.card.seekLocal')}
								>
									{#each getBarsForTrack(match.track_id) as barHeight, barIdx (barIdx)}
										{@const barPercent = (barIdx / 48) * 100}
										{@const isPast = isLocalCurrent && barPercent <= localProgress}
										<div class="flex h-full flex-1 items-center justify-center">
											<div
												class="w-full rounded-full transition-all duration-75 {isPast
													? 'bg-[#00E5FF] shadow-[0_0_6px_rgba(0,229,255,0.7)]'
													: 'bg-[#38BDF8]/35 group-hover/wave:bg-[#38BDF8]/55'}"
												style="height: {Math.max(18, Math.round(barHeight * 100))}%;"
											></div>
										</div>
									{/each}

									{#if isLocalCurrent}
										<div
											class="pointer-events-none absolute inset-y-0 w-[2px] bg-[#00E5FF] shadow-[0_0_8px_#00E5FF] transition-[left] duration-100"
											style="left: {localProgress}%;"
										></div>
									{/if}
								</div>

								<!-- Badges row -->
								<div class="flex flex-wrap items-center gap-1.5">
									<span
										class="inline-flex h-[20px] items-center rounded border border-amber-500/30 bg-amber-500/15 px-1.5 font-mono text-[10px] font-bold tracking-wider text-amber-300 uppercase"
									>
										{match.current_format.toUpperCase()}
										{match.current_bitrate ? `${match.current_bitrate} kbps` : ''}
									</span>

									{#if match.current_key}
										<KeyBadge value={match.current_key} variant="chip-plain" />
									{/if}

									{#if match.current_bpm}
										<span
											class="inline-flex h-[20px] items-center rounded border border-stroke/50 bg-surface-2 px-1.5 font-mono text-[10px] text-text-secondary"
										>
											{Math.round(match.current_bpm)} bpm
										</span>
									{/if}

									{#if match.current_energy}
										<span
											class="inline-flex h-[20px] items-center rounded border border-amber-500/30 bg-amber-500/10 px-1.5 font-mono text-[10px] font-bold text-amber-400"
										>
											⚡ {match.current_energy}
										</span>
									{/if}
								</div>

								<!-- File Path -->
								<div
									class="flex items-center gap-1.5 truncate font-mono text-[11px] text-text-tertiary"
									title={match.file_path}
								>
									<Icon name="folder" class="h-3 w-3 shrink-0 text-text-disabled" />
									<span class="truncate">{match.file_path}</span>
								</div>
							</div>

							<!-- Center Column: Transformation Arrow & Quality Benefit -->
							<div class="flex flex-col items-center justify-center gap-2 py-2 text-center lg:col-span-2">
								<div
									class="flex h-10 w-10 items-center justify-center rounded-full border border-emerald-500/40 bg-gradient-to-r from-amber-500/20 via-emerald-500/20 to-emerald-400/30 text-emerald-400 shadow-md"
								>
									<Icon name="arrow-right" class="h-5 w-5 stroke-[2.5]" />
								</div>
								<div class="space-y-0.5">
									<span class="font-mono text-[11px] font-bold tracking-wide text-emerald-400 uppercase">
										MP3 ➔ FLAC
									</span>
									<p class="text-[10px] font-medium text-text-tertiary">{$translate('upgrader.card.losslessStudio')}</p>
								</div>
							</div>

							<!-- Right Column: Beatport FLAC Lossless Version -->
							<div
								class="space-y-2.5 rounded-xl border border-emerald-500/40 bg-surface-1/90 p-3.5 shadow-sm lg:col-span-5"
							>
								<div class="flex items-center justify-between">
									<span
										class="flex items-center gap-1.5 text-[11px] font-bold tracking-wider text-emerald-400 uppercase"
									>
										<span class="inline-block h-2 w-2 rounded-full bg-emerald-400"></span>
										Beatport (FLAC Lossless)
									</span>
									<span class="font-mono text-xs font-semibold text-emerald-400 tabular-nums">
										{match.beatport_track.duration_formatted || formatDurationCompact(match.beatport_track.duration_ms)}
									</span>
								</div>

								<div class="flex items-start gap-3">
									<!-- Beatport Artwork or Play Button -->
									<div
										class="group relative h-11 w-11 shrink-0 overflow-hidden rounded-lg border border-emerald-500/30 bg-surface-2"
									>
										{#if match.beatport_track.artwork_url}
											<img
												src={match.beatport_track.artwork_url}
												alt={match.beatport_track.title}
												class="h-full w-full object-cover"
											/>
										{/if}
										<button
											type="button"
											class="absolute inset-0 flex items-center justify-center bg-black/60 transition-opacity hover:cursor-pointer
											{isBpPlaying ? 'bg-[#00FF96]/80 text-black opacity-100' : 'text-white opacity-0 group-hover:opacity-100'}"
											onclick={() => handleTogglePlayBeatport(match)}
											title={isBpPlaying ? $translate('upgrader.card.pause') : $translate('upgrader.card.playPreview')}
										>
											{#if isBpPlaying}
												<Icon name="pause" class="h-4 w-4" fill />
											{:else}
												<Icon name="play" class="ml-0.5 h-4 w-4" fill />
											{/if}
										</button>
									</div>

									<div class="min-w-0 flex-1">
										<div class="flex items-center gap-1.5">
											<Text variant="body-2" weight="bold" class="truncate text-text-primary">
												{match.beatport_track.title}
											</Text>
											{#if match.beatport_track.mix_name}
												<span class="truncate text-xs text-text-tertiary">
													({match.beatport_track.mix_name})
												</span>
											{/if}
										</div>
										<Text variant="caption" class="truncate text-text-secondary">
											{match.beatport_track.artists.map((a) => a.name).join(', ')}
										</Text>
									</div>
								</div>

								<!-- Badges row -->
								<div class="flex flex-wrap items-center gap-1.5">
									<span
										class="inline-flex h-[20px] items-center rounded border border-emerald-500/30 bg-emerald-500/15 px-1.5 font-mono text-[10px] font-bold tracking-wider text-emerald-400 uppercase"
									>
										FLAC LOSSLESS (24-bit / 44.1kHz)
									</span>

									{#if match.beatport_track.key}
										<KeyBadge value={match.beatport_track.key} variant="chip-plain" />
									{/if}

									{#if match.beatport_track.bpm}
										<span
											class="inline-flex h-[20px] items-center rounded border border-emerald-500/30 bg-emerald-500/10 px-1.5 font-mono text-[10px] text-emerald-400"
										>
											{Math.round(match.beatport_track.bpm)} bpm
										</span>
									{/if}

									{#if match.beatport_track.genre}
										<span
											class="inline-flex h-[20px] items-center rounded border border-stroke/40 bg-surface-2/70 px-1.5 text-[10px] text-text-tertiary"
										>
											{match.beatport_track.genre}
										</span>
									{/if}
								</div>

								<!-- Beatport Release Info / URL -->
								<div class="flex items-center gap-1.5 truncate text-[11px] text-text-tertiary">
									<Icon name="beatport" class="h-3 w-3 shrink-0 text-emerald-400" />
									<span class="truncate">
										{match.beatport_track.release_name || $translate('upgrader.card.officialCatalogue')}
										{match.beatport_track.release_date ? `(${match.beatport_track.release_date.slice(0, 4)})` : ''}
									</span>
								</div>
							</div>
						</div>

						<!-- Card Bottom: Checkbox -->
						<div class="mt-3 flex items-center justify-between border-t border-stroke/40 pt-3">
							<label
								class="flex items-center gap-2 text-xs font-medium select-none hover:cursor-pointer {isSelected
									? 'text-emerald-400'
									: 'text-text-secondary'}"
							>
								<input
									type="checkbox"
									checked={isSelected}
									onchange={() => upgraderStore.toggleMatchSelection(match.track_id)}
									class="h-4 w-4 rounded border-stroke bg-surface-2 text-emerald-500 hover:cursor-pointer focus:ring-emerald-500"
								/>
								<span>{$translate('upgrader.card.replace')}</span>
							</label>

							<span class="font-mono text-[11px] text-text-tertiary">
								{$translate('upgrader.card.replaceNote')}
							</span>
						</div>
					</div>
				{/each}
			{/if}
		</div>

		<!-- Footer -->
		<div
			class="sticky bottom-0 z-10 flex flex-shrink-0 items-center justify-between border-t border-stroke bg-surface-2/95 px-6 py-3.5"
		>
			<div class="text-xs">
				<span class="text-text-secondary">{$translate('upgrader.footer.selected')}</span>
				<span class="ml-1 font-mono font-bold text-emerald-400">{$selectedUpgradeCount}</span>
			</div>

			<div class="flex items-center gap-3">
				<button
					type="button"
					class="cursor-pointer rounded-lg border border-stroke/50 bg-surface-2 px-3 py-1.5 text-xs text-text-secondary transition-colors hover:bg-surface-3 hover:text-text-primary"
					onclick={onClose}
				>
					{$translate('common.close')}
				</button>

				<Button
					variant="primary"
					onclick={handleExecuteUpgrade}
					disabled={$selectedUpgradeCount === 0 || $isUpgrading || isAuthRequired}
					class="border-emerald-500 bg-emerald-600 py-1.5 text-xs font-semibold shadow-md shadow-emerald-950/40 hover:bg-emerald-500"
				>
					{#if $isUpgrading}
						<Icon name="refresh-cw" class="mr-2 h-3.5 w-3.5 animate-spin motion-reduce:animate-none" />
						{#if $upgradeProgress}
							{$translate('upgrader.footer.progress', {
								values: {
									current: $upgradeProgress.current,
									total: $upgradeProgress.total,
									title: $upgradeProgress.title,
								},
							})}
						{:else}
							{$translate('upgrader.footer.upgrading')}
						{/if}
					{:else}
						<Icon name="sparkles" class="mr-2 h-3.5 w-3.5" />
						{$translate('upgrader.footer.replace', { values: { count: $selectedUpgradeCount } })}
					{/if}
				</Button>
			</div>
		</div>
	</div>
</Modal>
