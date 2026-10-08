<script lang="ts">
	import { fade } from 'svelte/transition'
	import {
		Button,
		ConfirmModal,
		Icon,
		IconButton,
		Spinner,
		Text,
		Tooltip,
		UpdateModal,
		releaseNotesOpen,
	} from '$lib/components/common'
	import { language } from '$lib/stores'
	import { updaterStore, updateBannerVisible } from '$lib/stores/updater'
	import { translate } from '$shared/i18n'

	/**
	 * The in-app update strip, one line in the layout flow under the toolbar (CRA-200). It never opens a window by
	 * itself, never takes focus and never relaunches Crate while music plays or a long job runs: "Update now"
	 * becomes "Install when I quit" then, and "Restart now" asks first. The release notes open on request.
	 */

	let confirmRestartOpen = $state(false)
	let detailsOpen = $state(false)

	const status = $derived($updaterStore.status)
	const version = $derived($updaterStore.version ?? '')
	const busy = $derived($updaterStore.busy)
	const isError = $derived(status === 'error')

	// Under reduced motion the strip appears and disappears at once.
	const reduceMotion =
		typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches === true

	const message = $derived.by(() => {
		const values = { values: { version } }
		switch (status) {
			case 'available':
				return $translate('appUpdate.banner.available', values)
			case 'downloading':
				return $translate('appUpdate.banner.downloading', values)
			case 'installing':
				return $translate('appUpdate.banner.installing', values)
			case 'scheduled':
				return $translate('appUpdate.banner.scheduled', values)
			case 'error':
				return $translate(
					$updaterStore.errorPhase === 'install' ? 'appUpdate.banner.installFailed' : 'appUpdate.banner.downloadFailed',
					values
				)
			default:
				return ''
		}
	})

	const icon = $derived(isError ? 'alert-circle' : status === 'scheduled' ? 'check' : 'download')

	const percent = $derived(
		$updaterStore.progress === null
			? null
			: new Intl.NumberFormat($language, { style: 'percent', maximumFractionDigits: 0 }).format(
					$updaterStore.progress / 100
				)
	)

	// The raw error belongs to the error it describes: close it when the state moves on.
	$effect(() => {
		if (!isError) detailsOpen = false
	})

	function handleRestart() {
		if (busy) {
			confirmRestartOpen = true
			return
		}
		updaterStore.restartNow()
	}
</script>

<div id="update-banner" class="flex-shrink-0">
	<!-- The live region stays in the page (empty while hidden) so screen readers announce the strip when it appears
	     and when its message changes. It holds the message only, not the button labels. -->
	<p id="update-banner-status" class="sr-only" role="status" aria-live="polite">
		{$updateBannerVisible ? message : ''}
	</p>
	{#if $updateBannerVisible}
		<div class="bg-surface-1" data-state={status} transition:fade={{ duration: reduceMotion ? 0 : 150 }}>
			<div class="flex min-h-9 min-w-0 items-center gap-3 px-4 py-1 {isError ? 'bg-danger/10' : 'bg-brand-primary/10'}">
				{#if status === 'installing'}
					<Spinner icon="loader" color="current" class="h-4 w-4 flex-shrink-0 text-brand-primary" />
				{:else}
					<Icon name={icon} class="h-4 w-4 flex-shrink-0 {isError ? 'text-danger' : 'text-brand-primary'}" />
				{/if}

				<Text size="sm" truncate class="min-w-0 flex-1" title={message}>{message}</Text>

				{#if status === 'downloading'}
					<!-- The percentage is read from the progress bar; as text it would be announced at every step. -->
					{#if percent !== null}
						<span class="w-10 flex-shrink-0 text-right text-xs text-text-secondary tabular-nums" aria-hidden="true"
							>{percent}</span
						>
					{/if}
					<div
						class="h-1 w-32 flex-shrink-0 overflow-hidden rounded-full bg-surface-3"
						role="progressbar"
						aria-label={$translate('appUpdate.progress')}
						aria-valuemin={0}
						aria-valuemax={100}
						aria-valuenow={$updaterStore.progress ?? undefined}
					>
						{#if $updaterStore.progress === null}
							<div class="h-full w-full animate-pulse rounded-full bg-brand-primary motion-reduce:animate-none"></div>
						{:else}
							<div
								class="h-full w-full origin-left rounded-full bg-brand-primary transition-transform duration-200 motion-reduce:transition-none"
								style="transform: scaleX({$updaterStore.progress / 100})"
							></div>
						{/if}
					</div>
				{/if}

				{#if status === 'available'}
					<div class="flex flex-shrink-0 items-center gap-1">
						<Button size="sm" variant="ghost" class="whitespace-nowrap" onclick={() => releaseNotesOpen.set(true)}>
							{$translate('appUpdate.actions.seeWhatsNew')}
						</Button>
						<Button size="sm" variant="ghost" class="whitespace-nowrap" onclick={() => updaterStore.skipVersion()}>
							{$translate('appUpdate.actions.skipVersion')}
						</Button>
						<Button size="sm" variant="ghost" class="whitespace-nowrap" onclick={() => updaterStore.later()}>
							{$translate('appUpdate.actions.later')}
						</Button>
						<Button size="sm" variant="primary" class="ml-1 whitespace-nowrap" onclick={() => updaterStore.updateNow()}>
							{$translate(busy ? 'appUpdate.actions.installOnQuit' : 'appUpdate.actions.updateNow')}
						</Button>
					</div>
				{:else if status === 'scheduled'}
					<Button size="sm" variant="primary" class="flex-shrink-0 whitespace-nowrap" onclick={handleRestart}>
						{$translate('appUpdate.actions.restartNow')}
					</Button>
				{:else if isError}
					<div class="flex flex-shrink-0 items-center gap-1">
						{#if $updaterStore.error}
							<Tooltip
								open={detailsOpen}
								position="bottom"
								bubbleClass="max-w-sm rounded border border-stroke bg-surface-1 px-2 py-1 font-mono text-xs break-words text-text-primary shadow-lg"
								text={$updaterStore.error}
							>
								<Button
									size="sm"
									variant="ghost"
									class="whitespace-nowrap"
									onclick={() => (detailsOpen = !detailsOpen)}
								>
									{$translate('appUpdate.actions.details')}
								</Button>
							</Tooltip>
						{/if}
						<Button size="sm" variant="primary" class="ml-1 whitespace-nowrap" onclick={() => updaterStore.retry()}>
							{$translate('appUpdate.actions.retry')}
						</Button>
					</div>
				{/if}

				{#if status === 'available' || status === 'scheduled' || isError}
					<IconButton
						size="sm"
						icon="x"
						iconClass="h-3.5 w-3.5"
						class="flex-shrink-0"
						title={$translate('appUpdate.actions.hide')}
						ariaLabel={$translate('appUpdate.actions.hide')}
						onclick={() => updaterStore.dismiss()}
					/>
				{/if}
			</div>
		</div>
	{/if}
</div>

<UpdateModal open={$releaseNotesOpen} onClose={() => releaseNotesOpen.set(false)} />

<ConfirmModal
	open={confirmRestartOpen}
	title={$translate('appUpdate.restartConfirm.title')}
	message={$translate('appUpdate.restartConfirm.message')}
	confirmLabel={$translate('appUpdate.actions.restartNow')}
	destructive
	onConfirm={() => {
		confirmRestartOpen = false
		updaterStore.restartNow()
	}}
	onCancel={() => (confirmRestartOpen = false)}
/>
