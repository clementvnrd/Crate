<script lang="ts">
	import { toErrorMessage } from '$shared/utils/errors'
	import { beatportStore } from '$shared/stores/beatport'
	import { settingsStore } from '$shared/stores/settings'
	import * as settingsApi from '$shared/api/settings'
	import { Button, Text, Icon } from '$lib/components/common'
	import { toastStore } from '$shared/stores/toast'
	import { open } from '@tauri-apps/plugin-dialog'
	import { withNativeDialog } from '$shared/utils'
	import { translate } from '$shared/i18n'

	let downloadDest = $state(
		$settingsStore.beatportDownloadDestination || $beatportStore.downloadDestination || '~/Music/My Library/FLAC'
	)
	let isSaving = $state(false)

	async function handleBrowseFolder() {
		const selected = await withNativeDialog(() =>
			open({
				directory: true,
				multiple: false,
				title: $translate('settings.beatport.dialogTitle'),
			})
		)
		if (selected && typeof selected === 'string') {
			downloadDest = selected
		}
	}

	async function handleSave() {
		isSaving = true
		try {
			await settingsStore.setBeatportDownloadDestination(downloadDest)
			await settingsApi.setSetting('beatport_download_destination', downloadDest)
			beatportStore.setDownloadDestination(downloadDest)
			toastStore.success($translate('settings.beatport.toast.saved'))
		} catch (e) {
			toastStore.error(
				$translate('settings.beatport.toast.saveFailed', { values: { error: toErrorMessage(e, String(e)) } })
			)
		} finally {
			isSaving = false
		}
	}

	function handleOpenLogin() {
		beatportStore.openLoginModal()
	}

	function handleLogout() {
		beatportStore.logout()
		toastStore.info($translate('settings.beatport.toast.loggedOut'))
	}
</script>

<div class="max-w-xl space-y-6 select-none">
	<!-- Section 1: Compte Beatport -->
	<div>
		<div class="mb-2 text-xs font-semibold text-text-primary">{$translate('settings.beatport.account')}</div>
		<div
			class="flex items-center justify-between rounded-lg border border-stroke/70 bg-surface-1 px-3.5 py-2.5 shadow-xs"
		>
			<div class="flex min-w-0 items-center gap-3">
				<Icon name="beatport" class="h-4 w-4 flex-shrink-0 text-text-primary" />
				<div class="flex min-w-0 items-center gap-2">
					{#if $beatportStore.auth.is_authenticated}
						<span class="truncate text-xs font-semibold text-text-primary">{$beatportStore.auth.username}</span>
						<span
							class="h-1.5 w-1.5 flex-shrink-0 rounded-full bg-beatport-tint"
							title={$translate('beatport.connected')}
						></span>
					{:else}
						<span class="text-xs text-text-secondary">{$translate('settings.beatport.notConnected')}</span>
					{/if}
				</div>
			</div>

			<div>
				{#if $beatportStore.auth.is_authenticated}
					<button
						type="button"
						onclick={handleLogout}
						class="cursor-pointer text-xs text-text-tertiary transition-colors hover:text-red-500"
					>
						{$translate('settings.beatport.signOut')}
					</button>
				{:else}
					<Button variant="primary" size="sm" onclick={handleOpenLogin}>{$translate('beatport.signIn')}</Button>
				{/if}
			</div>
		</div>
	</div>

	<!-- Section 2: Format -->
	<div>
		<div class="mb-1 text-xs font-semibold text-text-primary">{$translate('settings.beatport.formatTitle')}</div>
		<p class="text-[11px] text-text-tertiary">
			{$translate('settings.beatport.formatHint')}
		</p>
	</div>

	<!-- Section 3: Dossier de téléchargement -->
	<div>
		<label for="bp-dest-dir" class="mb-1 block text-xs font-semibold text-text-primary"
			>{$translate('settings.beatport.destTitle')}</label
		>
		<p class="mb-2 text-[11px] text-text-tertiary">{$translate('settings.beatport.destHint')}</p>
		<div class="flex items-center gap-2">
			<input
				id="bp-dest-dir"
				type="text"
				bind:value={downloadDest}
				placeholder="/Users/.../Music/My Library/FLAC"
				class="flex-1 rounded-lg border border-stroke bg-surface-2 px-3 py-1.5 font-mono text-xs text-text-primary focus:border-brand-primary focus:outline-none"
			/>
			<Button variant="secondary" size="sm" onclick={handleBrowseFolder}
				>{$translate('settings.beatport.browse')}</Button
			>
		</div>
	</div>

	<!-- Section 4: Import -->
	<p class="text-[11px] text-text-tertiary">
		{$translate('settings.beatport.importHint')}
	</p>

	<!-- Save Action -->
	<div class="pt-2">
		<Button variant="primary" size="sm" onclick={handleSave} disabled={isSaving}>{$translate('common.save')}</Button>
	</div>
</div>
