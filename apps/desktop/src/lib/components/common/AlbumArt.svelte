<script lang="ts">
	import { getArtworkUrl } from '$shared/utils'
	import { appDataDir } from '$lib/stores/app'
	import Icon from './Icon.svelte'
	import { translate } from '$shared/i18n'

	type Size = 'xs' | 'sm' | 'md' | 'lg'

	type Props = {
		artworkPath: string | null
		artworkUrl?: string | null
		size?: Size
		class?: string
		onclick?: () => void
	}

	let { artworkPath, artworkUrl = null, size = 'md', class: className = '', onclick }: Props = $props()

	const sizeClasses: Record<Size, string> = {
		xs: 'h-6 w-6',
		sm: 'h-8 w-8',
		md: 'h-12 w-12',
		lg: 'aspect-square w-full max-w-[400px]',
	}

	const iconSizes: Record<Size, string> = {
		xs: 'h-4 w-4',
		sm: 'h-4 w-4',
		md: 'h-6 w-6',
		lg: 'h-16 w-16',
	}

	let localUrl = $derived(getArtworkUrl(artworkPath, $appDataDir))
	let localError = $state(false)
	let externalError = $state(false)

	let displayUrl = $derived.by(() => {
		if (localUrl && !localError) return localUrl
		if (artworkUrl && !externalError) return artworkUrl
		return null
	})

	$effect(() => {
		// Reset error states when props change
		void artworkPath
		localError = false
	})

	$effect(() => {
		void artworkUrl
		externalError = false
	})

	function handleError() {
		if (localUrl && !localError) {
			localError = true
		} else {
			externalError = true
		}
	}
</script>

{#snippet artwork(alt: string)}
	{#if displayUrl}
		<img src={displayUrl} {alt} class="h-full w-full object-cover" onerror={handleError} />
	{:else}
		<Icon name="music-note" class="{iconSizes[size]} text-text-tertiary" />
	{/if}
{/snippet}

<!-- A clickable artwork is a real button (keyboard and name for free); preflight resets its look to the div's. -->
{#if onclick}
	<button
		type="button"
		aria-label={$translate('common.viewArtwork')}
		class="flex flex-shrink-0 items-center justify-center overflow-hidden rounded bg-surface-2 {sizeClasses[
			size
		]} {className}"
		{onclick}
	>
		{@render artwork('')}
	</button>
{:else}
	<div
		class="flex flex-shrink-0 items-center justify-center overflow-hidden rounded bg-surface-2 {sizeClasses[
			size
		]} {className}"
	>
		{@render artwork($translate('common.albumArtwork'))}
	</div>
{/if}
