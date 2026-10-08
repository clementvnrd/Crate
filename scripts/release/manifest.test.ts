import { readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import {
	MAX_MANIFEST_NOTES,
	RELEASES_REPO,
	UPSTREAM_KEY_ID,
	assetName,
	assetUrl,
	buildManifest,
	channelOf,
	keyIdOfPubkey,
	keyIdOfSignature,
	manifestUrl,
	productNameOf,
	trimNotes,
} from './manifest.mjs'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
const readConf = (name: string) => JSON.parse(readFileSync(path.join(root, 'src-tauri', name), 'utf-8'))

// Upstream's public key (as it was in tauri.conf.json until CRA-199) and a signature it produced.
const UPSTREAM_PUBKEY =
	'dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDVFMzJFNEM1OTQ3MEI5N0QKUldSOXVYQ1V4ZVF5WGh2NXpGc04rTmRWQW1RcVliM2MxazJzUzczOHpCcGZsNlE2TEpyN0dzZ1YK'
// A real signature made with the fork's key (key ID B84B4C7F52EE0B2E) over a probe file.
const FORK_SIGNATURE =
	'dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVRdUMrNVNmMHhMdUIwVkJSSEViYTFEVE1UZXhxMFduVlNWbTV0ekdrZDZ6bWhVbTM4bWNwTXRhZmlmdVVlQUExQnEySEZGOUFJRk9CZ2JkbVdHTWFxcUhsQTNZV3RKMEFVPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkxNDg3MDA2CWZpbGU6cHJvYmUudHh0ClM0WDQzWXRyRDB5V296aXdTODFsWTZxQUt1T1NXS2h0eVh1NENPZFlUQ0tHWExpd3FzdVRQNlhFaWtEQUIzQTJFYVIvRHYyTGVwYk9pMkRNVFIxV0JRPT0K'

describe('channels and names', () => {
	it('maps stable and staging versions to their channel and refuses anything else', () => {
		expect(channelOf('0.3.0')).toBe('production')
		expect(channelOf('0.3.0-staging.2')).toBe('staging')
		expect(() => channelOf('0.3.0-beta.1')).toThrow(/neither stable nor -staging/)
		expect(() => channelOf('v0.3.0')).toThrow(/Invalid version/)
	})

	it('gives the staging channel its own app name', () => {
		expect(productNameOf('production')).toBe('Crate')
		expect(productNameOf('staging')).toBe('Crate Staging')
	})

	it('builds URL-safe asset names and download URLs', () => {
		const name = assetName('Crate Staging', '0.3.0-staging.1', '.app.tar.gz')
		expect(name).toBe('Crate-Staging_0.3.0-staging.1_aarch64.app.tar.gz')
		expect(assetUrl('0.3.0-staging.1', name)).toBe(
			`https://github.com/${RELEASES_REPO}/releases/download/v0.3.0-staging.1/Crate-Staging_0.3.0-staging.1_aarch64.app.tar.gz`
		)
	})
})

describe('minisign key IDs', () => {
	it('reads the key ID of a public key', () => {
		expect(keyIdOfPubkey(UPSTREAM_PUBKEY)).toBe(UPSTREAM_KEY_ID)
	})

	it('reads the key ID that produced a signature', () => {
		expect(keyIdOfSignature(FORK_SIGNATURE)).toBe('B84B4C7F52EE0B2E')
	})

	it('refuses something that is not a minisign key', () => {
		expect(() => keyIdOfPubkey(Buffer.from('hello').toString('base64'))).toThrow(/Not a minisign public key/)
	})
})

describe('buildManifest', () => {
	const base = {
		version: '0.3.0-staging.1',
		notes: 'First pre-release',
		pubDate: '2026-10-08T21:00:00.123Z',
		signature: FORK_SIGNATURE,
		url: 'https://github.com/clementvnrd/crate-releases/releases/download/v0.3.0-staging.1/x.app.tar.gz',
	}

	it('produces the Tauri static JSON format for Apple Silicon only', () => {
		expect(buildManifest(base)).toEqual({
			version: '0.3.0-staging.1',
			notes: 'First pre-release',
			pub_date: '2026-10-08T21:00:00Z',
			platforms: { 'darwin-aarch64': { signature: FORK_SIGNATURE, url: base.url } },
		})
	})

	it('refuses a missing signature, a non-https URL and a bad date', () => {
		expect(() => buildManifest({ ...base, signature: '' })).toThrow(/Missing updater signature/)
		expect(() => buildManifest({ ...base, url: 'http://example.com/x' })).toThrow(/must be https/)
		expect(() => buildManifest({ ...base, pubDate: 'not a date' })).toThrow(/Invalid publication date/)
	})

	it('caps very long notes and links to the full release page', () => {
		const long = Array.from({ length: 400 }, (_, i) => `- change number ${i}`).join('\n')
		const trimmed = trimNotes(long, '0.3.0')
		expect(trimmed.length).toBeLessThanOrEqual(MAX_MANIFEST_NOTES)
		expect(trimmed).toMatch(
			/Full release notes: https:\/\/github\.com\/clementvnrd\/crate-releases\/releases\/tag\/v0\.3\.0$/
		)
		expect(trimNotes('short', '0.3.0')).toBe('short')
	})
})

describe('updater configuration of the fork (guards CRA-199)', () => {
	it('never trusts upstream signing key again', () => {
		const pubkey = readConf('tauri.conf.json').plugins.updater.pubkey
		expect(keyIdOfPubkey(pubkey)).not.toBe(UPSTREAM_KEY_ID)
	})

	it.each([
		['tauri.prod.conf.json', 'production'],
		['tauri.staging.conf.json', 'staging'],
	])('%s only polls the fork release manifest of its channel', (file, channel) => {
		const endpoints: string[] = readConf(file).plugins.updater.endpoints
		expect(endpoints).toEqual([manifestUrl(channel)])
		expect(endpoints.join(' ')).not.toMatch(/storage\.googleapis\.com|blackboxaudio|bbx-audio\.com/)
	})
})
