<script lang="ts">
	import { updaterStore } from '$lib/stores/updater'
	import { translate } from '$shared/i18n'
	import Modal from './Modal.svelte'
	import Button from './Button.svelte'
	import Text from './Text.svelte'
	import { parseReleaseNotes } from './releaseNotes'

	/**
	 * "What's new in Crate {version}": the release notes of the update the updater found. Opened only on request
	 * (the update banner's "See what's new", Settings → About), never by the updater itself. Its primary action is
	 * the banner's: "Update now", or "Install when I quit" while music plays or a long job runs.
	 */
	type Props = {
		open: boolean
		onClose: () => void
	}

	let { open, onClose }: Props = $props()

	const notes = $derived(parseReleaseNotes($updaterStore.body))
	const canUpdate = $derived($updaterStore.status === 'available' && $updaterStore.update !== null)

	function handleUpdate() {
		// Close first: the banner under the toolbar shows the download and the install.
		onClose()
		updaterStore.updateNow()
	}
</script>

<Modal
	{open}
	size="md"
	title={$translate('appUpdate.notes.title', { values: { version: $updaterStore.version ?? '' } })}
	{onClose}
>
	{#if notes.length > 0}
		<div class="space-y-3">
			{#each notes as block, index (index)}
				{#if block.kind === 'heading'}
					<Text variant="header-2" class={index > 0 ? 'pt-1' : ''}>{block.text}</Text>
				{:else if block.kind === 'paragraph'}
					<Text size="sm" color="secondary" class="break-words">{block.text}</Text>
				{:else}
					<ul class="list-disc space-y-1 pl-5 text-sm break-words text-text-secondary marker:text-text-tertiary">
						{#each block.items as item, itemIndex (itemIndex)}
							<li>{item}</li>
						{/each}
					</ul>
				{/if}
			{/each}
		</div>
	{:else}
		<Text size="sm" color="secondary">{$translate('appUpdate.notes.empty')}</Text>
	{/if}

	{#snippet footer()}
		<Button variant="ghost" onclick={onClose}>{$translate('common.close')}</Button>
		{#if canUpdate}
			<Button variant="primary" onclick={handleUpdate}>
				{$translate($updaterStore.busy ? 'appUpdate.actions.installOnQuit' : 'appUpdate.actions.updateNow')}
			</Button>
		{/if}
	{/snippet}
</Modal>
