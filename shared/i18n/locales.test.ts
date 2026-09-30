import { describe, expect, it } from 'vitest'
import { get } from 'svelte/store'
import en from './locales/en.json'
import fr from './locales/fr.json'
import { locale, translate, waitLocale } from './index'

type Tree = { [key: string]: string | Tree }

function flatten(tree: Tree, prefix = ''): Record<string, string> {
	const out: Record<string, string> = {}
	for (const [key, value] of Object.entries(tree)) {
		if (typeof value === 'string') out[`${prefix}${key}`] = value
		else Object.assign(out, flatten(value, `${prefix}${key}.`))
	}
	return out
}

/** `{name}` and `{name, plural, …}` arguments used by a message, ignoring the plural branches. */
function argumentsOf(message: string): string[] {
	const found = new Set<string>()
	let depth = 0
	let token = ''
	for (const char of message) {
		if (char === '{') {
			depth++
			token = ''
		} else if (char === '}') {
			if (depth === 1 && token) found.add(token.split(',')[0].trim())
			depth = Math.max(0, depth - 1)
			token = ''
		} else if (depth === 1) {
			token += char
		}
	}
	return [...found].sort()
}

const english = flatten(en as Tree)
const french = flatten(fr as Tree)

describe('English and French stay complete (owner decision CRA-114)', () => {
	it('French has every key English has', () => {
		const missing = Object.keys(english).filter((key) => !(key in french))
		expect(missing).toEqual([])
	})

	it('French has no key English lacks (English is the source of truth)', () => {
		const orphans = Object.keys(french).filter((key) => !(key in english))
		expect(orphans).toEqual([])
	})

	it('French messages use the same {arguments} as the English ones', () => {
		const mismatched = Object.keys(english)
			.filter((key) => key in french)
			.filter((key) => argumentsOf(english[key]).join() !== argumentsOf(french[key]).join())
		expect(mismatched).toEqual([])
	})
})

describe('the other languages fall back to English', () => {
	it('shows the English text, never the raw key, for a key a language lacks', async () => {
		// `de` is one of the 13 lazy-loaded locales that lack the keys added by the fork.
		const german = flatten((await import('./locales/de.json')).default as Tree)
		const gap = Object.keys(english).find((key) => !(key in german))
		expect(gap, 'de is expected to lack at least one recent key').toBeDefined()

		await locale.set('de')
		await waitLocale('de')
		const shown = get(translate)(gap as string)
		await locale.set('en')

		expect(shown).not.toBe(gap)
		expect(shown).toBe(get(translate)(gap as string))
	})
})
