import { expect, test, type Page } from '@playwright/test'
import { VIEWS, preparePage, harnessUrl, settle, unmockedCommands, waitForApp, watchErrors } from './app'

// A next track that fails to load (CRA-180): the queue skips it, says so once, and never cuts what is playing. The
// harness's `?missing=` parameter makes `play_track` reject with "File not found" for those tracks, before its fake
// engine is touched, like the real `AudioService::play_track`.

const MAX_CONSECUTIVE_LOAD_FAILURES = 10
const player = VIEWS.find((view) => view.id === 'player')!

async function openLibrary(page: Page, missing: string[]): Promise<string[]> {
	const errors = watchErrors(page)
	await preparePage(page)
	await page.goto(harnessUrl({ params: { missing: missing.join(',') } }))
	await waitForApp(page)
	await expect(page.getByText('Afterglow Protocol', { exact: true }).first()).toBeVisible()
	await settle(page)
	return errors
}

async function playTrackCalls(page: Page): Promise<string[]> {
	return page.evaluate(() =>
		(
			window as unknown as { __harness: { calls: { command: string; args: Record<string, unknown> }[] } }
		).__harness.calls
			.filter((call) => call.command === 'play_track')
			.map((call) => String(call.args.id))
	)
}

/** Track ids in the order the library shows them (the queue next / previous walk). */
async function rowOrder(page: Page): Promise<string[]> {
	return page.locator('[data-track-id]').evaluateAll((rows) => rows.map((row) => row.getAttribute('data-track-id')!))
}

async function startRow(page: Page, id: string): Promise<void> {
	await page.locator(`[data-track-id="${id}"]`).dblclick()
	await expect.poll(() => playTrackCalls(page)).toEqual([id])
}

const toasts = (page: Page) => page.getByRole('alert')

test.describe('Queue — a track that fails to load', () => {
	test.use({ viewport: { width: 1400, height: 900 } })

	test('next skips a missing track in the middle of the queue, with one toast naming it', async ({ page }) => {
		const missing = 'trk-05'
		const errors = await openLibrary(page, [missing])
		const order = await rowOrder(page)
		const at = order.indexOf(missing)
		expect(at).toBeGreaterThanOrEqual(0)
		const before = order[(at - 1 + order.length) % order.length]
		const after = order[(at + 1) % order.length]

		await startRow(page, before)
		await page.keyboard.press('Shift+ArrowRight')

		await expect.poll(() => playTrackCalls(page)).toEqual([before, missing, after])
		await expect(toasts(page)).toHaveCount(1)
		await expect(toasts(page)).toContainText('Skipped "Glasshouse": its file could not be loaded')
		// A toast never takes the focus away from where the user is.
		await expect(page.locator('[role="alert"] :focus')).toHaveCount(0)

		expect(errors).toEqual([])
		expect(await unmockedCommands(page)).toEqual([])
	})

	test('a queue of missing files stops after the bound with one calm notice, the playing track untouched', async ({
		page,
	}) => {
		const playing = 'trk-01'
		const others = Array.from({ length: 15 }, (_, i) => `trk-${String(i + 2).padStart(2, '0')}`)
		const errors = await openLibrary(page, others)

		await startRow(page, playing)
		await page.keyboard.press('Shift+ArrowRight')

		await expect.poll(async () => (await playTrackCalls(page)).length).toBe(1 + MAX_CONSECUTIVE_LOAD_FAILURES)
		await expect(toasts(page)).toHaveCount(1)
		// The music never stopped, so the notice says what keeps playing instead of "Playback stopped".
		await expect(toasts(page)).toContainText(
			`Could not load ${MAX_CONSECUTIVE_LOAD_FAILURES} tracks in a row: "Afterglow Protocol" keeps playing`
		)
		// Nothing else is tried, and the track that was playing still plays.
		await page.waitForTimeout(500)
		expect(await playTrackCalls(page)).toHaveLength(1 + MAX_CONSECUTIVE_LOAD_FAILURES)
		const state = await page.evaluate(async () => {
			const internals = (
				window as unknown as {
					__TAURI_INTERNALS__: { invoke: (cmd: string) => Promise<{ is_playing: boolean; current_track_id: string }> }
				}
			).__TAURI_INTERNALS__
			return internals.invoke('get_playback_state')
		})
		expect(state).toMatchObject({ is_playing: true, current_track_id: playing })

		expect(errors).toEqual([])
		expect(await unmockedCommands(page)).toEqual([])
	})

	test('when the playing track ends and nothing after it loads, one error says playback stopped', async ({ page }) => {
		const others = Array.from({ length: 15 }, (_, i) => `trk-${String(i + 2).padStart(2, '0')}`)
		const errors = watchErrors(page)
		await preparePage(page)
		await page.goto(harnessUrl({ params: { playing: 'trk-01', missing: others.join(',') } }))
		await waitForApp(page)
		await player.open(page)
		await settle(page)

		// Start the restored track, then jump to its very end with the waveform slider's End key.
		await page.keyboard.press('Space')
		await expect.poll(() => playTrackCalls(page)).toEqual(['trk-01'])
		await page.getByRole('slider', { name: 'Playback position' }).first().focus()
		await page.keyboard.press('End')

		await expect
			.poll(async () => (await playTrackCalls(page)).length, { timeout: 10_000 })
			.toBe(1 + MAX_CONSECUTIVE_LOAD_FAILURES)
		await expect(toasts(page)).toHaveCount(1)
		await expect(toasts(page)).toContainText(
			`Playback stopped: ${MAX_CONSECUTIVE_LOAD_FAILURES} tracks in a row could not be loaded`
		)
		await page.waitForTimeout(500)
		expect(await playTrackCalls(page)).toHaveLength(1 + MAX_CONSECUTIVE_LOAD_FAILURES)

		expect(errors).toEqual([])
		expect(await unmockedCommands(page)).toEqual([])
	})
})

// Shuffle consistency (CRA-181): every playback start reaches the shuffle session, whatever started it. Before, a
// track started from the Suggested panel (or Duplicate Killer, the Upgrader) left the session on the previous track,
// so "previous" jumped back past it.
test.describe('Queue — shuffle history', () => {
	test.use({ viewport: { width: 1400, height: 900 } })

	test('"previous" goes back to a track started from the Suggested panel', async ({ page }) => {
		const errors = watchErrors(page)
		await preparePage(page)
		await page.goto(harnessUrl({ params: { playing: 'trk-01' } }))
		await waitForApp(page)

		await page.getByRole('button', { name: 'Shuffle', exact: true }).click()
		await page.locator('#wizard-view-switcher button', { hasText: 'Player' }).click()
		await settle(page)
		await page.getByRole('radio', { name: 'Suggested next' }).click()
		await page.getByRole('button', { name: 'Play "Undertow"' }).click()
		const heading = page.getByRole('heading', { level: 1 })
		await expect(heading).toHaveText('Undertow')
		const [undertow] = await playTrackCalls(page)

		await page.getByRole('button', { name: 'Next', exact: true }).click()
		await expect.poll(async () => (await playTrackCalls(page)).length).toBe(2)
		await expect(heading).not.toHaveText('Undertow')

		await page.getByRole('button', { name: 'Previous', exact: true }).click()
		await expect.poll(async () => (await playTrackCalls(page)).at(-1)).toBe(undertow)
		await expect(heading).toHaveText('Undertow')

		expect(errors).toEqual([])
		expect(await unmockedCommands(page)).toEqual([])
	})
})
