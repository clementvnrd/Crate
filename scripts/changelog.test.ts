import { readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { graduate, joinChangelog, notesFor, prepare, splitChangelog } from './changelog.js'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')

const SAMPLE = `# Changelog

Intro line.

## [Unreleased]

### Personal fork — change log

> A free-form note.

#### Process

- **2026-10-08 — Something.** A paragraph that is not a Keep-a-Changelog category.

### Upstream (blackboxaudio)

### Added

- Upstream item

## [0.2.9] - 2026-06-09

### Fixed

- Old fix

[Unreleased]: https://github.com/blackboxaudio/crate/compare/v0.2.9...HEAD
[0.2.9]: https://github.com/blackboxaudio/crate/compare/v0.2.8...v0.2.9
`

describe('changelog script', () => {
	it('round-trips the real CHANGELOG.md without changing a line of content', () => {
		const text = readFileSync(path.join(root, 'CHANGELOG.md'), 'utf-8')
		const squash = (s: string) => s.replace(/\n{2,}/g, '\n\n').trim()
		expect(squash(joinChangelog(splitChangelog(text)))).toBe(squash(text))
	})

	it('prepare moves the whole [Unreleased] body verbatim, free-form headings included', () => {
		const out = prepare(SAMPLE, '0.3.0-staging.1', '2026-10-08')
		expect(out).toContain('## [Unreleased]\n\n## [0.3.0-staging.1] - 2026-10-08\n\n### Personal fork — change log')
		expect(out).toContain('#### Process')
		expect(out).toContain('> A free-form note.')
		expect(out).toContain('- Upstream item')
		expect(out).toContain('[Unreleased]: https://github.com/clementvnrd/Crate/compare/v0.3.0-staging.1...HEAD')
		expect(out).toContain('[0.3.0-staging.1]: https://github.com/clementvnrd/Crate/compare/v0.2.9...v0.3.0-staging.1')
		expect(out).toContain('[0.2.9]: https://github.com/blackboxaudio/crate/compare/v0.2.8...v0.2.9')
	})

	it('prepare refuses an empty [Unreleased] and an existing version', () => {
		const once = prepare(SAMPLE, '0.3.0-staging.1', '2026-10-08')
		expect(() => prepare(once, '0.3.0-staging.2', '2026-10-09')).toThrow(/No changes/)
		expect(() => prepare(SAMPLE, '0.2.9', '2026-10-09')).toThrow(/already has a \[0\.2\.9\]/)
	})

	it('graduate merges every prerelease body, newest first, into the stable section', () => {
		let text = prepare(SAMPLE, '0.3.0-staging.1', '2026-10-08')
		text = text.replace('## [Unreleased]\n', '## [Unreleased]\n\n#### Fixed — later\n\n- A staging.2 fix\n')
		text = prepare(text, '0.3.0-staging.2', '2026-10-09')
		const out = graduate(text, '0.3.0', '2026-10-10')

		expect(out).not.toMatch(/## \[0\.3\.0-staging/)
		const stable = out.slice(out.indexOf('## [0.3.0] - 2026-10-10'), out.indexOf('## [0.2.9]'))
		expect(stable.indexOf('- A staging.2 fix')).toBeLessThan(stable.indexOf('#### Process'))
		expect(out).toContain('[0.3.0]: https://github.com/clementvnrd/Crate/compare/v0.2.9...v0.3.0')
		expect(out).not.toContain('[0.3.0-staging.1]:')
	})

	it('notesFor returns a section body and falls back to the latest staging section of the same base', () => {
		const text = prepare(SAMPLE, '0.3.0-staging.1', '2026-10-08')
		expect(notesFor(text, '0.2.9')).toBe('### Fixed\n\n- Old fix')
		expect(notesFor(text, '0.3.0-staging.3')).toContain('#### Process')
		expect(() => notesFor(text, '0.4.0')).toThrow(/not found/)
	})
})
