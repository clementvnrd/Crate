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
		data-surface="beatport"
		use:focusTrap={{ onEscape: onClose }}
		class="fixed top-0 right-0 bottom-0 z-50 flex w-full max-w-md flex-col border-l border-beatport-tint/30 bg-surface-1 shadow-2xl shadow-beatport-tint/10 select-none focus-visible:outline-none"
	>
		<!-- Header -->
		<div class="flex items-center justify-between border-b border-stroke-subtle bg-surface-2 px-5 py-4">
			<div class="flex items-center gap-2.5">
				<div
					class="flex h-8 w-8 items-center justify-center rounded-lg border border-beatport-tint/30 bg-beatport-tint/10 text-beatport-text"
				>
					<Icon name="cart" class="h-4 w-4" />
				</div>
				<div>
					<div class="flex items-center gap-2">
						<h2 id="beatport-cart-title" class="text-sm font-bold text-text-primary">
							{$translate('beatport.cart.title')}
						</h2>
						<span class="rounded-full bg-beatport-tint/20 px-2 py-0.5 text-[11px] font-bold text-beatport-text">
							{$translate('beatport.cart.trackCount', { values: { count: $beatportCartCount } })}
						</span>
					</div>
					<p class="text-[11px] text-text-secondary">
						{$translate('beatport.cart.totalDuration', {
							values: { duration: formatDurationCompact($beatportCartDuration) },
						})}
					</p>
				</div>
			</div>

			<button
				type="button"
				class="cursor-pointer rounded-lg p-1.5 text-text-secondary transition-colors hover:bg-surface-3 hover:text-text-primary"
				onclick={onClose}
				title={$translate('beatport.cart.close')}
			>
				<Icon name="x" class="h-5 w-5" />
			</button>
		</div>

		<!-- Track List Body -->
		<div class="flex-1 space-y-2 divide-y divide-stroke-subtle/50 overflow-y-auto p-4">
			{#each $beatportCart as track (track.id)}
				<div
					class="flex items-center justify-between rounded-xl border border-stroke bg-surface-2 p-3 text-xs transition-colors hover:border-beatport-tint/30"
				>
					<div class="flex min-w-0 items-center gap-3 pr-2">
						{#if track.artwork_url}
							<img src={track.artwork_url} alt="" class="h-10 w-10 flex-shrink-0 rounded-lg object-cover shadow-sm" />
						{:else}
							<div
								class="flex h-10 w-10 items-center justify-center rounded-lg bg-surface-3 text-[10px] font-bold text-text-secondary"
							>
								BP
							</div>
						{/if}
						<div class="min-w-0 truncate">
							<div class="truncate font-semibold text-text-primary">{track.title}</div>
							<div class="truncate text-[11px] text-text-secondary">{track.artists.map((a) => a.name).join(', ')}</div>
							<div class="mt-0.5 flex items-center gap-2 text-[10px] text-text-tertiary">
								{#if track.key}
									<span class="py-0.2 rounded bg-surface-3 px-1 font-mono text-beatport-text">{track.key}</span>
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
						class="cursor-pointer rounded-lg p-1.5 text-text-tertiary transition-colors hover:bg-danger-tint/10 hover:text-danger-text"
						onclick={() => beatportStore.removeFromCart(track.id)}
						title={$translate('beatport.cart.remove')}
					>
						<Icon name="x" class="h-4 w-4" />
					</button>
				</div>
			{/each}
		</div>

		<!-- Footer -->
		<div class="space-y-3 border-t border-stroke-subtle bg-surface-2 p-4">
			<div class="flex items-center justify-between text-xs text-text-secondary">
				<span
					>{$translate('beatport.cart.outputFormat')}
					<strong class="font-mono text-beatport-text">{$translate('beatport.cart.formatFlac')}</strong></span
				>
				<span
					>{$translate('beatport.cart.destination')}
					<strong class="text-beatport-body-text-strong"
						>{($beatportStore.downloadDestination || 'FLAC').replace(/^.*\/([^/]+)$/, '$1')}</strong
					></span
				>
			</div>

			<div class="flex items-center gap-2">
				{#if $beatportStore.isDownloading}
					<div
						class="flex w-full items-center justify-center gap-2.5 rounded-xl border border-beatport-tint/40 bg-beatport-wash/60 py-3 text-xs font-semibold text-beatport-text"
					>
						<Spinner class="h-4 w-4 text-beatport-text" />
						<span>{$beatportStore.downloadProgressText || $translate('beatport.cart.downloading')}</span>
					</div>
				{:else}
					<button
						type="button"
						class="cursor-pointer rounded-xl border border-stroke-strong bg-surface-3 px-4 py-3 text-xs font-semibold text-beatport-body-text transition-colors hover:bg-surface-4 hover:text-text-primary"
						onclick={handleClear}
					>
						{$translate('beatport.cart.clear')}
					</button>

					<button
						type="button"
						class="flex flex-1 cursor-pointer items-center justify-center gap-2 rounded-xl bg-beatport-tint py-3 text-xs font-bold text-black shadow-lg shadow-beatport-tint/20 transition-all hover:bg-beatport-bright active:scale-98"
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
