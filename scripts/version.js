#!/usr/bin/env node
/**
 * Version bump script for Crate
 * Synchronizes version across package.json, Cargo.toml, Cargo.lock (the app's own entry), tauri.conf.json,
 * tauri.staging.conf.json and tauri.dev.conf.json
 *
 * Production tauri.conf.json gets base version only (no prerelease).
 * Staging tauri.staging.conf.json gets the full semver version.
 *
 * Usage:
 *   node scripts/version.js <major|minor|patch>              # Standard version bump
 *   node scripts/version.js <major|minor|patch> staging       # Start staging prerelease
 *   node scripts/version.js prerelease                       # Increment prerelease number
 *   node scripts/version.js stage                            # Promote staging to stable
 *   node scripts/version.js --print <bump_type> [channel]    # Print new version without writing
 */

import { readFileSync, writeFileSync } from 'fs'
import { join, dirname } from 'path'
import { fileURLToPath } from 'url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const ROOT = join(__dirname, '..')

const STANDARD_BUMPS = ['major', 'minor', 'patch']
const PRERELEASE_CHANNELS = ['staging']

function parseVersion(version) {
	const match = version.match(/^(\d+)\.(\d+)\.(\d+)(?:-([a-zA-Z]+)\.(\d+))?$/)
	if (!match) {
		throw new Error(`Invalid version format: ${version}`)
	}
	return {
		major: parseInt(match[1], 10),
		minor: parseInt(match[2], 10),
		patch: parseInt(match[3], 10),
		channel: match[4] || null,
		prerelease: match[5] ? parseInt(match[5], 10) : null,
	}
}

function formatVersion({ major, minor, patch, channel, prerelease }) {
	const base = `${major}.${minor}.${patch}`
	if (channel && prerelease !== null) {
		return `${base}-${channel}.${prerelease}`
	}
	return base
}

function bumpBase(parsed, type) {
	switch (type) {
		case 'major':
			return { ...parsed, major: parsed.major + 1, minor: 0, patch: 0 }
		case 'minor':
			return { ...parsed, minor: parsed.minor + 1, patch: 0 }
		case 'patch':
			return { ...parsed, patch: parsed.patch + 1 }
		default:
			throw new Error(`Invalid bump type: ${type}`)
	}
}

function bumpVersion(version, bumpType, channel = null) {
	const parsed = parseVersion(version)

	// Standard bumps (major, minor, patch)
	if (STANDARD_BUMPS.includes(bumpType)) {
		if (parsed.channel) {
			throw new Error(`Cannot use '${bumpType}' on prerelease version ${version}. Use 'stage' to promote to stable.`)
		}

		// If channel specified, start a prerelease
		if (channel) {
			if (!PRERELEASE_CHANNELS.includes(channel)) {
				throw new Error(`Invalid channel: ${channel}. Use: ${PRERELEASE_CHANNELS.join(', ')}`)
			}
			const bumped = bumpBase(parsed, bumpType)
			return formatVersion({ ...bumped, channel, prerelease: 1 })
		}

		// Standard bump to stable
		const bumped = bumpBase(parsed, bumpType)
		return formatVersion({ ...bumped, channel: null, prerelease: null })
	}

	// Prerelease increment
	if (bumpType === 'prerelease') {
		if (!parsed.channel) {
			throw new Error(`Not on prerelease. Use 'minor staging' to start a prerelease.`)
		}
		return formatVersion({ ...parsed, prerelease: parsed.prerelease + 1 })
	}

	// Stage: promote staging to stable
	if (bumpType === 'stage') {
		if (!parsed.channel) {
			throw new Error(`Not on prerelease. Use 'minor staging' to start a prerelease.`)
		}
		return formatVersion({ ...parsed, channel: null, prerelease: null })
	}

	throw new Error(`Invalid bump type: ${bumpType}`)
}

