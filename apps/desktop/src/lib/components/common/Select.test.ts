import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/svelte'
import Select from './Select.svelte'

// CRA-196: the global shortcuts listen on `window`, where Up and Down change the volume. The arrows that open the
// select and move through its options stay in the control.
describe('Select, keys stay in the control', () => {
	const reachedWindow: string[] = []
	const onWindowKeydown = (event: KeyboardEvent) => reachedWindow.push(event.key)
	const options = ['dark', 'light', 'system'].map((value) => ({ value, label: value }))

	// jsdom has no Web Animations: the menu's `scale` transition gets an animation that finishes at once.
	const originalAnimate = Object.getOwnPropertyDescriptor(Element.prototype, 'animate')

	beforeEach(() => {
		reachedWindow.length = 0
		window.addEventListener('keydown', onWindowKeydown)
		Object.defineProperty(Element.prototype, 'animate', {
			configurable: true,
			writable: true,
			value: () => {
				const animation = { onfinish: null as null | (() => void), cancel: () => {}, currentTime: 0 }
				queueMicrotask(() => animation.onfinish?.())
				return animation
			},
		})
	})

	afterEach(() => {
		window.removeEventListener('keydown', onWindowKeydown)
		if (originalAnimate) Object.defineProperty(Element.prototype, 'animate', originalAnimate)
		else delete (Element.prototype as unknown as Record<string, unknown>).animate
	})

	it('opens with ArrowDown and moves through the options without changing the volume', async () => {
		const onchange = vi.fn()
		render(Select, { props: { value: 'dark', options, onchange } })
		const trigger = screen.getByRole('button')
		trigger.focus()
		await fireEvent.keyDown(trigger, { key: 'ArrowDown' }) // opens
		expect(trigger.getAttribute('aria-expanded')).toBe('true')
		await fireEvent.keyDown(trigger, { key: 'ArrowDown' })
		await fireEvent.keyDown(trigger, { key: 'ArrowUp' })
		await fireEvent.keyDown(trigger, { key: 'ArrowDown' })
		await fireEvent.keyDown(trigger, { key: 'Enter' })
		expect(onchange).toHaveBeenCalledWith('light')
		expect(reachedWindow.filter((key) => key.startsWith('Arrow'))).toEqual([])
	})

	it('leaves modified arrows to the global shortcuts (Cmd+↓ selects the next track)', async () => {
		const onchange = vi.fn()
		render(Select, { props: { value: 'dark', options, onchange } })
		const trigger = screen.getByRole('button')
		trigger.focus()
		await fireEvent.keyDown(trigger, { key: 'ArrowDown', metaKey: true })
		expect(trigger.getAttribute('aria-expanded')).toBe('false')
		await fireEvent.keyDown(trigger, { key: 'ArrowDown' }) // opens
		await fireEvent.keyDown(trigger, { key: 'ArrowUp', ctrlKey: true })
		await fireEvent.keyDown(trigger, { key: 'ArrowDown', shiftKey: true })
		expect(reachedWindow).toEqual(['ArrowDown', 'ArrowUp', 'ArrowDown'])
		expect(onchange).not.toHaveBeenCalled()
	})

	it('leaves ArrowUp to the global shortcuts while closed', async () => {
		render(Select, { props: { value: 'dark', options } })
		const trigger = screen.getByRole('button')
		trigger.focus()
		await fireEvent.keyDown(trigger, { key: 'ArrowUp' })
		expect(reachedWindow).toEqual(['ArrowUp'])
	})
})
