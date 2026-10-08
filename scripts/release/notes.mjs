#!/usr/bin/env node
/**
 * Print the release-page body of a version: its CHANGELOG.md section, capped under GitHub's limit,
 * followed by the source commit. Used by the release workflow.
 *
 * Usage (from the repository root): node scripts/release/notes.mjs <version> [commit]
 */

import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { notesFor } from '../changelog.js'
import { releaseBody } from './manifest.mjs'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..')
const [version, commitArg] = process.argv.slice(2)
if (!version) {
	console.error('Usage: node scripts/release/notes.mjs <version> [commit]')
	process.exit(1)
}

try {
	const commit = commitArg ?? execFileSync('git', ['rev-parse', 'HEAD'], { cwd: ROOT, encoding: 'utf-8' }).trim()
	process.stdout.write(releaseBody(notesFor(readFileSync(join(ROOT, 'CHANGELOG.md'), 'utf-8'), version), commit))
} catch (error) {
	console.error(`Error: ${error.message}`)
	process.exit(1)
}
