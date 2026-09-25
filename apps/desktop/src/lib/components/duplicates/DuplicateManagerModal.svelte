<script lang="ts">
	import type { Track, DuplicateGroup, DuplicateTrackInfo } from '$shared/types'
	import {
		duplicateStore,
		duplicateGroups,
		duplicateGroupCount,
		duplicateTrackCount,
		isDuplicateLoading,
		selectedDuplicateCount,
	} from '$shared/stores/duplicate'
	import { playerStore, currentTrack, isPlaying, playbackPosition } from '$shared/stores/player'
	import { libraryStore } from '$lib/stores/library'
	import { formatBitrate, formatDurationCompact } from '$shared/utils/format'
	import { getCamelotColor, formatCamelotKey } from '$shared/utils/camelot'
	import { Button, Icon, Text } from '$lib/components/common'
	import Modal from '$lib/components/common/Modal.svelte'

	type Props = {
		open: boolean
		onClose: () => void
	}

	let { open, onClose }: Props = $props()

	let isDeleting = $state(false)

	// Scan duplicates when modal is opened
	$effect(() => {
		if (open) {
			duplicateStore.load()
		}
	})

	function convertToTrack(trackInfo: DuplicateTrackInfo): Track {
		return {
			id: trackInfo.id,
			file_path: trackInfo.file_path,
			file_hash: trackInfo.file_hash,
			title: trackInfo.title,
			artist: trackInfo.artist,
			album: trackInfo.album,
			year: trackInfo.year,
			genre: trackInfo.genre,
			label: trackInfo.label,
			catalog_number: null,
			duration_ms: trackInfo.duration_ms,
			bpm: trackInfo.bpm,
			key: trackInfo.key,
			energy: trackInfo.energy,
			bitrate: trackInfo.bitrate,
			sample_rate: trackInfo.sample_rate,
			format: trackInfo.format,
			analysis_source: null,
			waveform_data: null,
			rating: trackInfo.rating,
			play_count: trackInfo.play_count,
			date_added: trackInfo.date_added,
			date_modified: trackInfo.date_added,
			last_played: null,
			rekordbox_id: null,
			artwork_path: trackInfo.artwork_path,
			artwork_source: null,
			color: null,
			library_root_id: null,
			relative_path: null,
			tags: [],
		}
	}

	function handleTogglePlay(trackInfo: DuplicateTrackInfo) {
		if ($currentTrack?.id === trackInfo.id && $isPlaying) {
			playerStore.pause()
			return
		}
		if ($currentTrack?.id === trackInfo.id && !$isPlaying) {
			playerStore.resume()
			return
		}

		const trackObj = convertToTrack(trackInfo)
		playerStore.play(trackObj)
	}

	async function handleWaveformClick(e: MouseEvent, trackInfo: DuplicateTrackInfo) {
		e.stopPropagation()
		const target = e.currentTarget as HTMLElement
		const rect = target.getBoundingClientRect()
		const clickX = e.clientX - rect.left
		const ratio = Math.max(0, Math.min(1, clickX / rect.width))
		const duration = trackInfo.duration_ms || 180000
		const targetMs = Math.round(ratio * duration)

		if ($currentTrack?.id === trackInfo.id) {
			if (!$isPlaying) {
				await playerStore.resume()
			}
			await playerStore.seek(targetMs)
		} else {
			const trackObj = convertToTrack(trackInfo)
			await playerStore.play(trackObj)
			await playerStore.seek(targetMs)
		}
	}

	async function handleDeleteSelected() {
		if ($selectedDuplicateCount === 0 || isDeleting) return
		isDeleting = true
		try {
			await duplicateStore.deleteSelected(async () => {
				await libraryStore.loadTracks()
			})
		} finally {
			isDeleting = false
		}
	}

	function isTrackLossless(format: string): boolean {
		const f = (format || '').toLowerCase()
		return ['flac', 'wav', 'wave', 'aiff', 'aif', 'alac'].includes(f)
	}

	const waveformCache = new Map<string, number[]>()

	function getBarsForTrack(id: string, fileHash: string | null): number[] {
		const key = id + (fileHash || '')
		const cached = waveformCache.get(key)
		if (cached) return cached

		let hash = 0
		const seed = key || 'crate-waveform-seed'
		for (let i = 0; i < seed.length; i++) {
			hash = (hash << 5) - hash + seed.charCodeAt(i)
			hash |= 0
		}

		const count = 64
		const bars: number[] = []
		for (let i = 0; i < count; i++) {
			const x = (i / count) * Math.PI
			const envelope = Math.sin(x) * 0.45 + 0.5
			const pseudoRand = Math.abs(Math.sin((hash + i * 19.87) * 43758.5453))
			const height = Math.max(0.18, Math.min(0.96, envelope * (0.3 + pseudoRand * 0.7)))
			bars.push(height)
		}

		waveformCache.set(key, bars)
		return bars
	}
