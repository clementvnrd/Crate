import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

// The frontend and the Rust side only meet through command names written as strings. A typo, or a
// command removed from the Rust handler list, fails at runtime with "command not found" and the
// type checker cannot see it. This test reads both sides and compares them.

const root = join(__dirname, '..', '..')
const rust = readFileSync(join(root, 'src-tauri', 'src', 'lib.rs'), 'utf8')
const registered = new Set([...rust.matchAll(/commands::\w+::(\w+)/g)].map((m) => m[1]))

// `invoke('name'` and `invoke<Type>('name'`, where the type may contain nested generics or an
// `import('…')` type.
const INVOKE = /\binvoke(?:<(?:[^<>()]|<[^<>]*>|\([^)]*\))*>)?\(\s*'([a-z][a-z0-9_]*)'/g

const apiDir = join(__dirname)
const invoked = new Map<string, string>()
for (const file of readdirSync(apiDir).filter((f) => f.endsWith('.ts') && !f.endsWith('.test.ts'))) {
	const source = readFileSync(join(apiDir, file), 'utf8')
	for (const match of source.matchAll(INVOKE)) invoked.set(match[1], file)
}

describe('the commands the frontend invokes', () => {
	it('finds both sides (so that an empty match cannot make the test pass)', () => {
		expect(registered.size).toBeGreaterThan(100)
		expect(invoked.size).toBeGreaterThan(100)
	})

	it('are all registered in the Rust handler list', () => {
		const unknown = [...invoked].filter(([name]) => !registered.has(name)).map(([name, file]) => `${name} (${file})`)
		expect(unknown).toEqual([])
	})
})
