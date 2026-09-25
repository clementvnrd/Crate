import { invoke } from '@tauri-apps/api/core'
import type { DuplicateScanResult, DuplicateCountInfo } from '../types'

/**
 * Scan for duplicate tracks in the library
 */
export async function getDuplicateGroups(): Promise<DuplicateScanResult> {
	return invoke<DuplicateScanResult>('get_duplicate_groups')
}

/**
 * Get count summary of duplicate groups and tracks
 */
export async function getDuplicateCount(): Promise<DuplicateCountInfo> {
	return invoke<DuplicateCountInfo>('get_duplicate_count')
}

/**
 * Ignore a group of tracks from duplicate detection
 */
export async function ignoreDuplicateGroup(trackIds: string[]): Promise<void> {
	return invoke<void>('ignore_duplicate_group', { trackIds })
}

/**
 * Unignore a group of tracks
 */
export async function unignoreDuplicateGroup(trackIds: string[]): Promise<void> {
	return invoke<void>('unignore_duplicate_group', { trackIds })
}
