import { afterEach, describe, expect, it, vi } from 'vitest'
import { focusTrap, nextFocusIndex } from './focusTrap'

describe('nextFocusIndex', () => {
	it('moves forward and wraps to the first element', () => {
		expect(nextFocusIndex(3, 0, false)).toBe(1)
		expect(nextFocusIndex(3, 2, false)).toBe(0)
	})

	it('moves backward and wraps to the last element', () => {
		expect(nextFocusIndex(3, 1, true)).toBe(0)
		expect(nextFocusIndex(3, 0, true)).toBe(2)
	})

	it('enters at the first element (Tab) or the last (Shift+Tab) when focus is on the panel', () => {
		expect(nextFocusIndex(3, -1, false)).toBe(0)
		expect(nextFocusIndex(3, -1, true)).toBe(2)
	})

	it('returns -1 when nothing can take focus', () => {
		expect(nextFocusIndex(0, -1, false)).toBe(-1)
	})
})

describe('focusTrap action', () => {
	afterEach(() => {
		document.body.innerHTML = ''
	})

	function setup() {
		const opener = document.createElement('button')
		opener.textContent = 'open'
		document.body.appendChild(opener)
		opener.focus()

		const panel = document.createElement('div')
		panel.innerHTML = '<button>first</button><input /><button disabled>off</button><button>last</button>'
		document.body.appendChild(panel)
		// jsdom has no layout: make every element count as visible.
		for (const el of panel.querySelectorAll<HTMLElement>('*')) {
			el.getClientRects = () => [{}] as unknown as DOMRectList
		}
		return { opener, panel }
	}

	it('focuses the panel itself on open, without choosing a field', () => {
		const { panel } = setup()
		const trap = focusTrap(panel)
		expect(document.activeElement).toBe(panel)
		expect(panel.getAttribute('tabindex')).toBe('-1')
		trap.destroy()
	})

	it('cycles Tab inside the panel, skipping disabled controls', () => {
		const { panel } = setup()
		const trap = focusTrap(panel)
		const [first, , , last] = panel.querySelectorAll<HTMLElement>('button, input')
		const tab = (shiftKey = false) =>
			(document.activeElement as HTMLElement).dispatchEvent(
				new KeyboardEvent('keydown', { key: 'Tab', shiftKey, bubbles: true, cancelable: true })
			)

		tab()
		expect(document.activeElement).toBe(first)
		last.focus()
		tab()
		expect(document.activeElement).toBe(first)
		tab(true)
		expect(document.activeElement).toBe(last)
		trap.destroy()
	})

	it('calls onEscape and gives focus back to the opener when destroyed', () => {
		const { opener, panel } = setup()
		const onEscape = vi.fn()
		const trap = focusTrap(panel, { onEscape })
		panel.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }))
		expect(onEscape).toHaveBeenCalledOnce()
		trap.destroy()
		expect(document.activeElement).toBe(opener)
	})
})
