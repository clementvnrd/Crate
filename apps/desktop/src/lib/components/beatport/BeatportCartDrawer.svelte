<script lang="ts">
	import {
		beatportStore,
		beatportCart,
		beatportCartCount,
		beatportCartDuration,
	} from '$shared/stores/beatport'
	import { formatDurationCompact } from '$shared/utils/format'
	import { Icon, Spinner } from '$lib/components/common'
	import { fly, fade } from 'svelte/transition'

	interface Props {
		isOpen: boolean
		onClose: () => void
	}

	let { isOpen = $bindable(false), onClose }: Props = $props()

	function handleDownload() {
		beatportStore.downloadSelection()
	}

	function handleClear() {
		beatportStore.clearCart()
		onClose()
	}
</script>

{#if isOpen && $beatportCartCount > 0}
	<!-- Backdrop -->
	<div
		transition:fade={{ duration: 150 }}
		class="fixed inset-0 z-40 bg-black/60 backdrop-blur-sm"
		onclick={onClose}
		role="presentation"
	></div>

	<!-- Slide-over Drawer -->
	<aside
		transition:fly={{ x: 400, duration: 250 }}
		class="fixed top-0 right-0 bottom-0 z-50 flex w-full max-w-md flex-col border-l border-emerald-500/30 bg-[#121418] shadow-2xl shadow-emerald-500/10 select-none"
	>
		<!-- Header -->
		<div class="flex items-center justify-between border-b border-[#252830] px-5 py-4 bg-[#181a20]">
			<div class="flex items-center gap-2.5">
				<div class="flex h-8 w-8 items-center justify-center rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/30">
					<Icon name="cart" class="h-4 w-4" />
				</div>
				<div>
					<div class="flex items-center gap-2">
						<h2 class="text-sm font-bold text-white">Panier Beatport</h2>
						<span class="rounded-full bg-emerald-500/20 px-2 py-0.5 text-[11px] font-bold text-emerald-400">
							{$beatportCartCount} morceau{$beatportCartCount > 1 ? 'x' : ''}
						</span>
					</div>
					<p class="text-[11px] text-neutral-400">Durée totale : {formatDurationCompact($beatportCartDuration)}</p>
				</div>
			</div>

			<button
				type="button"
				class="rounded-lg p-1.5 text-neutral-400 hover:bg-neutral-800 hover:text-white transition-colors cursor-pointer"
				onclick={onClose}
				title="Fermer le panier"
			>
				<Icon name="x" class="h-5 w-5" />
			</button>
		</div>

		<!-- Track List Body -->
		<div class="flex-1 overflow-y-auto p-4 space-y-2 divide-y divide-[#252830]/50">
			{#each $beatportCart as track (track.id)}
				<div class="flex items-center justify-between rounded-xl bg-[#181a20] border border-[#2e323d] p-3 text-xs transition-colors hover:border-emerald-500/30">
					<div class="flex items-center gap-3 min-w-0 pr-2">
						{#if track.artwork_url}
							<img src={track.artwork_url} alt="" class="h-10 w-10 rounded-lg object-cover flex-shrink-0 shadow-sm" />
						{:else}
							<div class="h-10 w-10 rounded-lg bg-neutral-800 flex items-center justify-center font-bold text-[10px] text-neutral-400">BP</div>
						{/if}
						<div class="min-w-0 truncate">
							<div class="font-semibold text-white truncate">{track.title}</div>
							<div class="text-[11px] text-neutral-400 truncate">{track.artists.map((a) => a.name).join(', ')}</div>
							<div class="flex items-center gap-2 mt-0.5 text-[10px] text-neutral-500">
								{#if track.key}
									<span class="rounded bg-neutral-800 px-1 py-0.2 text-emerald-400 font-mono">{track.key}</span>
								{/if}
								{#if track.bpm}
									<span>{Math.round(track.bpm)} BPM</span>
								{/if}
								<span>{track.duration_formatted}</span>
							</div>
						</div>
					</div>

					<button
						type="button"
						class="rounded-lg p-1.5 text-neutral-500 hover:bg-red-500/10 hover:text-red-400 transition-colors cursor-pointer"
						onclick={() => beatportStore.removeFromCart(track.id)}
						title="Retirer du panier"
					>
						<Icon name="x" class="h-4 w-4" />
					</button>
				</div>
			{/each}
		</div>

		<!-- Footer -->
		<div class="border-t border-[#252830] bg-[#181a20] p-4 space-y-3">
			<div class="flex items-center justify-between text-xs text-neutral-400">
				<span>Format de sortie : <strong class="text-emerald-400 font-mono">FLAC (Lossless)</strong></span>
				<span>Destination : <strong class="text-neutral-200">{($beatportStore.downloadDestination || 'FLAC').replace(/^.*\/([^\/]+)$/, '$1')}</strong></span>
			</div>

			<div class="flex items-center gap-2">
				{#if $beatportStore.isDownloading}
					<div class="flex w-full items-center justify-center gap-2.5 rounded-xl bg-emerald-950/60 border border-emerald-500/40 py-3 text-xs font-semibold text-emerald-400">
						<Spinner class="h-4 w-4 text-emerald-400" />
						<span>{$beatportStore.downloadProgressText || 'Téléchargement BeatportDL en cours...'}</span>
					</div>
				{:else}
					<button
						type="button"
						class="rounded-xl bg-neutral-800 border border-neutral-700 px-4 py-3 text-xs font-semibold text-neutral-300 hover:bg-neutral-700 hover:text-white transition-colors cursor-pointer"
						onclick={handleClear}
					>
						Vider
					</button>

					<button
						type="button"
						class="flex flex-1 items-center justify-center gap-2 rounded-xl bg-emerald-500 py-3 text-xs font-bold text-black shadow-lg shadow-emerald-500/20 hover:bg-emerald-400 active:scale-98 transition-all cursor-pointer"
						onclick={handleDownload}
					>
						<Icon name="download" class="h-4 w-4" />
						<span>Télécharger en FLAC & Synchro MIK</span>
					</button>
				{/if}
			</div>
		</div>
	</aside>
{/if}
