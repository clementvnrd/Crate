#!/usr/bin/env node
/**
 * Publish a built Crate release to the public releases repository, then point the update manifest
 * of its channel at it. Runs on the Mac (`yarn release:publish`, after `yarn release:local`) and in
 * the release workflow. Needs the GitHub CLI, authenticated (or `GH_TOKEN` set in CI).
 *
 * Usage (from the repository root):
 *   node scripts/release/publish.mjs --version 1.0.0-staging.1 [--bundle-dir <dir>] [--dry-run]
 *   node scripts/release/publish.mjs --version 1.0.0 --repoint [--dry-run]
 *   ... [--allow-key <KEY ID>]   # key rotation only, see docs/RELEASING.md
 *
 * Before anything is uploaded, the bundle must be the version being published (read from the
 * app's Info.plist inside the archive) and its signature must verify against the public key the
 * installed apps trust. `--repoint` re-points the channel manifest to a release that is already
 * published: this is the rollback path. A published version is never replaced: bump instead.
 */

import { execFileSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { parseArgs } from 'node:util'
import { notesFor } from '../changelog.js'
import {
	RELEASES_REPO,
	RUST_TARGET,
	assetName,
	assetUrl,
	buildManifest,
	channelOf,
	keyIdOfPubkey,
	keyIdOfSignature,
	manifestPath,
	productNameOf,
	releaseBody,
	releasePageUrl,
	tagOf,
	verifySignature,
} from './manifest.mjs'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..')
const DEFAULT_BUNDLE_DIR = join(
	process.env.CARGO_TARGET_DIR || join(ROOT, 'src-tauri', 'target'),
	RUST_TARGET,
	'release',
	'bundle'
)

function fail(message) {
	console.error(`\n✗ ${message}`)
	process.exit(1)
}

function gh(args, options = {}) {
	return execFileSync('gh', args, { encoding: 'utf-8', stdio: ['pipe', 'pipe', 'pipe'], ...options })
}

function ghSucceeds(args) {
	try {
		gh(args)
		return true
	} catch {
		return false
	}
}

function readJson(path) {
	return JSON.parse(readFileSync(path, 'utf-8'))
}

/** The public key the next build ships with (tauri.conf.json). */
function configuredPubkey() {
	return readJson(join(ROOT, 'src-tauri', 'tauri.conf.json')).plugins.updater.pubkey
}

/**
 * The signature must come from the key the installed apps trust. Normally that is the configured
 * key; during a key rotation the transition release is signed with the old key (`--allow-key`).
 */
function checkSigningKey(signature, allowKey) {
	const configured = keyIdOfPubkey(configuredPubkey())
	const signedWith = keyIdOfSignature(signature)
	const accepted = allowKey ? allowKey.toUpperCase() : configured
	if (signedWith !== accepted) {
		fail(`The update was signed with key ${signedWith}, but installed apps trust ${accepted}. Refusing to publish.`)
	}
	return signedWith
}

/** Current content and blob sha of the channel manifest in the releases repository, if any. */
function currentManifest(repo, path) {
	try {
		const raw = JSON.parse(gh(['api', `repos/${repo}/contents/${path}?ref=main`]))
		return { sha: raw.sha, json: JSON.parse(Buffer.from(raw.content, 'base64').toString('utf-8')) }
	} catch {
		return { sha: null, json: null }
	}
}

function putManifest(repo, channel, manifest, message) {
	const path = manifestPath(channel)
	const { sha, json: previous } = currentManifest(repo, path)
	const body = {
		message,
		branch: 'main',
		content: Buffer.from(JSON.stringify(manifest, null, 2) + '\n').toString('base64'),
		...(sha ? { sha } : {}),
	}
	gh(['api', '-X', 'PUT', `repos/${repo}/contents/${path}`, '--input', '-'], { input: JSON.stringify(body) })

	const after = currentManifest(repo, path).json
	if (after?.version !== manifest.version) fail(`The ${channel} manifest did not update (still ${after?.version}).`)
	console.log(`✓ ${path}: ${previous?.version ?? '(none)'} → ${manifest.version}`)
}

/** Version of the app inside the updater archive, read from its Info.plist. */
function bundledVersion(tarball, productName) {
	const plist = execFileSync('tar', ['-xzOf', tarball, `${productName}.app/Contents/Info.plist`], {
		encoding: 'utf-8',
		maxBuffer: 16 * 1024 * 1024,
	})
	const read = (key) => plist.match(new RegExp(`<key>${key}</key>\\s*<string>([^<]+)</string>`))?.[1]
	return { short: read('CFBundleShortVersionString'), build: read('CFBundleVersion') }
}

/** The three build outputs of *this* version, or a clear failure. */
function findBundle(bundleDir, productName, version) {
	const tarball = join(bundleDir, 'macos', `${productName}.app.tar.gz`)
	const signature = `${tarball}.sig`
	const dmg = join(bundleDir, 'dmg', `${productName}_${version}_aarch64.dmg`)
	for (const file of [tarball, signature, dmg]) if (!existsSync(file)) fail(`Missing build output: ${file}`)

	const { short, build } = bundledVersion(tarball, productName)
	if (short !== version && build !== version) {
		fail(`${tarball} contains version ${short ?? build ?? '?'}, not ${version}: rebuild with \`yarn release:local\`.`)
	}
	return { tarball, signature, dmg }
}

function publish({ version, bundleDir, repo, dryRun, allowKey }) {
	const channel = channelOf(version)
	const productName = productNameOf(channel)
	const tag = tagOf(version)

	const packageVersion = readJson(join(ROOT, 'package.json')).version
	if (packageVersion !== version) fail(`package.json is at ${packageVersion}, not ${version}: run \`yarn bump\` first.`)

	const files = findBundle(bundleDir, productName, version)
	const signature = readFileSync(files.signature, 'utf-8').trim()
	const signedWith = checkSigningKey(signature, allowKey)
	const trustedKey = allowKey ? null : configuredPubkey()
	if (trustedKey && !verifySignature(readFileSync(files.tarball), signature, trustedKey)) {
		fail(`The signature of ${files.tarball} does not verify: the archive changed after signing. Rebuild.`)
	}

	const notes = notesFor(readFileSync(join(ROOT, 'CHANGELOG.md'), 'utf-8'), version)
	const stage = mkdtempSync(join(tmpdir(), 'crate-release-'))
	const names = {
		tarball: assetName(productName, version, '.app.tar.gz'),
		signature: assetName(productName, version, '.app.tar.gz.sig'),
		dmg: assetName(productName, version, '.dmg'),
	}
	copyFileSync(files.tarball, join(stage, names.tarball))
	copyFileSync(files.signature, join(stage, names.signature))
	copyFileSync(files.dmg, join(stage, names.dmg))
	// The release page names the source commit (the code itself stays in the private repository).
	const commit =
		process.env.GITHUB_SHA ?? execFileSync('git', ['rev-parse', 'HEAD'], { cwd: ROOT, encoding: 'utf-8' }).trim()
	writeFileSync(join(stage, 'notes.md'), releaseBody(notes, commit))

	const manifest = buildManifest({
		version,
		notes,
		pubDate: new Date(),
		signature,
		url: assetUrl(version, names.tarball, repo),
	})

	console.log(`Release ${tag} (${channel}) → ${repo}`)
	console.log(
		`  bundle version ${version} ✓, signed with key ${signedWith} ✓${trustedKey ? ', signature verified ✓' : ''}`
	)
	for (const name of Object.values(names)) console.log(`  asset ${name}`)
	if (dryRun) {
		console.log('\n--dry-run: nothing published. Manifest that would be served:\n')
		console.log(JSON.stringify({ ...manifest, notes: `${manifest.notes.slice(0, 120)}…` }, null, 2))
		return
	}

	if (ghSucceeds(['release', 'view', tag, '--repo', repo])) {
		fail(
			`${tag} is already published in ${repo}. Published builds are never replaced: bump the version. ` +
				`If only the manifest update failed last time, run: yarn release:publish --version ${version} --repoint`
		)
	}
	gh(
		[
			'release',
			'create',
			tag,
			'--repo',
			repo,
			'--title',
			`${productName} ${version}`,
			'--notes-file',
			join(stage, 'notes.md'),
			...(channel === 'staging' ? ['--prerelease', '--latest=false'] : ['--latest']),
			join(stage, names.tarball),
			join(stage, names.signature),
			join(stage, names.dmg),
		],
		{ stdio: 'inherit' }
	)
	console.log(`✓ release created: ${releasePageUrl(version, repo)}`)
	putManifest(repo, channel, manifest, `Publish ${productName} ${version} on the ${channel} channel`)
}

/** Rollback: serve an already published release again (installed copies never downgrade by themselves). */
function repoint({ version, repo, dryRun, allowKey }) {
	const channel = channelOf(version)
	const productName = productNameOf(channel)
	const tag = tagOf(version)
	if (!ghSucceeds(['release', 'view', tag, '--repo', repo])) fail(`${tag} is not published in ${repo}.`)

	const stage = mkdtempSync(join(tmpdir(), 'crate-repoint-'))
	const sigName = assetName(productName, version, '.app.tar.gz.sig')
	gh(['release', 'download', tag, '--repo', repo, '--pattern', sigName, '--dir', stage])
	const signature = readFileSync(join(stage, sigName), 'utf-8').trim()
	checkSigningKey(signature, allowKey)
	const notes = JSON.parse(gh(['release', 'view', tag, '--repo', repo, '--json', 'body'])).body
	const manifest = buildManifest({
		version,
		notes,
		pubDate: new Date(),
		signature,
		url: assetUrl(version, assetName(productName, version, '.app.tar.gz'), repo),
	})

	console.log(`Re-point the ${channel} manifest to ${tag}`)
	if (dryRun) {
		console.log(JSON.stringify({ ...manifest, notes: `${manifest.notes.slice(0, 120)}…` }, null, 2))
		return
	}
	putManifest(repo, channel, manifest, `Re-point the ${channel} channel to ${productName} ${version}`)
}

const { values } = parseArgs({
	options: {
		version: { type: 'string' },
		'bundle-dir': { type: 'string', default: DEFAULT_BUNDLE_DIR },
		repo: { type: 'string', default: RELEASES_REPO },
		repoint: { type: 'boolean', default: false },
		'allow-key': { type: 'string' },
		'dry-run': { type: 'boolean', default: false },
	},
})

if (!values.version)
	fail('Usage: node scripts/release/publish.mjs --version <x.y.z[-staging.N]> [--repoint] [--dry-run]')

try {
	const options = {
		version: values.version,
		bundleDir: values['bundle-dir'],
		repo: values.repo,
		dryRun: values['dry-run'],
		allowKey: values['allow-key'],
	}
	if (values.repoint) repoint(options)
	else publish(options)
} catch (error) {
	fail(error.stderr?.toString().trim() || error.message)
}