</script>

<Modal {open} {onClose} size="4xl" flush>
	<div class="flex max-h-[88vh] flex-col bg-surface-1">
		<!-- Header -->
		<div class="flex items-center justify-between border-b border-stroke px-6 py-4 bg-surface-2/60">
			<div class="flex items-center gap-3">
				<!-- Modern sleek icon -->
				<div class="flex h-9 w-9 items-center justify-center rounded-xl bg-gradient-to-br from-brand-primary/20 via-brand-primary/10 to-sky-500/10 border border-brand-primary/30 text-brand-primary shadow-sm">
					<svg class="h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
						<circle cx="9" cy="12" r="6" />
						<circle cx="15" cy="12" r="6" stroke-dasharray="1.5 2.5" class="opacity-70" />
						<circle cx="9" cy="12" r="2" />
						<circle cx="15" cy="12" r="2" />
					</svg>
				</div>
				<div>
					<div class="flex items-center gap-2.5">
						<Text variant="header-1" weight="bold">Duplicate Killer</Text>
						{#if $duplicateGroupCount > 0}
							<span class="rounded-full bg-brand-primary/15 px-2.5 py-0.5 text-xs font-semibold text-brand-primary border border-brand-primary/30">
								{$duplicateGroupCount} groupe{$duplicateGroupCount > 1 ? 's' : ''} ({$duplicateTrackCount} doublon{$duplicateTrackCount > 1 ? 's' : ''})
							</span>
						{/if}
					</div>
				</div>
			</div>

			<!-- Header Actions (Pill style) -->
			<div class="flex items-center gap-2.5">
				{#if $duplicateGroupCount > 0}
					<button
						type="button"
						class="bg-surface-2 hover:bg-surface-3 border border-stroke/50 text-text-secondary hover:text-text-primary text-xs px-3 py-1.5 rounded-lg transition-colors cursor-pointer"
						onclick={() => duplicateStore.selectAllDuplicates()}
					>
						Tout cocher
					</button>
					<button
						type="button"
						class="bg-surface-2 hover:bg-surface-3 border border-stroke/50 text-text-secondary hover:text-text-primary text-xs px-3 py-1.5 rounded-lg transition-colors cursor-pointer"
						onclick={() => duplicateStore.deselectAll()}
					>
						Tout décocher
					</button>
				{/if}

				<button
					type="button"
					class="bg-surface-2 hover:bg-surface-3 border border-stroke/50 text-text-secondary hover:text-text-primary text-xs px-3 py-1.5 rounded-lg transition-colors cursor-pointer inline-flex items-center gap-1.5 disabled:opacity-50"
					onclick={() => duplicateStore.load()}
					disabled={$isDuplicateLoading}
					title="Actualiser la liste des doublons"
				>
					<Icon name="refresh-cw" class="h-3.5 w-3.5 {$isDuplicateLoading ? 'animate-spin text-brand-primary' : ''}" />
					<span>Actualiser</span>
				</button>

				<button
					type="button"
					class="bg-surface-2 hover:bg-surface-3 border border-stroke/50 text-text-secondary hover:text-text-primary text-xs px-3 py-1.5 rounded-lg transition-colors cursor-pointer inline-flex items-center gap-1"
					onclick={onClose}
					title="Fermer"
				>
					<Icon name="x" class="h-3.5 w-3.5" />
					<span>Fermer</span>
				</button>
			</div>
		</div>

		<!-- Body Content -->
		<div class="min-h-[380px] flex-1 overflow-y-auto p-6 space-y-5">
			{#if $isDuplicateLoading && $duplicateGroups.length === 0}
				<div class="flex h-72 flex-col items-center justify-center gap-3">
					<div class="h-10 w-10 animate-spin rounded-full border-3 border-brand-primary border-t-transparent"></div>
					<Text variant="body-1" class="text-text-secondary font-medium">Analyse et recherche des doublons...</Text>
				</div>
			{:else if $duplicateGroups.length === 0}
				<div class="flex h-80 flex-col items-center justify-center gap-4 text-center px-4">
					<div class="flex h-16 w-16 items-center justify-center rounded-2xl bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 shadow-lg">
						<Icon name="check" class="h-8 w-8 stroke-[3]" />
					</div>
					<div>
						<Text variant="header-2" weight="bold" class="text-text-primary mb-1">
							Aucun doublon détecté dans votre bibliothèque !
						</Text>
						<Text variant="body-1" class="text-text-secondary max-w-md mx-auto">
							Toutes les pistes sont uniques. Votre collection est propre, optimisée et prête pour le mix.
						</Text>
					</div>
				</div>
			{:else}
				{#each $duplicateGroups as group (group.id)}
					<div class="rounded-xl border border-stroke bg-surface-2/70 p-4 shadow-sm transition-all hover:border-stroke-strong">
						<!-- Minimalist Group Header -->
						<div class="flex items-center justify-between pb-2.5 mb-3 border-b border-stroke/40">
							<div class="flex items-center gap-2.5 min-w-0">
								<Text variant="body-2" weight="bold" class="truncate text-text-primary">
									{group.tracks[0].artist || 'Artiste inconnu'} — {group.tracks[0].title || 'Titre inconnu'}
								</Text>

								{#if group.match_type === 'exact_hash'}
									<span class="inline-flex items-center gap-1 rounded-md bg-emerald-500/15 border border-emerald-500/30 px-2 py-0.5 text-[10px] font-medium text-emerald-400 shrink-0">
										<Icon name="sparkles" class="h-3 w-3" />
										Audio identique (Blake3)
									</span>
								{:else}
									<span class="inline-flex items-center gap-1 rounded-md bg-sky-500/15 border border-sky-500/30 px-2 py-0.5 text-[10px] font-medium text-sky-400 shrink-0">
										<Icon name="clone" class="h-3 w-3" />
										Correspondance métadonnées
									</span>
								{/if}
							</div>

							<button
								type="button"
								class="bg-surface-2 hover:bg-surface-3 border border-stroke/50 text-text-secondary hover:text-text-primary text-xs px-3 py-1.5 rounded-lg transition-colors cursor-pointer shrink-0"
								onclick={() => duplicateStore.ignoreGroup(group)}
							>
								Ignorer ce groupe
							</button>
						</div>

						<!-- Tracks Comparison List -->
						<div class="grid grid-cols-1 md:grid-cols-2 gap-3.5">
							{#each group.tracks as track (track.id)}
								{@const isSelectedForDeletion = $duplicateStore.selectedTrackIdsToDelete.has(track.id)}
								{@const isCurrent = $currentTrack?.id === track.id}
								{@const isPlayingThis = isCurrent && $isPlaying}
								{@const lossless = isTrackLossless(track.format)}
								{@const progressPercent = isCurrent ? Math.min(100, Math.max(0, ($playbackPosition / (track.duration_ms || 1)) * 100)) : 0}

								<div
									class="relative flex flex-col justify-between rounded-xl border p-3.5 transition-all gap-3
									{isSelectedForDeletion
										? 'border-rose-500/40 bg-rose-950/15'
										: track.recommended_keep
											? 'border-amber-500/30 bg-surface-1/90 shadow-sm'
											: 'border-stroke/60 bg-surface-1/70'}"
								>
									<!-- Track Top Info -->
									<div class="space-y-2.5">
										<div class="flex items-start justify-between gap-3">
											<div class="flex items-center gap-3 min-w-0">
												<!-- Preview Player Button -->
												<button
													type="button"
													class="group relative flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border transition-all hover:cursor-pointer
													{isPlayingThis
														? 'border-[#00E5FF] bg-[#00E5FF] text-black shadow-md shadow-[#00E5FF]/40'
														: 'border-stroke bg-surface-2 text-text-primary hover:border-[#00E5FF]/60 hover:bg-[#00E5FF]/15 hover:text-[#00E5FF]'}"
													onclick={() => handleTogglePlay(track)}
													title={isPlayingThis ? 'Mettre en pause' : 'Écouter un extrait'}
												>
													{#if isPlayingThis}
														<Icon name="pause" class="h-4 w-4" fill />
													{:else}
														<Icon name="play" class="h-4 w-4 group-hover:scale-110 transition-transform ml-0.5" fill />
													{/if}
												</button>

												<div class="min-w-0">
													<div class="flex items-center gap-1.5">
														<Text variant="body-2" weight="bold" class="truncate text-text-primary">
															{track.title || 'Sans titre'}
														</Text>
													</div>
													<Text variant="caption" class="truncate text-text-secondary">
														{track.artist || 'Artiste inconnu'} {track.album ? `• ${track.album}` : ''}
													</Text>
												</div>
											</div>

											<!-- Top Right: Star & Duration Highlighted -->
											<div class="flex items-center gap-2 shrink-0">
												{#if track.recommended_keep}
													<span class="text-xs select-none drop-shadow-[0_0_6px_rgba(251,191,36,0.6)] cursor-default" title="Morceau recommandé (meilleure qualité)">⭐</span>
												{/if}
												<span class="font-mono text-xs font-semibold tabular-nums {isPlayingThis ? 'text-[#00E5FF]' : 'text-text-secondary'}">
													{#if isPlayingThis}
														{formatDurationCompact($playbackPosition)} / {formatDurationCompact(track.duration_ms)}
													{:else}
														{formatDurationCompact(track.duration_ms)}
													{/if}
												</span>
											</div>
										</div>

										<!-- Navigable Interactive Audio Waveform in Center -->
										<div
											role="button"
											tabindex="0"
											class="group/wave relative flex h-9 w-full cursor-pointer items-center justify-between gap-[2px] rounded-lg border border-stroke/40 bg-surface-0/70 px-2.5 py-1.5 transition-all hover:border-[#00E5FF]/40 hover:bg-surface-0 select-none"
											onclick={(e) => handleWaveformClick(e, track)}
											onkeydown={(e) => {
												if (e.key === 'Enter' || e.key === ' ') {
													handleTogglePlay(track)
												}
											}}
											title="Cliquer pour naviguer dans le morceau"
										>
											{#each getBarsForTrack(track.id, track.file_hash) as barHeight, barIdx}
												{@const barPercent = (barIdx / 64) * 100}
												{@const isPast = isCurrent && barPercent <= progressPercent}
												<div class="h-full flex-1 flex items-center justify-center">
													<div
														class="w-full rounded-full transition-all duration-75 {isPast
															? 'bg-[#00E5FF] shadow-[0_0_6px_rgba(0,229,255,0.7)]'
															: 'bg-[#38BDF8]/35 group-hover/wave:bg-[#38BDF8]/55'}"
														style="height: {Math.max(16, Math.round(barHeight * 100))}%;"
													></div>
												</div>
											{/each}

											{#if isCurrent}
												<!-- Glowing Playhead -->
												<div
													class="pointer-events-none absolute inset-y-0 w-[2px] bg-[#00E5FF] shadow-[0_0_8px_#00E5FF] transition-[left] duration-100"
													style="left: {progressPercent}%;"
												></div>
											{/if}
										</div>

										<!-- Single-line Streamlined Audio Badges (Beatport/MIK style) -->
										<div class="flex items-center gap-1.5 flex-wrap">
											<!-- Camelot Key Badge -->
											{#if track.key}
												{@const camelot = getCamelotColor(track.key)}
												{@const formattedKey = formatCamelotKey(track.key)}
												{#if camelot}
													<span
														class="inline-flex h-[20px] min-w-[32px] px-1.5 items-center justify-center rounded text-[10px] font-mono font-bold tracking-tight shadow-sm select-none"
														style="background-color: {camelot.bg}; color: {camelot.text};"
														title="{formattedKey} ({camelot.name}) • Camelot Key"
													>
														{formattedKey}
													</span>
												{:else}
													<span class="inline-flex h-[20px] px-1.5 items-center justify-center rounded text-[10px] font-mono bg-surface-3 text-text-secondary border border-stroke/50 select-none">
														{formattedKey}
													</span>
												{/if}
											{/if}

											<!-- BPM Badge -->
											{#if track.bpm}
												<span class="inline-flex h-[20px] items-center px-1.5 rounded text-[10px] font-mono text-text-secondary bg-surface-2 border border-stroke/50 select-none">
													{Math.round(track.bpm)} bpm
												</span>
											{/if}

											<!-- Energy Badge -->
											{#if track.energy}
												<span class="inline-flex h-[20px] items-center px-1.5 rounded text-[10px] font-mono font-bold text-amber-400 bg-amber-500/10 border border-amber-500/30 select-none">
													⚡ {track.energy}
												</span>
											{/if}

											<!-- Format & Bitrate Badge -->
											{#if track.format}
												<span class="inline-flex h-[20px] items-center px-1.5 rounded text-[10px] font-mono font-bold uppercase tracking-wider select-none {lossless ? 'text-emerald-400 bg-emerald-500/10 border border-emerald-500/30' : 'text-text-secondary bg-surface-2 border border-stroke/50'}">
													{track.format} {track.bitrate ? `${track.bitrate} kbps` : ''}
												</span>
											{/if}

											<!-- Sample Rate Badge -->
											{#if track.sample_rate}
												<span class="inline-flex h-[20px] items-center px-1.5 rounded text-[10px] font-mono text-text-tertiary bg-surface-2/60 border border-stroke/40 select-none">
													{(track.sample_rate / 1000).toFixed(track.sample_rate % 1000 !== 0 ? 1 : 0)} kHz
												</span>
											{/if}

											<!-- Cues Badge -->
											{#if track.cue_count > 0}
												<span class="inline-flex h-[20px] items-center px-1.5 rounded text-[10px] font-mono font-medium text-indigo-300 bg-indigo-500/15 border border-indigo-500/30 select-none">
													{track.cue_count} cue{track.cue_count > 1 ? 's' : ''}
												</span>
											{/if}
										</div>

										<!-- Discrete File Path with Icon -->
										<div class="flex items-center gap-1.5 text-[11px] text-text-tertiary font-mono truncate" title={track.file_path}>
											<Icon name="folder" class="h-3 w-3 shrink-0 text-text-disabled" />
											<span class="truncate">{track.file_path}</span>
										</div>
									</div>

									<!-- Action Checkbox -->
									<div class="pt-2.5 border-t border-stroke/40 flex items-center justify-between">
										<label class="flex items-center gap-2 text-xs font-medium hover:cursor-pointer select-none {isSelectedForDeletion ? 'text-rose-400' : 'text-text-secondary'}">
											<input
												type="checkbox"
												checked={isSelectedForDeletion}
												onchange={() => duplicateStore.toggleTrackSelection(track.id)}
												class="h-4 w-4 rounded border-stroke bg-surface-2 text-rose-500 focus:ring-rose-500 hover:cursor-pointer"
											/>
											<span>{isSelectedForDeletion ? 'Marqué pour suppression' : 'Conserver ce morceau'}</span>
										</label>
									</div>
								</div>
							{/each}
						</div>
					</div>
				{/each}
			{/if}
		</div>

		<!-- Footer -->
		<div class="flex items-center justify-between border-t border-stroke px-6 py-3.5 bg-surface-2/80">
			<div class="text-xs">
				<span class="text-text-secondary">Sélectionnés :</span>
				<span class="font-bold text-text-primary ml-1 font-mono">{$selectedDuplicateCount}</span>
			</div>

			<div class="flex items-center gap-3">
				<button
					type="button"
					class="bg-surface-2 hover:bg-surface-3 border border-stroke/50 text-text-secondary hover:text-text-primary text-xs px-3 py-1.5 rounded-lg transition-colors cursor-pointer"
					onclick={onClose}
				>
					Fermer
				</button>

				<Button
					variant="danger"
					onclick={handleDeleteSelected}
					disabled={$selectedDuplicateCount === 0 || isDeleting}
					class="font-semibold shadow-md shadow-rose-950/40 text-xs py-1.5"
				>
					{#if isDeleting}
						<Icon name="refresh-cw" class="mr-2 h-3.5 w-3.5 animate-spin" />
						Suppression en cours...
					{:else}
						<Icon name="trash" class="mr-2 h-3.5 w-3.5" />
						Supprimer les {$selectedDuplicateCount} morceau{$selectedDuplicateCount > 1 ? 'x' : ''} (Corbeille)
					{/if}
				</Button>
			</div>
		</div>
	</div>
</Modal>
