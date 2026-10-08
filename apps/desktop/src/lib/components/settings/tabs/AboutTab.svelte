<script lang="ts">
	import { Button, Icon, Spinner, Text, releaseNotesOpen } from '$lib/components/common'
	import { appEnvironment, appInfo } from '$lib/stores/app'
	import { language } from '$lib/stores'
	import { updaterStore } from '$lib/stores/updater'
	import { translate } from '$shared/i18n'
	import { formatTimeAgo } from '$shared/utils'

	/** "Last checked …" is recomputed every 30 seconds while the tab is open. */
	const CLOCK_TICK_MS = 30_000

	let now = $state(Date.now())

	$effect(() => {
		const id = setInterval(() => (now = Date.now()), CLOCK_TICK_MS)
		return () => clearInterval(id)
	})

	const version = $derived($updaterStore.version ?? '')
	const status = $derived($updaterStore.status)
	const inFlight = $derived(
		status === 'checking' || status === 'downloading' || status === 'installing' || status === 'scheduled'
	)

	const statusLabel = $derived.by(() => {
		const values = { values: { version } }
		switch (status) {
			case 'checking':
				return $translate('settings.about.checking')
			case 'upToDate':
				return $translate('settings.about.upToDate')
			case 'available':
				return $translate('appUpdate.banner.available', values)
			case 'downloading':
				return $translate('appUpdate.banner.downloading', values)
			case 'installing':
				return $translate('appUpdate.banner.installing', values)
			case 'scheduled':
				return $translate('appUpdate.banner.scheduled', values)
			case 'error':
				if ($updaterStore.errorPhase === 'download') return $translate('appUpdate.banner.downloadFailed', values)
				if ($updaterStore.errorPhase === 'install') return $translate('appUpdate.banner.installFailed', values)
				return $translate('settings.about.checkFailed')
			default:
				return ''
		}
	})

	const lastCheckedLabel = $derived.by(() => {
		const lastChecked = $updaterStore.lastChecked
		if (lastChecked === null) return $translate('settings.about.neverChecked')
		// A check that just finished is newer than the last clock tick.
		const ago = formatTimeAgo(lastChecked, Math.max(now, lastChecked), $language)
		return ago === null
			? $translate('settings.about.lastCheckedJustNow')
			: $translate('settings.about.lastChecked', { values: { time: ago } })
	})

	// Which builds this copy follows; switching is not possible (Staging is a separate app with its own library).
	const channelLabel = $derived.by(() => {
		switch ($appEnvironment) {
			case 'production':
				return $translate('settings.about.channelStable')
			case 'staging':
				return $translate('settings.about.channelStaging')
			default:
				return $translate('settings.about.channelDevelopment')
		}
	})
</script>

<div class="space-y-8">
	<!-- Application Section -->
	<section>
		<Text variant="header-3" class="mb-4">{$translate('settings.about.application')}</Text>
		<div class="space-y-3">
			<div class="flex justify-between">
				<Text size="sm" color="secondary" as="span">{$translate('settings.about.version')}</Text>
				<Text size="sm" as="span">{$appInfo?.version ?? $translate('common.unknown')}</Text>
			</div>
			<div class="flex justify-between">
				<Text size="sm" color="secondary" as="span">{$translate('settings.about.environment')}</Text>
				<Text size="sm" as="span" class="capitalize">{$appInfo?.environment ?? $translate('common.unknown')}</Text>
			</div>
			<div class="flex justify-between">
				<Text size="sm" color="secondary" as="span">{$translate('settings.diagnostics.dataDirectory')}</Text>
				<Text variant="code" truncate class="max-w-xs" title={$appInfo?.dataDir}>
					{$appInfo?.dataDir ?? $translate('common.unknown')}
				</Text>
			</div>
		</div>
	</section>

	<!-- Updates Section -->
	<section>
		<Text variant="header-3" class="mb-4">{$translate('settings.about.updates')}</Text>
		<div class="space-y-3">
			<div class="flex items-center justify-between gap-4">
				<div class="min-w-0 space-y-0.5">
					<div id="about-update-status" class="flex min-w-0 items-start gap-2" role="status" aria-live="polite">
						{#if status === 'checking'}
							<Spinner class="mt-0.5 h-4 w-4 flex-shrink-0" />
						{:else if status === 'error'}
							<Icon name="alert-circle" class="mt-0.5 h-4 w-4 flex-shrink-0 text-danger" />
						{/if}
						{#if statusLabel}
							<Text size="sm" as="span">{statusLabel}</Text>
						{/if}
					</div>
					<Text size="xs" color="secondary">{lastCheckedLabel}</Text>
				</div>
				<div class="flex flex-shrink-0 items-center gap-2">
					{#if status === 'available'}
						<Button size="sm" variant="ghost" onclick={() => releaseNotesOpen.set(true)}>
							{$translate('appUpdate.actions.seeWhatsNew')}
						</Button>
					{/if}
					<Button size="sm" variant="secondary" disabled={inFlight} onclick={() => updaterStore.check(false)}>
						{$translate('settings.about.checkForUpdates')}
					</Button>
				</div>
			</div>
			<div class="flex justify-between">
				<Text size="sm" color="secondary" as="span">{$translate('settings.about.channel')}</Text>
				<Text size="sm" as="span">{channelLabel}</Text>
			</div>
		</div>
	</section>
</div>
