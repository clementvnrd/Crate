import { createHash, createPublicKey, verify } from 'node:crypto'

/**
 * Pure helpers shared by the release scripts and the release workflow: where releases live, how
 * assets are named, how `latest.json` is built, and which signing key a file was signed with.
 *
 * The updater manifest of each channel is a file committed to the public releases repository
 * (`channels/<channel>/latest.json`), so every change of what the app is offered is a commit and a
 * rollback is a revert. The binaries themselves are GitHub release assets of that repository.
 */

/** Public repository that holds only built apps and update manifests (recommended in CRA-198, pending). */
export const RELEASES_REPO = 'clementvnrd/crate-releases'

/** Key ID of upstream's updater key: a fork build must never trust it again (CRA-199). */
export const UPSTREAM_KEY_ID = '5E32E4C59470B97D'

/** Crate ships for Apple Silicon Macs only (CRA-122). */
export const PLATFORM = 'darwin-aarch64'
export const ARCH = 'aarch64'
export const RUST_TARGET = 'aarch64-apple-darwin'

/** Release notes shown inside the app are capped; the release page keeps the full text. */
export const MAX_MANIFEST_NOTES = 4000

/** GitHub refuses a release body over 125,000 characters; keep a margin. */
export const MAX_RELEASE_NOTES = 120_000

const SEMVER = /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?$/

export function assertVersion(version) {
	if (typeof version !== 'string' || !SEMVER.test(version)) {
		throw new Error(`Invalid version "${version}": expected MAJOR.MINOR.PATCH or MAJOR.MINOR.PATCH-staging.N`)
	}
	return version
}

/** `staging` for `x.y.z-staging.N`, `production` for `x.y.z`; anything else is refused. */
export function channelOf(version) {
	assertVersion(version)
	if (/^\d+\.\d+\.\d+$/.test(version)) return 'production'
	if (/^\d+\.\d+\.\d+-staging\.\d+$/.test(version)) return 'staging'
	throw new Error(`Version "${version}" is neither stable nor -staging.N`)
}

/** The staging channel is a separate app (own identifier and library), see `tauri.staging.conf.json`. */
export function productNameOf(channel) {
	return channel === 'staging' ? 'Crate Staging' : 'Crate'
}

export function tagOf(version) {
	return `v${assertVersion(version)}`
}

export function manifestPath(channel) {
	return `channels/${channel}/latest.json`
}

export function manifestUrl(channel, repo = RELEASES_REPO) {
	return `https://raw.githubusercontent.com/${repo}/main/${manifestPath(channel)}`
}

/** GitHub rewrites spaces in asset names, so assets get explicit, URL-safe names. */
export function assetName(productName, version, extension) {
	return `${productName.replace(/\s+/g, '-')}_${assertVersion(version)}_${ARCH}${extension}`
}

export function assetUrl(version, name, repo = RELEASES_REPO) {
	return `https://github.com/${repo}/releases/download/${tagOf(version)}/${encodeURIComponent(name)}`
}

export function releasePageUrl(version, repo = RELEASES_REPO) {
	return `https://github.com/${repo}/releases/tag/${tagOf(version)}`
}

function decodeBase64(value) {
	return Buffer.from(String(value).trim(), 'base64').toString('utf-8')
}

/** Minisign key IDs are printed as the little-endian u64 stored in bytes 2..10 of the key/signature. */
function keyIdFromPayload(payloadB64) {
	const bytes = Buffer.from(payloadB64.trim(), 'base64')
	if (bytes.length < 10) throw new Error('Malformed minisign payload')
	return Buffer.from(bytes.subarray(2, 10)).reverse().toString('hex').toUpperCase()
}

/** Key ID of a Tauri updater public key (the base64 string stored in `tauri.conf.json`). */
export function keyIdOfPubkey(pubkeyB64) {
	const lines = decodeBase64(pubkeyB64).split('\n')
	if (!lines[0]?.startsWith('untrusted comment:') || !lines[1]) throw new Error('Not a minisign public key')
	return keyIdFromPayload(lines[1])
}

