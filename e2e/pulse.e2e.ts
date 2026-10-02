import { expect, test, type Page } from '@playwright/test'
import { VIEWS, preparePage, harnessUrl, settle, unmockedCommands, waitForApp, watchErrors } from './app'
import { runAudit } from './audit'

// The Pulse screens built on the listening-history backends: the recap of a week or a year (CRA-125), the
// discovery funnel (CRA-126), the timeline of a Rekordbox set (CRA-127) and the history export (CRA-128).

const pulse = VIEWS.find((view) => view.id === 'pulse')!

async function openPulse(page: Page, options: Parameters<typeof harnessUrl>[0] = {}): Promise<string[]> {
	const errors = watchErrors(page)
	await preparePage(page)
	await page.goto(harnessUrl(options))
	await waitForApp(page)
	await pulse.open(page)
	await expect(pulse.landmark(page)).toBeVisible()
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

test.describe('Pulse history screens', () => {
	test.use({ viewport: { width: 1400, height: 900 } })

	test('the recap shows the week, moves between periods and switches to the year', async ({ page }) => {
		const errors = await openPulse(page)
		const recap = page.getByRole('region', { name: 'Your week' })
		await expect(recap).toBeVisible()
		await expect(recap.getByText('Different tracks')).toBeVisible()
		await expect(recap.getByText('Top artists')).toBeVisible()

		const next = recap.getByRole('button', { name: 'Next period' })
		await expect(next).toBeDisabled()
		await recap.getByRole('button', { name: 'Previous period' }).click()
		await expect(next).toBeEnabled()
		expect(await calls(page)).toContainEqual(
			expect.objectContaining({ command: 'get_recap', args: { period: 'week', offset: 1 } })
		)

		await recap.getByRole('radio', { name: 'Year' }).click()
		await expect(page.getByRole('region', { name: 'Your year' })).toBeVisible()
		expect(await calls(page)).toContainEqual(
			expect.objectContaining({ command: 'get_recap', args: { period: 'year', offset: 0 } })
		)

		expect(errors).toEqual([])
		expect(await unmockedCommands(page)).toEqual([])
	})

	test('the funnel follows the Pulse period and lists the sources', async ({ page }) => {
		const errors = await openPulse(page)
		const funnel = page.getByRole('region', { name: 'Crate to booth' })
		await expect(funnel.getByText('Played in a set').first()).toBeVisible()
		await expect(funnel.getByRole('rowheader', { name: 'Bandcamp' })).toBeVisible()

		await page.getByRole('radio', { name: /^All/ }).click()
		await expect(funnel.getByText('Every release found')).toBeVisible()
		expect(await calls(page)).toContainEqual(
			expect.objectContaining({ command: 'get_discovery_funnel', args: { since: null } })
		)
		expect(errors).toEqual([])
	})

	test('a set opens its timeline in a modal that names every control and fits the window', async ({ page }) => {
		const errors = await openPulse(page)
		await page.getByRole('button', { name: 'Open the timeline of Friday warehouse' }).click()
		const dialog = page.locator('dialog[open]')
		await expect(dialog.getByText('Friday warehouse')).toBeVisible()
		await expect(dialog.getByText('Harmonic transitions')).toBeVisible()
		await expect(dialog.getByText('Opening track')).toBeVisible()
		// Every track shows its Mixed In Key energy, or a dash when Crate does not know the file.
		await expect(dialog.getByRole('columnheader', { name: 'Energy' })).toBeVisible()
		await expect(dialog.getByTitle(/^Energy \d+\/10/).first()).toBeVisible()
		// The energy change from the previous track is shown per row, and a jump of 3 levels or
		// more (the set planner's threshold) is flagged in the transition column and counted.
		await expect(dialog.getByText('Energy jumps')).toBeVisible()
		await expect(dialog.getByText('-6 energy').first()).toBeVisible()
		await settle(page)

		const report = await runAudit(page)
		expect(report.unnamedControls).toEqual([])
		expect(report.outOfWindow).toEqual([])
		expect(report.smallText.filter((entry) => entry.includes('dialog'))).toEqual([])

		await page.keyboard.press('Escape')
		await expect(dialog).toHaveCount(0)
		expect(errors).toEqual([])
		expect(await unmockedCommands(page)).toEqual([])
	})

	test('the export button opens the native save dialog and exports nothing when it is cancelled', async ({ page }) => {
		const errors = await openPulse(page)
		await page.getByRole('button', { name: 'Export history' }).click()
		await expect.poll(async () => (await calls(page)).map((call) => call.command)).toContain('plugin:dialog|save')
		expect((await calls(page)).map((call) => call.command)).not.toContain('export_listening_history')
		expect(errors).toEqual([])
	})
})

test.describe('Pulse history screens, empty and in French', () => {
	test.use({ viewport: { width: 1000, height: 600 } })

	test('an empty history shows the empty states', async ({ page }) => {
		const errors = await openPulse(page, { params: { library: 'empty' } })
		await expect(page.getByText('No plays in this period.')).toBeVisible()
		await expect(page.getByText(/^No discoveries in this period\./)).toBeVisible()
		expect(errors).toEqual([])
	})

	test('the recap and the set timeline read in French', async ({ page }) => {
		const errors = await openPulse(page, { lang: 'fr' })
		await expect(page.getByRole('region', { name: 'Votre semaine' })).toBeVisible()
		await expect(page.getByRole('button', { name: "Exporter l'historique" })).toBeVisible()
		await page.getByRole('button', { name: 'Ouvrir la timeline de Friday warehouse' }).click()
		const dialog = page.locator('dialog[open]')
		await expect(dialog.getByText('Transitions harmoniques')).toBeVisible()
		await expect(dialog.getByRole('columnheader', { name: 'Énergie' })).toBeVisible()
		await expect(dialog.getByText("Sauts d'énergie")).toBeVisible()
		await expect(dialog.getByText('Énergie -6').first()).toBeVisible()
		await settle(page)
		expect((await runAudit(page)).outOfWindow).toEqual([])
		expect(errors).toEqual([])
	})
})
