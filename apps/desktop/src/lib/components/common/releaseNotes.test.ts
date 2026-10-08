import { describe, expect, it } from 'vitest'
import { parseReleaseNotes } from './releaseNotes'

describe('parseReleaseNotes', () => {
	it('returns nothing for empty notes', () => {
		expect(parseReleaseNotes(null)).toEqual([])
		expect(parseReleaseNotes('  \n\n ')).toEqual([])
	})

	it('reads headings, paragraphs and bullet lists', () => {
		const notes = [
			'## Polish & Unify 0.4',
			'',
			'A calmer update experience.',
			'It never interrupts playback.',
			'',
			'- First change',
			'* Second change',
			'  continued on the next line',
			'',
			'### Fixes',
			'+ A fix',
		].join('\n')

		expect(parseReleaseNotes(notes)).toEqual([
			{ kind: 'heading', text: 'Polish & Unify 0.4' },
			{ kind: 'paragraph', text: 'A calmer update experience. It never interrupts playback.' },
			{ kind: 'list', items: ['First change', 'Second change continued on the next line'] },
			{ kind: 'heading', text: 'Fixes' },
			{ kind: 'list', items: ['A fix'] },
		])
	})

	it('shows inline Markdown as plain text', () => {
		expect(parseReleaseNotes('- **Bold** `code` and [a link](https://example.com)')).toEqual([
			{ kind: 'list', items: ['Bold code and a link'] },
		])
	})

	it('keeps a list and a paragraph that follow each other apart', () => {
		expect(parseReleaseNotes('Intro line\n- item\nAfter the list')).toEqual([
			{ kind: 'paragraph', text: 'Intro line' },
			{ kind: 'list', items: ['item'] },
			{ kind: 'paragraph', text: 'After the list' },
		])
	})
})
