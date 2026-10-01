<script lang="ts">
	import type { Snippet } from 'svelte'
	import { scale } from 'svelte/transition'
	import Text from './Text.svelte'

	type Props = {
		open: boolean
		title?: string
		/** `none`: no maximum width, `panelClass` gives it. */
		size?: 'sm' | 'md' | 'lg' | 'xl' | '2xl' | '3xl' | '4xl' | 'none'
		flush?: boolean
		/**
		 * Look of the panel (border, background, radius, shadow), replacing the default
		 * `rounded-lg border border-stroke bg-surface-1 text-text-primary shadow-xl`. For a family modal that keeps
		 * its own look (Spotify's connection modal); the position and the height bound stay the Modal's.
		 */
		panelClass?: string
		/** Backdrop behind the panel, replacing the default `backdrop:bg-black/60`. */
		backdropClass?: string
		/** `dark`: the panel renders with the dark theme's tokens in both themes (a dark family panel). */
		theme?: 'dark'
		onClose: () => void
		onSubmit?: () => void
		children: Snippet
		footer?: Snippet
	}

	let {
		open,
		title,
		size = 'sm',
		flush = false,
		panelClass = 'rounded-lg border border-stroke bg-surface-1 text-text-primary shadow-xl',
		backdropClass = 'backdrop:bg-black/60',
		theme,
		onClose,
		onSubmit,
		children,
		footer,
	}: Props = $props()

	const sizeClasses: Record<string, string> = {
		sm: 'max-w-sm',
		md: 'max-w-md',
		lg: 'max-w-xl',
		xl: 'max-w-2xl',
		'2xl': 'max-w-4xl',
		'3xl': 'max-w-5xl',
		'4xl': 'max-w-6xl',
		none: '',
	}

	let dialogEl: HTMLDialogElement | undefined = $state()
	let visible = $state(false)
	let mousedownTarget: EventTarget | null = $state(null)

	// Open dialog when open becomes true
	$effect(() => {
		if (!dialogEl) return
		if (open) {
			visible = true
			dialogEl.showModal()
		} else if (visible) {
			visible = false
		}
	})

	// Handle transition end to close dialog
	function handleOutroEnd() {
		dialogEl?.close()
		visible = false
	}

	const FOCUSABLE_SELECTOR =
		'input:not([disabled]):not([tabindex="-1"]), button:not([disabled]):not([tabindex="-1"]), select:not([disabled]):not([tabindex="-1"]), textarea:not([disabled]):not([tabindex="-1"]), [tabindex]:not([tabindex="-1"]):not([disabled])'

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.stopPropagation()
			e.preventDefault()
			onClose()
		} else if (e.key === 'Enter' && onSubmit) {
			const target = e.target as HTMLElement
			if (target.tagName !== 'TEXTAREA' && target.tagName !== 'BUTTON') {
				e.stopPropagation()
				e.preventDefault()
				onSubmit()
			}
		} else if (e.key === 'Tab') {
			trapFocus(e)
		}
	}

	function trapFocus(e: KeyboardEvent) {
		if (!dialogEl) return
		const focusable = [...dialogEl.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)]
		if (focusable.length === 0) return

		const active = document.activeElement as HTMLElement
		const currentIndex = focusable.indexOf(active)

		let nextIndex: number
		if (e.shiftKey) {
			nextIndex = currentIndex <= 0 ? focusable.length - 1 : currentIndex - 1
		} else {
			nextIndex = currentIndex >= focusable.length - 1 ? 0 : currentIndex + 1
		}

		e.preventDefault()
		focusable[nextIndex].focus()
	}

	function handleBackdropMousedown(e: MouseEvent) {
		mousedownTarget = e.target
	}

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === dialogEl && mousedownTarget === dialogEl) {
			onClose()
		}
	}
</script>

<dialog
	bind:this={dialogEl}
	class="fixed inset-0 m-0 h-full max-h-none w-full max-w-none bg-transparent p-0 {backdropClass}"
	onkeydown={handleKeydown}
	onmousedown={handleBackdropMousedown}
	onclick={handleBackdropClick}
>
	{#if visible}
		<div
			class="fixed top-1/2 left-1/2 flex max-h-[calc(100vh-2rem)] w-[calc(100vw-2rem)] {sizeClasses[size] ??
				'max-w-md'} -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden {panelClass}"
			data-theme={theme}
			transition:scale={{ start: 0.95, duration: 200 }}
			onoutroend={handleOutroEnd}
		>
			{#if title}
				<div class="border-b border-stroke-subtle px-4 py-3">
					<Text variant="header-1" weight="medium">{title}</Text>
				</div>
			{/if}

			<div
				class="min-h-0 {flush ? 'flex h-full min-h-0 flex-1 flex-col overflow-hidden' : 'overflow-y-auto px-4 py-4'}"
			>
				{@render children()}
			</div>

			{#if footer}
				<div class="flex justify-end gap-2 border-t border-stroke-subtle px-4 py-3">
					{@render footer()}
				</div>
			{/if}
		</div>
	{/if}
</dialog>
