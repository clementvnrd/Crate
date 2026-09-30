import { beforeEach, describe, expect, it, vi } from 'vitest'

const invoke = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => invoke(...args) }))

import { getDiscoveryFunnel } from './discovery'
import {
	analyzeSet,
	applyOrganisation,
	getDiscrepancyReport,
	getOrganisationBatches,
	planOrganisation,
	suggestNextTracks,
	suggestSetOrder,
	undoOrganisation,
} from './library'
import { exportListeningHistory, getRecap, getRekordboxSessionTimeline } from './stats'

// Tauri turns the camelCase keys of the JavaScript side into the snake_case parameters of the Rust
// commands, and an `Option` parameter is sent as `null` when it is absent. These tests pin the
// command names and the exact argument objects, which is where a typo would only show at runtime.

const rule = { destination_root: '/Music', template: '{artist}/{album}/{artist} - {title}' }

describe('the listening-history commands', () => {
	beforeEach(() => invoke.mockReset().mockResolvedValue(undefined))

	it('asks for the current period unless an offset is given', async () => {
		await getRecap('week')
		await getRecap('year', 2)
		expect(invoke).toHaveBeenNthCalledWith(1, 'get_recap', { period: 'week', offset: 0 })
		expect(invoke).toHaveBeenNthCalledWith(2, 'get_recap', { period: 'year', offset: 2 })
	})

	it('exports with a format and a path', async () => {
		await exportListeningHistory('csv', '/tmp/history.csv')
		expect(invoke).toHaveBeenCalledWith('export_listening_history', { format: 'csv', path: '/tmp/history.csv' })
	})

	it('reads the timeline of one set', async () => {
		await getRekordboxSessionTimeline('session-1')
		expect(invoke).toHaveBeenCalledWith('get_rekordbox_session_timeline', { sessionId: 'session-1' })
	})

	it('reads the discovery funnel for all time, or from a day', async () => {
		await getDiscoveryFunnel()
		await getDiscoveryFunnel('2026-09-01')
		expect(invoke).toHaveBeenNthCalledWith(1, 'get_discovery_funnel', { since: null })
		expect(invoke).toHaveBeenNthCalledWith(2, 'get_discovery_funnel', { since: '2026-09-01' })
	})
})

describe('the DJ-preparation commands', () => {
	beforeEach(() => invoke.mockReset().mockResolvedValue(undefined))

	it('suggests next tracks with an optional limit', async () => {
		await suggestNextTracks('t1')
		await suggestNextTracks('t1', 5)
		expect(invoke).toHaveBeenNthCalledWith(1, 'suggest_next_tracks', { trackId: 't1', limit: null })
		expect(invoke).toHaveBeenNthCalledWith(2, 'suggest_next_tracks', { trackId: 't1', limit: 5 })
	})

	it('analyses a set and suggests an order with an optional first track', async () => {
		await analyzeSet(['a', 'b'])
		await suggestSetOrder(['a', 'b'])
		await suggestSetOrder(['a', 'b'], 'b')
		expect(invoke).toHaveBeenNthCalledWith(1, 'analyze_set', { trackIds: ['a', 'b'] })
		expect(invoke).toHaveBeenNthCalledWith(2, 'suggest_set_order', { trackIds: ['a', 'b'], startTrackId: null })
		expect(invoke).toHaveBeenNthCalledWith(3, 'suggest_set_order', { trackIds: ['a', 'b'], startTrackId: 'b' })
	})
})

describe('the library-maintenance commands', () => {
	beforeEach(() => invoke.mockReset().mockResolvedValue(undefined))

	it('builds the report with or without a Rekordbox export', async () => {
		await getDiscrepancyReport()
		await getDiscrepancyReport('/tmp/rekordbox.xml')
		expect(invoke).toHaveBeenNthCalledWith(1, 'get_discrepancy_report', { rekordboxXmlPath: null })
		expect(invoke).toHaveBeenNthCalledWith(2, 'get_discrepancy_report', { rekordboxXmlPath: '/tmp/rekordbox.xml' })
	})

	it('plans for the whole library or for some tracks', async () => {
		await planOrganisation(rule)
		await planOrganisation(rule, ['a'])
		expect(invoke).toHaveBeenNthCalledWith(1, 'plan_organisation', { rule, trackIds: null })
		expect(invoke).toHaveBeenNthCalledWith(2, 'plan_organisation', { rule, trackIds: ['a'] })
	})

	it('applies only with the plan id and the confirmation, and sends every key the command needs', async () => {
		await applyOrganisation(rule, 'plan-1', true, ['a'])
		await applyOrganisation(rule, 'plan-1', false)
		expect(invoke).toHaveBeenNthCalledWith(1, 'apply_organisation', {
			rule,
			trackIds: ['a'],
			expectedPlanId: 'plan-1',
			understandsExternalTools: true,
		})
		expect(invoke).toHaveBeenNthCalledWith(2, 'apply_organisation', {
			rule,
			trackIds: null,
			expectedPlanId: 'plan-1',
			understandsExternalTools: false,
		})
	})

	it('undoes a batch and lists the batches', async () => {
		await undoOrganisation('batch-1')
		await getOrganisationBatches()
		await getOrganisationBatches(3)
		expect(invoke).toHaveBeenNthCalledWith(1, 'undo_organisation', { batchId: 'batch-1' })
		expect(invoke).toHaveBeenNthCalledWith(2, 'get_organisation_batches', { limit: null })
		expect(invoke).toHaveBeenNthCalledWith(3, 'get_organisation_batches', { limit: 3 })
	})
})
