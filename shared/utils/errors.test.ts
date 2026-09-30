import { afterEach, describe, expect, it } from 'vitest'
import { get } from 'svelte/store'
import { locale, translate } from '../i18n'
import { localizeBackendError, toErrorMessage } from './errors'

afterEach(() => {
	locale.set('en')
})

describe('toErrorMessage', () => {
	it('keeps the string a Tauri command rejects with', () => {
		expect(toErrorMessage('Disk full: /Volumes/USB', 'fallback')).toBe('Disk full: /Volumes/USB')
	})
	it('reads Error and message-like objects', () => {
		expect(toErrorMessage(new Error('boom'), 'fallback')).toBe('boom')
		expect(toErrorMessage({ message: 'from object' }, 'fallback')).toBe('from object')
	})
	it('falls back for anything else', () => {
		expect(toErrorMessage(undefined, 'fallback')).toBe('fallback')
		expect(toErrorMessage('', 'fallback')).toBe('fallback')
	})
	it('translates a known backend message wherever it comes from', () => {
		locale.set('fr')
		const expected = get(translate)('errors.beatportAuthRequired')
		expect(toErrorMessage('Beatport authentication required. Please sign in from the Beatport tab.', 'x')).toBe(
			expected
		)
		expect(toErrorMessage(new Error('Beatport authentication required'), 'x')).toBe(expected)
		expect(toErrorMessage({ message: 'Beatport authentication required' }, 'x')).toBe(expected)
	})
})

describe('localizeBackendError', () => {
	const samples: [string, string][] = [
		['Beatport authentication required. No access token available.', 'errors.beatportAuthRequired'],
		[
			'Beatport authentication required (session expired or unauthorised). Please sign in again from the Beatport tab.',
			'errors.beatportAuthRequired',
		],
		['Beatport token expired or invalid', 'errors.beatportTokenInvalid'],
		['Artist not found on Beatport', 'errors.beatportArtistNotFound'],
		['The specified path is not a valid folder: /tmp/x', 'errors.albumInvalidFolder'],
		['No supported audio file found in this folder.', 'errors.albumNoAudio'],
	]

	it.each(samples)('maps %j to %s in French and English', (message, key) => {
		locale.set('fr')
		const french = localizeBackendError(message)
		expect(french).toBe(get(translate)(key))
		expect(french).not.toBe(key)
		expect(french).not.toBe(message)

		locale.set('en')
		expect(localizeBackendError(message)).toBe(get(translate)(key))
	})

	it('leaves an unknown message untouched', () => {
		expect(localizeBackendError('Network error (account): timeout')).toBe('Network error (account): timeout')
	})
})
