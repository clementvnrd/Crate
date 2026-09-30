<script lang="ts">
	import { beatportStore, beatportCart, beatportCartCount, beatportCartDuration } from '$shared/stores/beatport'
	import { formatDurationCompact } from '$shared/utils/format'
	import { Icon, Spinner, focusTrap } from '$lib/components/common'
	import { fly, fade } from 'svelte/transition'
	import { translate } from '$shared/i18n'

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
	<div
		transition:fly={{ x: 400, duration: 250 }}
		role="dialog"
		aria-modal="true"
		aria-labelledby="beatport-cart-title"
		use:focusTrap={{ onEscape: onClose }}
		class="fixed top-0 right-0 bottom-0 z-50 flex w-full max-w-md flex-col border-l border-emerald-500/30 bg-[#121418] shadow-2xl shadow-emerald-500/10 select-none focus-visible:outline-none"
	>
		<!-- Header -->
		<div class="flex items-center justify-between border-b border-[#252830] bg-[#181a20] px-5 py-4">
			<div class="flex items-center gap-2.5">
				<div
					class="flex h-8 w-8 items-center justify-center rounded-lg border border-emerald-500/30 bg-emerald-500/10 text-emerald-400"
				>
					<Icon name="cart" class="h-4 w-4" />
				</div>
				<div>
					<div class="flex items-center gap-2">
						<h2 id="beatport-cart-title" class="text-sm font-bold text-white">{$translate('beatport.cart.title')}</h2>
						<span class="rounded-full bg-emerald-500/20 px-2 py-0.5 text-[11px] font-bold text-emerald-400">
							{$translate('beatport.cart.trackCount', { values: { count: $beatportCartCount } })}
						</span>
					</div>
					<p class="text-[11px] text-neutral-400">
						{$translate('beatport.cart.totalDuration', {
							values: { duration: formatDurationCompact($beatportCartDuration) },
						})}
					</p>
				</div>
			</div>

			<button
				type="button"
				class="cursor-pointer rounded-lg p-1.5 text-neutral-400 transition-colors hover:bg-neutral-800 hover:text-white"
				onclick={onClose}
				title={$translate('beatport.cart.close')}
			>
				<Icon name="x" class="h-5 w-5" />
			</button>
		</div>

		<!-- Track List Body -->
		<div class="flex-1 space-y-2 divide-y divide-[#252830]/50 overflow-y-auto p-4">
			{#each $beatportCart as track (track.id)}
				<div
					class="flex items-center justify-between rounded-xl border border-[#2e323d] bg-[#181a20] p-3 text-xs transition-colors hover:border-emerald-500/30"
				>
					<div class="flex min-w-0 items-center gap-3 pr-2">
						{#if track.artwork_url}
							<img src={track.artwork_url} alt="" class="h-10 w-10 flex-shrink-0 rounded-lg object-cover shadow-sm" />
						{:else}
							<div
								class="flex h-10 w-10 items-center justify-center rounded-lg bg-neutral-800 text-[10px] font-bold text-neutral-400"
							>
								BP
							</div>
						{/if}
						<div class="min-w-0 truncate">
							<div class="truncate font-semibold text-white">{track.title}</div>
							<div class="truncate text-[11px] text-neutral-400">{track.artists.map((a) => a.name).join(', ')}</div>
							<div class="mt-0.5 flex items-center gap-2 text-[10px] text-neutral-500">
								{#if track.key}
									<span class="py-0.2 rounded bg-neutral-800 px-1 font-mono text-emerald-400">{track.key}</span>
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
						class="cursor-pointer rounded-lg p-1.5 text-neutral-500 transition-colors hover:bg-red-500/10 hover:text-red-400"
						onclick={() => beatportStore.removeFromCart(track.id)}
						title={$translate('beatport.cart.remove')}
					>
						<Icon name="x" class="h-4 w-4" />
					</button>
				</div>
			{/each}
		</div>

		<!-- Footer -->
		<div class="space-y-3 border-t border-[#252830] bg-[#181a20] p-4">
			<div class="flex items-center justify-between text-xs text-neutral-400">
				<span
					>{$translate('beatport.cart.outputFormat')}
					<strong class="font-mono text-emerald-400">{$translate('beatport.cart.formatFlac')}</strong></span
				>
				<span
					>{$translate('beatport.cart.destination')}
					<strong class="text-neutral-200"
						>{($beatportStore.downloadDestination || 'FLAC').replace(/^.*\/([^/]+)$/, '$1')}</strong
					></span
				>
			</div>

			<div class="flex items-center gap-2">
				{#if $beatportStore.isDownloading}
					<div
						class="flex w-full items-center justify-center gap-2.5 rounded-xl border border-emerald-500/40 bg-emerald-950/60 py-3 text-xs font-semibold text-emerald-400"
					>
						<Spinner class="h-4 w-4 text-emerald-400" />
						<span>{$beatportStore.downloadProgressText || $translate('beatport.cart.downloading')}</span>
					</div>
				{:else}
					<button
						type="button"
						class="cursor-pointer rounded-xl border border-neutral-700 bg-neutral-800 px-4 py-3 text-xs font-semibold text-neutral-300 transition-colors hover:bg-neutral-700 hover:text-white"
						onclick={handleClear}
					>
						{$translate('beatport.cart.clear')}
					</button>

					<button
						type="button"
						class="flex flex-1 cursor-pointer items-center justify-center gap-2 rounded-xl bg-emerald-500 py-3 text-xs font-bold text-black shadow-lg shadow-emerald-500/20 transition-all hover:bg-emerald-400 active:scale-98"
						onclick={handleDownload}
					>
						<Icon name="download" class="h-4 w-4" />
						<span>{$translate('beatport.cart.download')}</span>
					</button>
				{/if}
			</div>
		</div>
	</div>
{/if}
