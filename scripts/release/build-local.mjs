#!/usr/bin/env node
/**
 * Build a signed Crate release on this Mac (Apple Silicon), then optionally publish it.
 * This is the recommended way to release (CRA-198): free, about 8 minutes, and the updater
 * signing key never leaves the Mac. The release workflow on GitHub Actions is the backup.
 *
 * Usage (from the repository root):
 *   yarn release:local              # build the version in package.json
 *   yarn release:local --publish    # build, then run scripts/release/publish.mjs
 *   yarn release:local --dry-run    # print what would run, build nothing
 *
 * The signing key is read from ~/.tauri/crate-updater.key and its password from the login Keychain
 * item "crate-updater-signing" (see docs/RELEASING.md). Neither is printed.
 */

import { execFileSync, spawnSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { parseArgs } from 'node:util'
import { notesFor } from '../changelog.js'
import { RUST_TARGET, channelOf, productNameOf } from './manifest.mjs'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..')
const KEY_PATH = join(homedir(), '.tauri', 'crate-updater.key')
const KEYCHAIN_SERVICE = 'crate-updater-signing'

function fail(message) {
	console.error(`\n✗ ${message}`)
	process.exit(1)
}

const { values } = parseArgs({
	options: {
		publish: { type: 'boolean', default: false },
		'dry-run': { type: 'boolean', default: false },
		'allow-dirty': { type: 'boolean', default: false },
	},
})

if (process.platform !== 'darwin' || process.arch !== 'arm64') fail('Releases are built on an Apple Silicon Mac.')

const version = JSON.parse(readFileSync(join(ROOT, 'package.json'), 'utf-8')).version
const channel = channelOf(version)
const config = channel === 'staging' ? 'src-tauri/tauri.staging.conf.json' : 'src-tauri/tauri.prod.conf.json'
// Same feature sets as `yarn build:staging` / `yarn build:production`.
const features = channel === 'staging' ? 'desktop,devtools' : 'desktop'

const dirty = execFileSync('git', ['status', '--porcelain'], { cwd: ROOT, encoding: 'utf-8' }).trim()
if (dirty && !values['allow-dirty']) fail('The working tree has uncommitted changes: a release is built from a commit.')

try {
	notesFor(readFileSync(join(ROOT, 'CHANGELOG.md'), 'utf-8'), version)
} catch {
	const fix = channel === 'staging' ? `yarn changelog:prepare ${version}` : `yarn changelog:graduate ${version}`
	fail(`CHANGELOG.md has no section for ${version}: run \`${fix}\` first.`)
}

// The Tauri CLI is called directly: through `yarn tauri … -- …`, Yarn 1 drops every option written
// before `--`, and the build silently becomes a production build for the default target.
const tauri = join(ROOT, 'node_modules', '.bin', 'tauri')
const args = ['build', '--config', config, '--target', RUST_TARGET, '--features', features]
console.log(`Crate ${version} (${channel}) — tauri ${args.join(' ')}`)
if (values['dry-run']) {
	console.log('--dry-run: nothing built.')
	process.exit(0)
}

if (!existsSync(KEY_PATH)) fail(`Signing key not found at ${KEY_PATH} (see docs/RELEASING.md, "Signing key").`)
let password
try {
	password = execFileSync('security', ['find-generic-password', '-s', KEYCHAIN_SERVICE, '-w'], {
		encoding: 'utf-8',
		stdio: ['ignore', 'pipe', 'ignore'],
	}).trim()
} catch {
	fail(`Keychain item "${KEYCHAIN_SERVICE}" not found (see docs/RELEASING.md, "Signing key").`)
}

const build = spawnSync(tauri, args, {
	cwd: ROOT,
	stdio: 'inherit',
	env: {
		...process.env,
		CRATE_ENV: channel,
		TAURI_SIGNING_PRIVATE_KEY: readFileSync(KEY_PATH, 'utf-8'),
		TAURI_SIGNING_PRIVATE_KEY_PASSWORD: password,
	},
})
if (build.status !== 0) fail(`tauri build exited with ${build.status}.`)

const targetDir = process.env.CARGO_TARGET_DIR || join(ROOT, 'src-tauri', 'target')
const bundle = join(targetDir, RUST_TARGET, 'release', 'bundle')
const archive = join(bundle, 'macos', `${productNameOf(channel)}.app.tar.gz`)
if (!existsSync(archive) || !existsSync(`${archive}.sig`)) {
	fail(`tauri build succeeded but ${archive} (and its .sig) is missing: the build did not use ${config}.`)
}
console.log(`\n✓ Built ${version}: ${bundle}`)

if (values.publish) {
	const script = join(ROOT, 'scripts', 'release', 'publish.mjs')
	const publish = spawnSync('node', [script, '--version', version, '--bundle-dir', bundle], {
		cwd: ROOT,
		stdio: 'inherit',
	})
	process.exit(publish.status ?? 1)
} else {
	console.log(`Next: yarn release:publish --version ${version} --bundle-dir "${bundle}"`)
}
