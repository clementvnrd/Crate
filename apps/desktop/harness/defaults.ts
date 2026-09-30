// Safety net for commands that have no precise handler: an answer of the shape the TypeScript type most likely
// expects, chosen from the command name alone. It keeps a view from crashing on a missing command; it does not
// make the view look right. Every command answered here is reported once (console.warn + `window.__harness.unmocked`)
// so the gap is visible, and the fix is to add a real handler in `handlers/`.

export type DefaultShape = 'list' | 'boolean' | 'number' | 'null'

const GETTER_VERBS = new Set(['get', 'list', 'search', 'suggest', 'take'])

/** A plural noun ("tracks", "devices"), not a word that merely ends in "s" ("status", "progress", "analysis"). */
function isPlural(word: string): boolean {
	return word.length > 3 && word.endsWith('s') && !/(ss|us|is)$/.test(word)
}

/**
 * Which default a command name gets:
 * - predicates (`is_*`, `has_*`, `*_detect_status`, `*_exists`, getters of `*_enabled`) → `false`
 * - `*_count`, `*_size` → `0`
 * - getters (`get_*`, `list_*`, `search_*`, `*_get_*`…) of a plural noun → `[]`, otherwise `null`
 * - everything else (setters, actions) → `null`, which is what a `Promise<void>` command resolves with
 */
export function classifyCommand(command: string): DefaultShape {
	const name = command.replace(/^plugin:[a-z-]+\|/, '')
	const words = name.split('_')
	const verbAt = words.findIndex((word) => GETTER_VERBS.has(word))
	const isGetter = verbAt !== -1 && verbAt <= 2

	if (words.includes('is') || words.includes('has') || words.includes('can')) return 'boolean'
	if (/_(detect_status|exists)$/.test(name)) return 'boolean'
	if (isGetter && words[words.length - 1] === 'enabled') return 'boolean'
	if (['count', 'size'].includes(words[words.length - 1])) return 'number'
	if (isGetter && words.slice(verbAt + 1).some(isPlural)) return 'list'
	return 'null'
}

export function defaultResponse(command: string): unknown {
	switch (classifyCommand(command)) {
		case 'list':
			return []
		case 'boolean':
			return false
		case 'number':
			return 0
		default:
			return null
	}
}
