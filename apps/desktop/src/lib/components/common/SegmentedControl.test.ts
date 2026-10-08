import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/svelte'
import SegmentedControl from './SegmentedControl.svelte'

// jsdom has no layout: the scroller and its segments get fixed metrics. Five 60 px segments after 4 px of padding
// in a 150 px wide scroller: 4 + 5 × 60 + 4 = 308 px of content, so up to 158 px can scroll.
const SEGMENT = 60
const PADDING = 4
const options = ['today', '7d', '30d', 'year', 'all'].map((value) => ({ value, label: value }))

let layout = { clientWidth: 150, scrollWidth: PADDING * 2 + SEGMENT * options.length }
let scrollPosition = 0
const scrollTo = vi.fn((target: ScrollToOptions) => {
	scrollPosition = target.left ?? scrollPosition
})

const isScroller = (el: Element) => el.getAttribute('role') === 'radiogroup'
const indexOf = (el: Element) => (el.parentElement ? [...el.parentElement.children].indexOf(el) : 0)

const stubs: Record<string, (this: HTMLElement) => number> = {
	clientWidth() {
		return isScroller(this) ? layout.clientWidth : 0
	},
	scrollWidth() {
		return isScroller(this) ? layout.scrollWidth : 0
	},
	scrollLeft() {
		return isScroller(this) ? scrollPosition : 0
	},
	offsetLeft() {
		return this.getAttribute('role') === 'radio' ? PADDING + indexOf(this) * SEGMENT : 0
	},
	offsetWidth() {
		return this.getAttribute('role') === 'radio' ? SEGMENT : 0
	},
}
const originals = new Map<string, PropertyDescriptor | undefined>()

beforeEach(() => {
	layout = { clientWidth: 150, scrollWidth: PADDING * 2 + SEGMENT * options.length }
	scrollPosition = 0
	scrollTo.mockClear()
	for (const [name, get] of Object.entries(stubs)) {
		originals.set(name, Object.getOwnPropertyDescriptor(HTMLElement.prototype, name))
		Object.defineProperty(HTMLElement.prototype, name, { configurable: true, get })
	}
	originals.set('scrollTo', Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'scrollTo'))
	Object.defineProperty(HTMLElement.prototype, 'scrollTo', { configurable: true, writable: true, value: scrollTo })
})

afterEach(() => {
	for (const [name, descriptor] of originals) {
		if (descriptor) Object.defineProperty(HTMLElement.prototype, name, descriptor)
		else delete (HTMLElement.prototype as unknown as Record<string, unknown>)[name]
	}
	originals.clear()
})

function renderControl(props: { value: string; scrollable?: boolean; onchange?: (value: string) => void }) {
	return render(SegmentedControl<string>, {
		props: {
			options,
			value: props.value,
			onchange: props.onchange ?? (() => {}),
			ariaLabel: 'Period',
			variant: 'boxed',
			scrollable: props.scrollable ?? true,
		},
	})
}

const previousArrow = () => screen.queryByRole('button', { name: 'Show previous options' })
const nextArrow = () => screen.queryByRole('button', { name: 'Show more options' })
const scroller = () => screen.getByRole('radiogroup', { name: 'Period' })

