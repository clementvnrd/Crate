#!/usr/bin/env node
/**
 * Changelog management script for Crate
 *
 * Usage:
 *   node scripts/changelog.js prepare <version>   # Turn [Unreleased] into a version section
 *   node scripts/changelog.js graduate <version>  # Consolidate prereleases to stable
 *   node scripts/changelog.js notes <version>     # Print the release notes of a version
 *
 * Every operation is text-preserving: section bodies are moved verbatim, never re-parsed. The
 * fork keeps its change log as free-form `###`/`####` headings and paragraphs (see the
 * "Personal fork — change log" section), which a Keep-a-Changelog category parser would drop.
 */

import { readFileSync, writeFileSync } from 'fs'
import { join, dirname } from 'path'
import { fileURLToPath, pathToFileURL } from 'url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const ROOT = join(__dirname, '..')
const CHANGELOG_PATH = join(ROOT, 'CHANGELOG.md')

/** Links in the footer point to the fork, not to upstream. */
export const REPO_URL = 'https://github.com/clementvnrd/Crate'

const VERSION_HEADER = /^## \[([^\]]+)\](?:\s*-\s*(\d{4}-\d{2}-\d{2}))?\s*$/
const FOOTER_LINK = /^\[[^\]]+\]:\s*\S+/

function getToday() {
	return new Date().toISOString().split('T')[0]
}

export function getBaseVersion(version) {
	return version.replace(/-.*$/, '')
}

/**
 * Split the changelog into a header, version sections and a footer of reference links.
 * Each section keeps its raw lines; `body` is everything after the `## [x]` line.
 */
export function splitChangelog(text) {
	const lines = text.replace(/\r\n/g, '\n').split('\n')

	// The footer is the trailing block of `[x]: url` reference links (and blank lines).
	let footerStart = lines.length
	for (let i = lines.length - 1; i >= 0; i--) {
		if (lines[i].trim() === '' || FOOTER_LINK.test(lines[i])) {
			if (FOOTER_LINK.test(lines[i])) footerStart = i
			continue
		}
		break
	}

	const header = []
	const sections = []
	let current = null
	for (let i = 0; i < footerStart; i++) {
		const match = lines[i].match(VERSION_HEADER)
		if (match) {
			current = { version: match[1], date: match[2] ?? null, headerLine: lines[i], body: [] }
			sections.push(current)
		} else if (current) {
			current.body.push(lines[i])
		} else {
			header.push(lines[i])
		}
	}

	const footer = lines.slice(footerStart).filter((line) => line.trim() !== '')
	return { header, sections, footer }
}

function trimBlankEdges(lines) {
	let start = 0
	let end = lines.length
	while (start < end && lines[start].trim() === '') start++
	while (end > start && lines[end - 1].trim() === '') end--
	return lines.slice(start, end)
}

/** True when a section body holds more than headings and blank lines. */
export function hasContent(body) {
	return body.some((line) => line.trim() !== '' && !/^#{3,6}\s/.test(line))
}

export function joinChangelog({ header, sections, footer }) {
	const parts = [trimBlankEdges(header).join('\n')]
	for (const section of sections) {
		const body = trimBlankEdges(section.body)
		parts.push(body.length > 0 ? `${section.headerLine}\n\n${body.join('\n')}` : section.headerLine)
	}
	if (footer.length > 0) parts.push(footer.join('\n'))
	return parts.join('\n\n') + '\n'
}

function updateFooterLinks(footer, version, removePattern = null, previousVersion = undefined) {
	let links = removePattern
		? footer.filter((line) => !removePattern.test(line.match(/^\[([^\]]+)\]/)?.[1] ?? ''))
		: [...footer]

	const unreleasedIndex = links.findIndex((line) => line.startsWith('[Unreleased]:'))
	// `graduate` passes the previous stable version: the [Unreleased] link still names the last prerelease.
	const fromUnreleased =
		unreleasedIndex >= 0 ? (links[unreleasedIndex].match(/compare\/v(.+?)\.\.\.HEAD/)?.[1] ?? null) : null
	const previous = previousVersion !== undefined ? previousVersion : fromUnreleased

	const unreleased = `[Unreleased]: ${REPO_URL}/compare/v${version}...HEAD`
	const versionLink = previous
		? `[${version}]: ${REPO_URL}/compare/v${previous}...v${version}`
		: `[${version}]: ${REPO_URL}/releases/tag/v${version}`

	if (unreleasedIndex >= 0) {
		links.splice(unreleasedIndex, 1, unreleased, versionLink)
	} else {
		links = [unreleased, versionLink, ...links]
	}
	return links
}

