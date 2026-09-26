import { describe, expect, it } from 'vitest'
import { toErrorMessage } from './errors'

describe('toErrorMessage', () => {
	it('keeps the string a Tauri command rejects with', () => {
		expect(toErrorMessage('Beatport authentication required', 'fallback')).toBe('Beatport authentication required')
	})
	it('reads Error and message-like objects', () => {
		expect(toErrorMessage(new Error('boom'), 'fallback')).toBe('boom')
		expect(toErrorMessage({ message: 'from object' }, 'fallback')).toBe('from object')
	})
	it('falls back for anything else', () => {
		expect(toErrorMessage(undefined, 'fallback')).toBe('fallback')
		expect(toErrorMessage('', 'fallback')).toBe('fallback')
	})
})
