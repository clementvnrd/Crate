import { get } from 'svelte/store'
import { translate } from '../i18n'

/**
 * Backend messages (always English) that users meet often and can act on, with the translation
 * key that replaces them in the interface. Every other message is shown as the backend wrote it.
 * The wording is matched on stable prefixes, so reword a backend message and this table together
 * (`AUTH_REQUIRED_PREFIX` in `beatport/client.rs` is the Rust side of the first entry).
 */
const KNOWN_BACKEND_ERRORS: ReadonlyArray<{ pattern: RegExp; key: string }> = [
	{ pattern: /^Beatport authentication required/, key: 'errors.beatportAuthRequired' },
	{ pattern: /^Beatport token expired or invalid/, key: 'errors.beatportTokenInvalid' },
	{ pattern: /^Artist not found on Beatport/, key: 'errors.beatportArtistNotFound' },
	{ pattern: /^The specified path is not a valid folder/, key: 'errors.albumInvalidFolder' },
	{ pattern: /^No supported audio file found in this folder/, key: 'errors.albumNoAudio' },
]

/** The interface-language version of a known backend message, or the message unchanged. */
export function localizeBackendError(message: string): string {
	for (const { pattern, key } of KNOWN_BACKEND_ERRORS) {
		if (pattern.test(message)) return get(translate)(key)
	}
	return message
}

/**
 * Human-readable message of a caught error. Tauri rejects `invoke` with the backend error
 * *string* (CrateError serializes to its message), so `instanceof Error` alone loses it.
 * Known backend messages come back in the interface language.
 */
export function toErrorMessage(error: unknown, fallback: string): string {
	if (typeof error === 'string' && error.trim() !== '') return localizeBackendError(error)
	if (error instanceof Error && error.message) return localizeBackendError(error.message)
	if (error && typeof error === 'object' && 'message' in error) {
		const message = (error as { message: unknown }).message
		if (typeof message === 'string' && message.trim() !== '') return localizeBackendError(message)
	}
	return fallback
}
