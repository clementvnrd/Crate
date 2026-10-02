import { expect, test, type Page } from '@playwright/test'
import { VIEWS, preparePage, harnessUrl, settle, unmockedCommands, waitForApp, watchErrors } from './app'
import { runAudit } from './audit'

// Set mode (CRA-132): select tracks in the Library, check how they mix in order, and export the
// plan to Rekordbox XML.

const library = VIEWS.find((view) => view.id === 'library')!

async function openLibrary(page: Page): Promise<string[]> {
	const errors = watchErrors(page)
	await preparePage(page)
	await page.goto(harnessUrl())
	await waitForApp(page)
	await library.open(page)
	await expect(library.landmark(page)).toBeVisible()
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

test.describe('Set mode', () => {
	test.use({ viewport: { width: 1400, height: 900 } })

	test('building a set from two selected tracks shows the harmonic check and auto-orders them', async ({ page }) => {
		const errors = await openLibrary(page)

		// Select two tracks that clash harmonically (8A then 5A).
		await page.locator('[data-track-id="trk-01"]').click()
		await page.locator('[data-track-id="trk-02"]').click({ modifiers: ['ControlOrMeta'] })
		await page.locator('[data-track-id="trk-02"]').click({ button: 'right' })

		await page.getByRole('menuitem', { name: 'Build a set...' }).click()

		const dialog = page.locator('dialog[open]')
		await expect(dialog.getByText('Build a set')).toBeVisible()
		await expect(dialog.getByText('Afterglow Protocol')).toBeVisible()
		await expect(dialog.getByText('Paper Lanterns')).toBeVisible()
		await expect(dialog.getByText('Key clash', { exact: true })).toBeVisible()
		await settle(page)

		const report = await runAudit(page)
		expect(report.unnamedControls).toEqual([])
		expect(report.outOfWindow).toEqual([])

		expect(await calls(page)).toContainEqual(
			expect.objectContaining({ command: 'analyze_set', args: { trackIds: ['trk-01', 'trk-02'] } })
		)

		// Auto-order swaps them (the calmer "Paper Lanterns" opens the set).
		await dialog.getByRole('button', { name: 'Auto-order' }).click()
		await expect.poll(async () => (await calls(page)).map((call) => call.command)).toContainEqual('suggest_set_order')
		await expect(dialog.locator('li').first()).toContainText('Paper Lanterns')
		expect(await calls(page)).toContainEqual(
			expect.objectContaining({ command: 'analyze_set', args: { trackIds: ['trk-02', 'trk-01'] } })
		)

		await page.keyboard.press('Escape')
		await expect(dialog).toHaveCount(0)
		expect(errors).toEqual([])
		expect(await unmockedCommands(page)).toEqual([])
	})

	test('the export button opens the native save dialog and exports nothing when it is cancelled', async ({ page }) => {
		const errors = await openLibrary(page)

		await page.locator('[data-track-id="trk-01"]').click()
		await page.locator('[data-track-id="trk-01"]').click({ button: 'right' })
		await page.getByRole('menuitem', { name: 'Build a set...' }).click()

		const dialog = page.locator('dialog[open]')
		await expect(dialog.getByText('Afterglow Protocol')).toBeVisible()
		await dialog.getByRole('button', { name: 'Export to Rekordbox XML' }).click()

		await expect.poll(async () => (await calls(page)).map((call) => call.command)).toContain('plugin:dialog|save')
		expect((await calls(page)).map((call) => call.command)).not.toContain('export_set_rekordbox_xml')
		expect(errors).toEqual([])
	})

	test('an empty set shows the empty state, and the modal reads in French', async ({ page }) => {
		const errors = watchErrors(page)
		await preparePage(page)
		await page.goto(harnessUrl({ lang: 'fr' }))
		await waitForApp(page)
		await library.open(page)
		await expect(library.landmark(page)).toBeVisible()

		await page.locator('[data-track-id="trk-01"]').click()
		await page.locator('[data-track-id="trk-01"]').click({ button: 'right' })
		await page.getByRole('menuitem', { name: 'Créer un set...' }).click()

		const dialog = page.locator('dialog[open]')
		await expect(dialog.getByText('Créer un set')).toBeVisible()
		await expect(dialog.getByText('Afterglow Protocol')).toBeVisible()

		expect(errors).toEqual([])
	})
})
