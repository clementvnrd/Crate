import { describe, expect, it } from 'vitest'
import { get } from 'svelte/store'
import { locale, translate } from './index'

describe('i18n initialisation', () => {
	it('has a locale and messages the moment the module is imported (no async loader round-trip)', () => {
		// addMessages() + init() at module load must be enough: a component rendering on the very
		// first tick would otherwise throw "Cannot format a message without first setting the
		// initial locale".
		expect(get(locale)).toBe('en')
		expect(get(translate)('common.cancel')).not.toBe('common.cancel')
	})

	it('switches to French synchronously, without waiting for a loader', () => {
		locale.set('fr')
		expect(get(locale)).toBe('fr')
		expect(get(translate)('common.cancel')).not.toBe('common.cancel')
		locale.set('en')
	})
})
