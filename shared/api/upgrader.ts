import { invoke } from '@tauri-apps/api/core'
import type { UpgradeScanResult, UpgradeCountInfo, UpgradeReplacementResult, UpgradeMatch } from '../types'

/**
 * Scan for eligible MP3 tracks and matching Beatport FLAC lossless tracks
 */
export async function getUpgradeMatches(): Promise<UpgradeScanResult> {
	return invoke<UpgradeScanResult>('get_upgrade_matches')
}

/**
 * Get count summary of available upgrade matches
 */
export async function getUpgradeCount(): Promise<UpgradeCountInfo> {
	return invoke<UpgradeCountInfo>('get_upgrade_count')
}

/**
 * Ignore an upgrade match between local track and Beatport track
 */
export async function ignoreUpgradeMatch(trackId: string, beatportId: string): Promise<void> {
	return invoke<void>('ignore_upgrade_match', { trackId, beatportId })
}

/**
 * Unignore an upgrade match
 */
export async function unignoreUpgradeMatch(trackId: string, beatportId: string): Promise<void> {
	return invoke<void>('unignore_upgrade_match', { trackId, beatportId })
}

/**
 * Execute atomic upgrade replacement of selected matches
 */
export async function executeUpgradeReplacements(matches: UpgradeMatch[]): Promise<UpgradeReplacementResult> {
	return invoke<UpgradeReplacementResult>('execute_upgrade_replacements', { matches })
}
