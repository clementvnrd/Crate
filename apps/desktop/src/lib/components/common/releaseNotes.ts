import { writable } from 'svelte/store'

/**
 * Whether the "What's new in Crate {version}" sheet (`UpdateModal`) is open. It is rendered once, by the update
 * banner in the layout, outside every other dialog; the banner and Settings → About open it through this flag
 * (a dialog nested inside the Settings dialog would see its Tab key handled twice).
 */
export const releaseNotesOpen = writable(false)

export type ReleaseNotesBlock =
	| { kind: 'heading'; text: string }
	| { kind: 'paragraph'; text: string }
	| { kind: 'list'; items: string[] }

/** `**bold**`, `` `code` `` and `[label](url)` read as their plain text: the sheet shows text, not Markdown. */
function plain(text: string): string {
	return text
		.replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
		.replace(/(\*\*|__)(.+?)\1/g, '$2')
		.replace(/`([^`]+)`/g, '$1')
		.trim()
}

/**
 * Split release notes (the Markdown of a GitHub release) into headings, paragraphs and bullet lists, so the sheet
 * can lay them out without a Markdown renderer. `#` lines are headings, `-`, `*` and `+` lines are list items
 * (a following indented line continues the item), blank lines separate paragraphs.
 */
export function parseReleaseNotes(body: string | null | undefined): ReleaseNotesBlock[] {
	const blocks: ReleaseNotesBlock[] = []
	let paragraph: string[] = []
	let list: string[] | null = null

	const flush = () => {
		if (paragraph.length > 0) blocks.push({ kind: 'paragraph', text: plain(paragraph.join(' ')) })
		if (list && list.length > 0) blocks.push({ kind: 'list', items: list })
		paragraph = []
		list = null
	}

	for (const raw of (body ?? '').split(/\r?\n/)) {
		const line = raw.trim()
		if (line === '') {
			flush()
			continue
		}
		const heading = line.match(/^#{1,6}\s+(.*)$/)
		if (heading) {
			flush()
			blocks.push({ kind: 'heading', text: plain(heading[1]) })
			continue
		}
		const item = line.match(/^[-*+]\s+(.*)$/)
		if (item) {
			if (paragraph.length > 0) flush()
			list ??= []
			list.push(plain(item[1]))
			continue
		}
		if (list && /^\s/.test(raw)) {
			list[list.length - 1] = plain(`${list[list.length - 1]} ${line}`)
			continue
		}
		if (list) flush()
		paragraph.push(line)
	}
	flush()
	return blocks
}