/** Move the whole [Unreleased] body, verbatim, under a new `## [version] - date` heading. */
export function prepare(text, version, today = getToday()) {
	const doc = splitChangelog(text)
	const unreleased = doc.sections.find((s) => s.version === 'Unreleased')
	if (!unreleased) throw new Error('No [Unreleased] section found in CHANGELOG.md')
	if (doc.sections.some((s) => s.version === version)) {
		throw new Error(`CHANGELOG.md already has a [${version}] section`)
	}
	if (!hasContent(unreleased.body)) throw new Error('No changes in [Unreleased] section to release')

	const released = { version, date: today, headerLine: `## [${version}] - ${today}`, body: unreleased.body }
	unreleased.body = []
	doc.sections.splice(doc.sections.indexOf(unreleased) + 1, 0, released)
	doc.footer = updateFooterLinks(doc.footer, version)
	return joinChangelog(doc)
}

/** Replace every `<base>-*` prerelease section by one stable section holding their bodies, newest first. */
export function graduate(text, stableVersion, today = getToday()) {
	const doc = splitChangelog(text)
	const base = getBaseVersion(stableVersion)
	const prefix = new RegExp(`^${base.replace(/\./g, '\\.')}-`)
	const prereleases = doc.sections.filter((s) => prefix.test(s.version))
	if (prereleases.length === 0) throw new Error(`No prerelease sections found for ${base}`)
	if (doc.sections.some((s) => s.version === stableVersion)) {
		throw new Error(`CHANGELOG.md already has a [${stableVersion}] section`)
	}

	const body = []
	for (const section of prereleases) {
		const content = trimBlankEdges(section.body)
		if (content.length === 0) continue
		if (body.length > 0) body.push('')
		body.push(...content)
	}

	doc.sections = doc.sections.filter((s) => !prefix.test(s.version))
	const unreleasedIndex = doc.sections.findIndex((s) => s.version === 'Unreleased')
	const previousStable = doc.sections.find((s) => s.version !== 'Unreleased')?.version ?? null
	doc.sections.splice(unreleasedIndex + 1, 0, {
		version: stableVersion,
		date: today,
		headerLine: `## [${stableVersion}] - ${today}`,
		body,
	})
	doc.footer = updateFooterLinks(doc.footer, stableVersion, prefix, previousStable)
	return joinChangelog(doc)
}

/**
 * Release notes of a version: its own section, or — for a `-staging.N` rebuild that has no section
 * of its own — the most recent staging section of the same base version.
 */
export function notesFor(text, version) {
	const { sections } = splitChangelog(text)
	let section = sections.find((s) => s.version === version)
	if (!section && /-staging\.\d+$/.test(version)) {
		const prefix = `${getBaseVersion(version)}-staging.`
		section = sections.find((s) => s.version.startsWith(prefix))
	}
	if (!section) throw new Error(`Version ${version} not found in CHANGELOG.md`)
	return trimBlankEdges(section.body).join('\n')
}

function printUsage() {
	console.error(`
Usage:
  yarn changelog:prepare <version>   # Move the [Unreleased] content under a version heading
  yarn changelog:graduate <version>  # Consolidate prereleases to stable version
  node scripts/changelog.js notes <version>  # Print the release notes of a version

Examples:
  yarn changelog:prepare 1.0.0-staging.1   # Create staging release entry
  yarn changelog:graduate 1.0.0            # Consolidate all 1.0.0-staging.* entries to 1.0.0
`)
}

function main(argv) {
	const [command, version] = argv
	if (!command || !version) {
		printUsage()
		process.exit(1)
	}

	const text = readFileSync(CHANGELOG_PATH, 'utf-8')
	switch (command) {
		case 'prepare':
			writeFileSync(CHANGELOG_PATH, prepare(text, version))
			console.log(`Prepared changelog for version ${version}`)
			break
		case 'graduate':
			writeFileSync(CHANGELOG_PATH, graduate(text, version))
			console.log(`Graduated prereleases of ${getBaseVersion(version)} to ${version}`)
			break
		case 'notes':
			process.stdout.write(notesFor(text, version) + '\n')
			break
		default:
			console.error(`Unknown command: ${command}`)
			printUsage()
			process.exit(1)
	}
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	try {
		main(process.argv.slice(2))
	} catch (error) {
		console.error(`Error: ${error.message}`)
		process.exit(1)
	}
}