describe('SegmentedControl, scrollable', () => {
	it('is unchanged without the prop: no scroller, no arrows', () => {
		renderControl({ value: 'today', scrollable: false })
		expect(scroller().classList.contains('scroll-affordance')).toBe(false)
		expect(screen.getAllByRole('radio')).toHaveLength(5)
		expect(previousArrow()).toBeNull()
		expect(nextArrow()).toBeNull()
	})

	it('shows no arrow and no fade when every option fits', () => {
		layout.clientWidth = layout.scrollWidth
		renderControl({ value: 'today' })
		expect(scroller().classList.contains('scroll-affordance')).toBe(true)
		expect(scroller().hasAttribute('data-fade-start')).toBe(false)
		expect(scroller().hasAttribute('data-fade-end')).toBe(false)
		expect(previousArrow()).toBeNull()
		expect(nextArrow()).toBeNull()
		expect(scrollTo).not.toHaveBeenCalled()
	})

	it('shows only the next arrow and the end fade when options are hidden at the end', () => {
		renderControl({ value: 'today' })
		expect(nextArrow()).not.toBeNull()
		expect(previousArrow()).toBeNull()
		expect(scroller().hasAttribute('data-fade-end')).toBe(true)
		expect(scroller().hasAttribute('data-fade-start')).toBe(false)
		// The first option is already in view: nothing scrolls.
		expect(scrollTo).not.toHaveBeenCalled()
	})

	it('scrolls the selected option into view on mount, at once', () => {
		renderControl({ value: 'all' })
		expect(scrollTo).toHaveBeenCalledWith({ left: 158, behavior: 'auto' })
	})

	it('swaps the arrows when scrolled to the end', async () => {
		renderControl({ value: 'all' })
		await fireEvent.scroll(scroller())
		expect(previousArrow()).not.toBeNull()
		expect(nextArrow()).toBeNull()
		expect(scroller().hasAttribute('data-fade-start')).toBe(true)
		expect(scroller().hasAttribute('data-fade-end')).toBe(false)
	})

	it('pages forward with the next arrow and then shows both arrows', async () => {
		renderControl({ value: 'today' })
		await fireEvent.click(nextArrow()!)
		// One view (150) minus the two 44 px edges = 62, but never less than half a view (75).
		expect(scrollTo).toHaveBeenLastCalledWith({ left: 75, behavior: 'smooth' })
		await fireEvent.scroll(scroller())
		expect(previousArrow()).not.toBeNull()
		expect(nextArrow()).not.toBeNull()
	})

	it('keeps the arrows out of the tab order', () => {
		renderControl({ value: 'today' })
		expect(nextArrow()?.getAttribute('tabindex')).toBe('-1')
	})

	it('moves the selection with the arrow keys and scrolls the new selection into view', async () => {
		const onchange = vi.fn()
		const { rerender } = renderControl({ value: 'today', onchange })
		await fireEvent.keyDown(screen.getByRole('radio', { name: 'today' }), { key: 'End' })
		expect(onchange).toHaveBeenCalledWith('all')
		await rerender({ value: 'all' })
		expect(scrollTo).toHaveBeenLastCalledWith({ left: 158, behavior: 'smooth' })
		const selected = screen.getByRole('radio', { name: 'all' })
		expect(selected.getAttribute('aria-checked')).toBe('true')
		expect(selected.getAttribute('tabindex')).toBe('0')
	})
})

// CRA-196: the global shortcuts listen on `window` (arrows seek and change the volume). A focused control keeps the
// keys it handles: they move the selection and never reach `window`; any other key still does.
describe('SegmentedControl, keys stay in the control', () => {
	const reachedWindow: string[] = []
	const onWindowKeydown = (event: KeyboardEvent) => reachedWindow.push(event.key)

	beforeEach(() => {
		reachedWindow.length = 0
		window.addEventListener('keydown', onWindowKeydown)
	})

	afterEach(() => {
		window.removeEventListener('keydown', onWindowKeydown)
	})

	for (const scrollable of [false, true]) {
		it(`arrows, Home and End change the selection only${scrollable ? ' (scrollable)' : ''}`, async () => {
			const onchange = vi.fn()
			renderControl({ value: '7d', onchange, scrollable })
			const radio = screen.getByRole('radio', { name: '7d' })
			radio.focus()
			for (const key of ['ArrowRight', 'ArrowLeft', 'ArrowUp', 'ArrowDown', 'Home', 'End']) {
				await fireEvent.keyDown(radio, { key })
			}
			expect(onchange.mock.calls.map(([value]) => value)).toEqual(['30d', 'today', 'today', '30d', 'today', 'all'])
			expect(reachedWindow).toEqual([])
		})
	}

	it('lets the keys it does not handle, and modified arrows, through to the global shortcuts', async () => {
		const onchange = vi.fn()
		renderControl({ value: '7d', onchange, scrollable: false })
		const radio = screen.getByRole('radio', { name: '7d' })
		radio.focus()
		await fireEvent.keyDown(radio, { key: 'm' })
		await fireEvent.keyDown(radio, { key: 'ArrowRight', shiftKey: true }) // next track
		await fireEvent.keyDown(radio, { key: 'ArrowLeft', metaKey: true }) // fine seek
		expect(reachedWindow).toEqual(['m', 'ArrowRight', 'ArrowLeft'])
		expect(onchange).not.toHaveBeenCalled()
	})
})
