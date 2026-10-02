import type {
	OrganisationBatch,
	OrganisationPlan,
	OrganisationResult,
	OrganisationRule,
	PlannedMove,
} from '$shared/types'
import type { HandlerMap } from '../types'
import type { HarnessState } from '../state'
import { discrepancyReport, DUPLICATE_SCAN } from '../fixtures/maintenance'
import { UPGRADE_MATCHES } from '../fixtures/beatport'

// Duplicate Killer and Beatport Quality Upgrader: both report nothing on an empty library.

// Assisted physical organisation (CRA-130): a small, honest-enough simulation of
// `src-tauri/src/services/library/organise.rs` so a designer or an e2e test can walk the whole
// dry-run -> confirm -> apply -> undo flow without a real filesystem. It renders the same tokens the
// backend does and never marks a move "already in place" unless the computed target equals the
// track's current path, but it does not simulate target collisions or cross-volume folders: every
// renamed file is free to move in the harness, the way an already-tidy library mostly is.

function renderToken(token: string, track: HarnessState['tracks'][number]): string {
	switch (token) {
		case 'artist':
			return track.artist?.trim() || 'Unknown Artist'
		case 'title':
			return track.title?.trim() || 'Untitled'
		case 'album':
			return track.album?.trim() || 'Unknown Album'
		case 'genre':
			return track.genre?.trim() || 'Unknown Genre'
		case 'label':
			return track.label?.trim() || 'Unknown Label'
		case 'key':
			return track.key?.trim() || 'Unknown Key'
		case 'year':
			return track.year ? String(track.year) : 'Unknown Year'
		case 'bpm':
			return track.bpm ? String(Math.round(track.bpm)) : 'Unknown BPM'
		case 'energy':
			return track.energy !== null && track.energy !== undefined ? String(track.energy) : 'Unknown Energy'
		default:
			return `{${token}}`
	}
}

function renderSegment(segment: string, track: HarnessState['tracks'][number]): string {
	return segment
		.replace(/\{(\w+)\}/g, (_, token: string) => renderToken(token, track))
		.replace(/[\\/:*?"<>|]/g, '_')
		.trim()
}

function planOrganisation(state: HarnessState, rule: OrganisationRule, trackIds: string[] | null): OrganisationPlan {
	const wanted = trackIds ? new Set(trackIds) : null
	const root = rule.destination_root.replace(/\/+$/, '')
	const tracks = state.tracks
		.filter((track) => !wanted || wanted.has(track.id))
		.slice()
		.sort((a, b) => `${a.artist}${a.title}`.localeCompare(`${b.artist}${b.title}`))

	const moves: PlannedMove[] = tracks.map((track) => {
		const segments = rule.template.split('/').map((segment) => renderSegment(segment, track))
		const stem = segments.pop() ?? '_'
		const extension = track.file_path.includes('.') ? track.file_path.slice(track.file_path.lastIndexOf('.')) : ''
		const to = [root, ...segments, `${stem}${extension}`].join('/')
		return {
			track_id: track.id,
			title: track.title ?? '',
			artist: track.artist ?? '',
			from: track.file_path,
			to,
			status: to === track.file_path ? 'already_in_place' : 'move',
		}
	})

	const to_move = moves.filter((move) => move.status === 'move').length
	const already_in_place = moves.filter((move) => move.status === 'already_in_place').length
	const id = [rule.destination_root, rule.template, ...moves.map((m) => `${m.track_id}:${m.to}:${m.status}`)].join('|')

	return { id, rule, moves, to_move, already_in_place, blocked: 0, breaks_external_paths: true }
}

export function maintenanceHandlers(state: HarnessState): HandlerMap {
	const empty = state.params.libraryEmpty
	return {
		get_duplicate_groups: () =>
			empty ? { groups: [], total_duplicate_tracks: 0, total_groups: 0, total_reclaimable_bytes: 0 } : DUPLICATE_SCAN,
		get_duplicate_count: () => ({
			group_count: empty ? 0 : DUPLICATE_SCAN.total_groups,
			track_count: empty ? 0 : DUPLICATE_SCAN.total_duplicate_tracks,
			reclaimable_bytes: empty ? 0 : DUPLICATE_SCAN.total_reclaimable_bytes,
		}),

		get_upgrade_matches: () => ({
			matches: empty ? [] : UPGRADE_MATCHES,
			total_scanned: state.tracks.length,
			total_eligible_mp3s: empty ? 0 : 5,
			potential_upgrades_count: empty ? 0 : UPGRADE_MATCHES.length,
		}),
		get_upgrade_count: () => ({
			match_count: empty ? 0 : UPGRADE_MATCHES.length,
			eligible_mp3_count: empty ? 0 : 5,
		}),

		plan_organisation: ({ rule, trackIds }) =>
			planOrganisation(state, rule as OrganisationRule, (trackIds as string[] | null) ?? null),

		apply_organisation: ({ rule, trackIds, expectedPlanId, understandsExternalTools }): OrganisationResult => {
			if (!understandsExternalTools) {
				throw 'confirm that Rekordbox and other tools that remember file paths will lose the moved files'
			}
			const plan = planOrganisation(state, rule as OrganisationRule, (trackIds as string[] | null) ?? null)
			if (plan.id !== expectedPlanId) {
				throw 'the library or the folders changed since the preview: preview again'
			}
			const batch_id = `batch-${++state.organisationBatchCounter}`
			const entries = plan.moves
				.filter((move) => move.status === 'move')
				.map((move) => {
					const track = state.tracks.find((t) => t.id === move.track_id)
					if (track) track.file_path = move.to
					return { track_id: move.track_id, from: move.from, to: move.to, undone: false }
				})
			state.organisationBatches.unshift({ batch_id, moved_at: new Date().toISOString(), entries })
			return { batch_id, moved: entries.length, failed: [] }
		},

		undo_organisation: ({ batchId }): OrganisationResult => {
			const batch = state.organisationBatches.find((b) => b.batch_id === batchId)
			if (!batch) return { batch_id: String(batchId), moved: 0, failed: [] }
			let moved = 0
			for (const entry of batch.entries) {
				if (entry.undone) continue
				const track = state.tracks.find((t) => t.id === entry.track_id)
				if (track) track.file_path = entry.from
				entry.undone = true
				moved++
			}
			return { batch_id: String(batchId), moved, failed: [] }
		},

		get_organisation_batches: ({ limit }): OrganisationBatch[] => {
			const max = typeof limit === 'number' ? limit : 10
			return state.organisationBatches.slice(0, max).map((batch) => ({
				batch_id: batch.batch_id,
				moved_at: batch.moved_at,
				files: batch.entries.length,
				undone: batch.entries.filter((entry) => entry.undone).length,
			}))
		},

		get_discrepancy_report: ({ rekordboxXmlPath }) => {
			if (state.params.discrepancyReportFails) throw 'Could not open the Mixed In Key database'
			return discrepancyReport(empty, (rekordboxXmlPath as string) ?? null)
		},
	}
}
