// Focus trap for the overlays that keep their own look instead of the common `Modal` (the Beatport login and cart
// panels, the Spotify connection panel in Pulse). Same behaviour as `Modal`: Tab and Shift+Tab cycle inside the
// overlay, Escape closes it, and focus goes back where it was when the overlay closes.
//
// On open, focus moves to the overlay's panel itself (tabindex -1, no outline), never to its first field: a mouse
// user sees exactly what they saw before (no field highlighted), and the next Tab lands on the first control.

export const FOCUSABLE_SELECTOR = [
	'a[href]:not([tabindex="-1"])',
	'input:not([disabled]):not([type="hidden"]):not([tabindex="-1"])',
	'button:not([disabled]):not([tabindex="-1"])',
	'select:not([disabled]):not([tabindex="-1"])',
	'textarea:not([disabled]):not([tabindex="-1"])',
	'[tabindex]:not([tabindex="-1"]):not([disabled])',
].join(', ')

/**
 * The index Tab (or Shift+Tab when `backwards`) should move to among `count` focusable elements, wrapping at both
 * ends. `current` is -1 when focus is on none of them (on the panel itself): Tab then goes to the first one and
 * Shift+Tab to the last. Returns -1 when there is nothing to focus.
 */
export function nextFocusIndex(count: number, current: number, backwards: boolean): number {
	if (count <= 0) return -1
	if (current < 0 || current >= count) return backwards ? count - 1 : 0
	if (backwards) return current === 0 ? count - 1 : current - 1
	return current === count - 1 ? 0 : current + 1
}

export interface FocusTrapOptions {
	/** Called on Escape (the overlay closes itself). Leave out when the overlay already handles Escape. */
	onEscape?: () => void
}

function isVisible(element: HTMLElement): boolean {
	return element.getClientRects().length > 0
}

/** Svelte action: `<div use:focusTrap={{ onEscape: close }}>`. */
export function focusTrap(node: HTMLElement, options: FocusTrapOptions = {}) {
	let current = options
	const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null

	if (!node.hasAttribute('tabindex')) node.setAttribute('tabindex', '-1')
	if (!node.contains(document.activeElement)) node.focus({ preventScroll: true })

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && current.onEscape) {
			e.preventDefault()
			e.stopPropagation()
			current.onEscape()
			return
		}
		if (e.key !== 'Tab') return
		const focusable = [...node.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)].filter(isVisible)
		const index = nextFocusIndex(focusable.length, focusable.indexOf(document.activeElement as HTMLElement), e.shiftKey)
		e.preventDefault()
		if (index >= 0) focusable[index].focus()
	}

	node.addEventListener('keydown', handleKeydown)

	return {
		update(next: FocusTrapOptions = {}) {
			current = next
		},
		destroy() {
			node.removeEventListener('keydown', handleKeydown)
			if (previous && previous.isConnected) previous.focus({ preventScroll: true })
		},
	}
}
