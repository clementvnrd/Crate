import { convertFileSrc } from '@tauri-apps/api/core'

/**
 * Converts an artwork relative path to a displayable URL using Tauri's asset protocol.
 * Returns undefined if no artwork path or app data dir is available.
 *
 * @param artworkPath - Relative path like "artwork/{track_id}.webp"
 * @param dataDir - Absolute app data directory supplied by the caller (e.g. the desktop app store)
 * @returns Asset URL for use in img src, or undefined
 */
export function getArtworkUrl(
	artworkPath: string | null | undefined,
	dataDir: string | null | undefined
): string | undefined {
	if (!artworkPath) return undefined
	const trimmed = artworkPath.trim()
	if (
		trimmed.startsWith('http://') ||
		trimmed.startsWith('https://') ||
		trimmed.startsWith('asset://') ||
		trimmed.startsWith('data:') ||
		trimmed.startsWith('blob:')
	) {
		return trimmed
	}
	if (!dataDir) return undefined

	const fullPath = trimmed.startsWith('/') ? trimmed : `${dataDir}/${trimmed}`
	return convertFileSrc(fullPath)
}

