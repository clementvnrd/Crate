import { expect, test, type Page } from '@playwright/test'
import { VIEWS, preparePage, harnessUrl, settle, unmockedCommands, waitForApp, watchErrors } from './app'

// Next-track suggestions (CRA-133): a ranked list learned from the transitions the owner actually played in
// Rekordbox, filled out with a harmonically and rhythmically compatible fallback. The fake backend behind
// these cases is `apps/desktop/harness/handlers/library.ts`'s `suggest_next_tracks` handler.

const player = VIEWS.find((view) => view.id === 'player')!

async function openPlayer(page: Page, options: Parameters<typeof harnessUrl>[0] = {}): Promise<string[]> {
	const errors = watchErrors(page)
	await preparePage(page)
	await page.goto(harnessUrl(options))
	await waitForApp(page)
	await player.open(page)
	await settle(page)
	return errors
}

async function calls(page: Page): Promise<{ command: string; args: Record<string, unknown> }[]> {
	return page.evaluate(
		() =>
			(window as unknown as { __harness: { calls: { command: string; args: Record<string, unknown> }[] } }).__harness
				.calls
	)
}

test.describe('Player — suggested next', () => {
	test.use({ viewport: { width: 1400, height: 900 } })

	test('hints to add the track to the library when the Hero track is not one', async ({ page }) => {
		// With no `playing` param the Hero falls back to the most recent standalone file, which the
		// harness never routes through the 'library' playback source — nothing eligible to suggest from.
		const errors = await openPlayer(page)
		await expect(player.landmark(page)).toBeVisible()

		await page.getByRole('radio', { name: 'Suggested next' }).click()
		await expect(page.getByRole('heading', { name: 'Add this track to your library' })).toBeVisible()

		expect(errors).toEqual([])
		expect(await unmockedCommands(page)).toEqual([])
	})

	test('lists what the owner actually played next, then compatible tracks', async ({ page }) => {
		const errors = await openPlayer(page, { params: { playing: 'trk-01' } })
		await expect(page.getByRole('heading', { level: 1 })).toHaveText('Afterglow Protocol')

		await page.getByRole('radio', { name: 'Suggested next' }).click()

		// Neon Rain: played next once (history) — a key clash the owner mixed into anyway. Undertow:
		// never played after it, but a compatible fallback (see the harness's SUGGESTION_HISTORY and
		// compatibleIds — both are computed from the same seed data as the rest of the harness library).
		await expect(page.getByText('Neon Rain')).toBeVisible()
		await expect(page.getByText('Undertow')).toBeVisible()
		await expect(page.getByText('1 time').first()).toBeVisible()
		await expect(page.getByText('Compatible').first()).toBeVisible()

		expect(await calls(page)).toContainEqual(
			expect.objectContaining({ command: 'suggest_next_tracks', args: { trackId: 'trk-01', limit: null } })
		)
		expect(errors).toEqual([])
		expect(await unmockedCommands(page)).toEqual([])
	})

	test('plays a suggestion with one click', async ({ page }) => {
		await openPlayer(page, { params: { playing: 'trk-01' } })
		await page.getByRole('radio', { name: 'Suggested next' }).click()

		await page.getByRole('button', { name: 'Play "Undertow"' }).click()

		await expect(page.getByRole('heading', { level: 1 })).toHaveText('Undertow')
	})
})

// Continuous playback (CRA-148): when the library track ends, the next one starts by itself. The harness's fake
// engine now ends like the real one — `is_playing: false` with the position clamped at the duration — which is
// what the app's backend sync sees when the track runs out between two interpolated ticks.
test.describe('Player — end of track', () => {
	test.use({ viewport: { width: 1400, height: 900 } })

	test('starts the next library track when the current one ends (shuffle off)', async ({ page }) => {
		const errors = await openPlayer(page, { params: { playing: 'trk-01' } })
		await expect(page.getByRole('heading', { level: 1 })).toHaveText('Afterglow Protocol')

		const playTrackIds = async () => (await calls(page)).filter((c) => c.command === 'play_track').map((c) => c.args.id)

		// Start the restored track, then jump to its very end with the waveform slider's End key.
		await page.keyboard.press('Space')
		await expect.poll(playTrackIds).toEqual(['trk-01'])
		await page.getByRole('slider', { name: 'Playback position' }).first().focus()
		await page.keyboard.press('End')

		await expect.poll(async () => (await playTrackIds()).length, { timeout: 10_000 }).toBe(2)
		expect((await playTrackIds())[1]).not.toBe('trk-01')
		await expect(page.getByRole('heading', { level: 1 })).not.toHaveText('Afterglow Protocol')

		expect(errors).toEqual([])
		expect(await unmockedCommands(page)).toEqual([])
	})
})