/** Key ID that produced a Tauri `.sig` file (base64 of a minisign signature). */
export function keyIdOfSignature(signatureB64) {
	const lines = decodeBase64(signatureB64).split('\n')
	if (!lines[0]?.startsWith('untrusted comment:') || !lines[1]) throw new Error('Not a minisign signature')
	return keyIdFromPayload(lines[1])
}

/** Cut `text` at a line boundary so that, with `suffix` appended, it fits in `max` characters. */
export function capText(text, max, suffix) {
	const value = String(text ?? '').trim()
	if (value.length <= max) return value
	const cut = value.slice(0, max - suffix.length)
	return cut.slice(0, Math.max(cut.lastIndexOf('\n'), 0) || cut.length).trimEnd() + suffix
}

export function trimNotes(notes, version, repo = RELEASES_REPO) {
	return capText(notes, MAX_MANIFEST_NOTES, `\n\n… Full release notes: ${releasePageUrl(version, repo)}`)
}

/** Release-page body: the whole changelog section, capped under GitHub's limit. */
export function releaseBody(notes, commit) {
	const footer = `\n\n---\nBuilt from commit \`${String(commit).slice(0, 12)}\`.`
	const capped = capText(
		notes,
		MAX_RELEASE_NOTES - footer.length,
		'\n\n… (truncated: see CHANGELOG.md for the full log)'
	)
	return `${capped}${footer}\n`
}

const ED25519_SPKI_PREFIX = Buffer.from('302a300506032b6570032100', 'hex')

/**
 * Verify a Tauri updater signature (minisign, Ed25519) over `fileBytes` with a Tauri public key,
 * exactly like the updater will: the file signature (raw or BLAKE2b-512 prehashed) and the
 * global signature over the trusted comment. Returns false on any mismatch.
 */
export function verifySignature(fileBytes, signatureB64, pubkeyB64) {
	try {
		const keyPayload = Buffer.from(decodeBase64(pubkeyB64).split('\n')[1].trim(), 'base64')
		const [, sigLine, trustedLine, globalLine] = decodeBase64(signatureB64).split('\n')
		const sigPayload = Buffer.from(sigLine.trim(), 'base64')
		if (keyPayload.length !== 42 || sigPayload.length !== 74) return false
		if (!keyPayload.subarray(2, 10).equals(sigPayload.subarray(2, 10))) return false

		const key = createPublicKey({
			key: Buffer.concat([ED25519_SPKI_PREFIX, keyPayload.subarray(10)]),
			format: 'der',
			type: 'spki',
		})
		const algorithm = sigPayload.subarray(0, 2).toString('latin1')
		const message = algorithm === 'ED' ? createHash('blake2b512').update(fileBytes).digest() : Buffer.from(fileBytes)
		const signature = sigPayload.subarray(10)
		if (!verify(null, message, key, signature)) return false

		const trusted = trustedLine.replace(/^trusted comment: /, '')
		const global = Buffer.from(globalLine.trim(), 'base64')
		return verify(null, Buffer.concat([signature, Buffer.from(trusted, 'utf-8')]), key, global)
	} catch {
		return false
	}
}

/** The `latest.json` served to the updater (Tauri v2 static JSON format), Apple Silicon only. */
export function buildManifest({ version, notes, pubDate, signature, url }) {
	assertVersion(version)
	if (!signature || !String(signature).trim()) throw new Error('Missing updater signature')
	keyIdOfSignature(signature)
	if (!/^https:\/\//.test(url ?? '')) throw new Error(`Update URL must be https: ${url}`)
	const date = pubDate instanceof Date ? pubDate : new Date(pubDate)
	if (Number.isNaN(date.getTime())) throw new Error(`Invalid publication date: ${pubDate}`)

	return {
		version,
		notes: trimNotes(notes, version),
		pub_date: date.toISOString().replace(/\.\d{3}Z$/, 'Z'),
		platforms: {
			[PLATFORM]: { signature: String(signature).trim(), url },
		},
	}
}
