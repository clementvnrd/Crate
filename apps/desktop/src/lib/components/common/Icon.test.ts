import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

const SRC = join(__dirname, '../../..')
const iconSource = readFileSync(join(__dirname, 'Icon.svelte'), 'utf8')

function svelteFiles(dir: string): string[] {
	return readdirSync(dir).flatMap((entry) => {
		const path = join(dir, entry)
		if (statSync(path).isDirectory()) return svelteFiles(path)
		return path.endsWith('.svelte') ? [path] : []
	})
}

/** Names defined as keys of the icon maps, plus the special-cased brand icon. */
const defined = new Set(['beatport', ...[...iconSource.matchAll(/^\s*'?([a-z0-9-]+)'?\s*:/gm)].map((m) => m[1])])

describe('Icon', () => {
	it('defines every icon name used with a literal <Icon name="…">', () => {
		const missing: string[] = []
		for (const file of svelteFiles(SRC)) {
			for (const match of readFileSync(file, 'utf8').matchAll(/<Icon\s+name="([a-z0-9-]+)"/g)) {
				if (!defined.has(match[1])) missing.push(`${match[1]} (${file.slice(SRC.length + 1)})`)
			}
		}
		expect(missing).toEqual([])
	})
})