function updateVersion(bumpType, channel = null) {
	// Validate bump type
	const validTypes = [...STANDARD_BUMPS, 'prerelease', 'stage']
	if (!validTypes.includes(bumpType)) {
		console.error(`Invalid bump type: ${bumpType}`)
		printUsage()
		process.exit(1)
	}

	// Read current version from package.json
	const packageJsonPath = join(ROOT, 'package.json')
	const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'))
	const oldVersion = packageJson.version
	const newVersion = bumpVersion(oldVersion, bumpType, channel)

	// Cargo.lock records the app's own version too: left behind, the next cargo command rewrites it
	// and the release commit would not match what was built. Checked before anything is written, so a
	// failure never leaves a half-bumped tree.
	const cargoTomlPath = join(ROOT, 'src-tauri', 'Cargo.toml')
	let cargoToml = readFileSync(cargoTomlPath, 'utf-8')
	const crateName = cargoToml.match(/^name = "([^"]+)"$/m)?.[1]
	const cargoLockPath = join(ROOT, 'src-tauri', 'Cargo.lock')
	const cargoLock = readFileSync(cargoLockPath, 'utf-8')
	const lockEntry = new RegExp(`(\\[\\[package\\]\\]\\nname = "${crateName}"\\nversion = ")[^"]+(")`)
	if (!crateName || !lockEntry.test(cargoLock)) {
		console.error(`Could not find the "${crateName}" entry in src-tauri/Cargo.lock: nothing was changed.`)
		process.exit(1)
	}

	// Update package.json
	packageJson.version = newVersion
	writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, '\t') + '\n')
	console.log(`Updated package.json: ${oldVersion} -> ${newVersion}`)

	// Update Cargo.toml (regex handles optional prerelease suffix)
	cargoToml = cargoToml.replace(/^version = "[\d.]+(?:-[a-zA-Z]+\.\d+)?"$/m, `version = "${newVersion}"`)
	writeFileSync(cargoTomlPath, cargoToml)
	console.log(`Updated src-tauri/Cargo.toml: ${oldVersion} -> ${newVersion}`)

	writeFileSync(
		cargoLockPath,
		cargoLock.replace(lockEntry, (_, before, after) => `${before}${newVersion}${after}`)
	)
	console.log(`Updated src-tauri/Cargo.lock (${crateName}): -> ${newVersion}`)

	// Compute base version (no prerelease) for production tauri.conf.json
	const parsed = parseVersion(newVersion)
	const baseVersion = formatVersion({ ...parsed, channel: null, prerelease: null })

	// Update tauri.conf.json with base version only (no prerelease). Only the version line is
	// rewritten, so the file keeps its hand-made formatting (compact arrays) and the diff stays one line.
	const tauriConfPath = join(ROOT, 'src-tauri', 'tauri.conf.json')
	const tauriConfText = readFileSync(tauriConfPath, 'utf-8')
	const versionLine = /^(\s*"version":\s*")[^"]*(")/m
	if (!versionLine.test(tauriConfText)) throw new Error('No "version" line in src-tauri/tauri.conf.json')
	const updatedTauriConf = tauriConfText.replace(versionLine, (_, before, after) => `${before}${baseVersion}${after}`)
	// The first "version" line must be the top-level one: refuse to write anything else.
	if (JSON.parse(updatedTauriConf).version !== baseVersion) {
		throw new Error('The first "version" line of src-tauri/tauri.conf.json is not the top-level version')
	}
	writeFileSync(tauriConfPath, updatedTauriConf)
	console.log(`Updated src-tauri/tauri.conf.json: -> ${baseVersion}`)

	// Update tauri.staging.conf.json with full version
	const stagingConfPath = join(ROOT, 'src-tauri', 'tauri.staging.conf.json')
	const stagingConf = JSON.parse(readFileSync(stagingConfPath, 'utf-8'))
	if (parsed.prerelease !== null) {
		// Staging: the full version (the fork ships macOS only, so no Windows/WiX override).
		stagingConf.version = newVersion
	} else {
		// Stable: the staging app inherits the base version.
		delete stagingConf.version
	}
	if (stagingConf.bundle?.windows) delete stagingConf.bundle.windows
	if (stagingConf.bundle && Object.keys(stagingConf.bundle).length === 0) delete stagingConf.bundle
	writeFileSync(stagingConfPath, JSON.stringify(stagingConf, null, '\t') + '\n')
	console.log(
		`Updated src-tauri/tauri.staging.conf.json: -> ${parsed.prerelease !== null ? newVersion : '(inherited)'}`
	)

	// Update tauri.dev.conf.json with dev version
	const devConfPath = join(ROOT, 'src-tauri', 'tauri.dev.conf.json')
	const devConf = JSON.parse(readFileSync(devConfPath, 'utf-8'))
	const devVersion = `${baseVersion}-dev`
	devConf.version = devVersion
	writeFileSync(devConfPath, JSON.stringify(devConf, null, '\t') + '\n')
	console.log(`Updated src-tauri/tauri.dev.conf.json: -> ${devVersion}`)

	console.log(`\nVersion bumped from ${oldVersion} to ${newVersion}`)
}

function printUsage() {
	console.error(`
Usage:
  yarn bump <major|minor|patch>            # Standard version bump
  yarn bump <major|minor|patch> staging    # Start staging prerelease
  yarn bump prerelease                     # Increment prerelease number
  yarn bump stage                          # Promote staging to stable

Examples:
  yarn bump minor                # 0.1.0 -> 0.2.0
  yarn bump minor staging        # 0.1.0 -> 0.2.0-staging.1
  yarn bump prerelease           # 0.2.0-staging.1 -> 0.2.0-staging.2
  yarn bump stage                # 0.2.0-staging.2 -> 0.2.0
`)
}

// --print mode: compute and print the new version without writing files
if (process.argv[2] === '--print') {
	const bumpType = process.argv[3]
	const channel = process.argv[4] || null

	if (!bumpType) {
		console.error('Usage: node scripts/version.js --print <bump_type> [channel]')
		process.exit(1)
	}

	try {
		const packageJsonPath = join(ROOT, 'package.json')
		const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'))
		const newVersion = bumpVersion(packageJson.version, bumpType, channel)
		console.log(newVersion)
	} catch (error) {
		console.error(`Error: ${error.message}`)
		process.exit(1)
	}
} else {
	const bumpType = process.argv[2]
	const channel = process.argv[3]

	if (!bumpType) {
		printUsage()
		process.exit(1)
	}

	try {
		updateVersion(bumpType, channel)
	} catch (error) {
		console.error(`Error: ${error.message}`)
		process.exit(1)
	}
}
