import { expect, test, type Page } from '@playwright/test'
import { harnessUrl, preparePage, settle, unmockedCommands, waitForApp, watchErrors } from './app'
import { runAudit } from './audit'

// Assisted physical organisation (CRA-130): a naming/folder rule applied to the real files on disk, reached
// from Settings > Library ("Open the organisation tool"). The hard rule behind this screen (register defect
// C6): never move a file without a preview shown first, so every test here checks the dry run is what gets
// applied, and that applying requires an explicit, freshly-ticked acknowledgement every time.

async function openOrganiser(page: Page, options: Parameters<typeof harnessUrl>[0] = {}): Promise<string[]> {
	const errors = watchErrors(page)
	await preparePage(page)
	await page.goto(harnessUrl(options))
	await waitForApp(page)
	await page.getByRole('button', { name: /^(Settings|Paramètres)$/ }).click()
	await page.getByRole('button', { name: /^(Library|Bibliothèque)$/ }).click()
	await page.getByRole('button', { name: /^(Open the organisation tool|Ouvrir l'outil d'organisation)$/ }).click()
	await expect(
		page
			.locator('dialog[open]')
			.last()
			.getByText(/^(Organise files on disk|Organiser les fichiers)/)
	).toBeVisible()
	await settle(page)
	return errors
}

test.describe('Assisted physical organisation', () => {
	test.use({ viewport: { width: 1400, height: 900 } })

	test('the dry run button is disabled until a destination and a rule are set', async ({ page }) => {
		const errors = await openOrganiser(page)
		const organiser = page.locator('dialog[open]').last()
		await expect(organiser.getByRole('button', { name: 'Run the dry run' })).toBeDisabled()
		expect(errors).toEqual([])
	})

	test('the dry run previews every move, applying needs a fresh acknowledgement, and the run can be undone', async ({
		page,
	}) => {
		const errors = await openOrganiser(page)
		const organiser = page.locator('dialog[open]').last()

		await organiser.getByPlaceholder('Choose a folder').fill('/Volumes/Harness/Organised')
		await organiser.getByRole('button', { name: 'Run the dry run' }).click()

		// The dry run touches nothing: it only shows what would happen.
		await expect(organiser.getByRole('columnheader', { name: 'Current location' })).toBeVisible()
		await expect(organiser.getByRole('columnheader', { name: 'New location' })).toBeVisible()
		const willMoveCount = await organiser.locator('dl').getByText(/^\d+$/).first().innerText()
		expect(Number(willMoveCount)).toBeGreaterThan(0)

		const applyButton = organiser.getByRole('button', { name: /^Move \d+ files?$/ })
		await expect(applyButton).toBeEnabled()
		await applyButton.click()

		// Trying to confirm without ticking the acknowledgement box does nothing: no apply call, dialog stays open.
		const confirmDialog = page.locator('dialog[open]').last()
		await expect(confirmDialog.getByText('Move the files?')).toBeVisible()
		await confirmDialog.getByRole('button', { name: /^Move \d+ files?$/ }).click()
		await expect(page.getByText('Check the box to confirm you understand before moving files.')).toBeVisible()
		expect((await calls(page)).map((c) => c.command)).not.toContain('apply_organisation')
		await expect(confirmDialog).toBeVisible()

		// Ticking the box and confirming applies exactly the previewed plan.
		await confirmDialog.getByRole('checkbox', { name: /I understand/ }).check()
		await confirmDialog.getByRole('button', { name: /^Move \d+ files?$/ }).click()

		await expect(organiser.getByText('The library is already organised')).toBeVisible()
		expect((await calls(page)).map((c) => c.command)).toContain('apply_organisation')

		// The applied run shows up under "Past runs" and can be undone.
		const pastRuns = organiser.locator('li', { hasText: 'file' }).first()
		await expect(pastRuns).toBeVisible()
		await pastRuns.getByRole('button', { name: 'Undo' }).click()
		const undoDialog = page.locator('dialog[open]').last()
		await expect(undoDialog.getByText('Undo this run?')).toBeVisible()
		await undoDialog.getByRole('button', { name: 'Undo' }).click()
		await expect(pastRuns.getByRole('button', { name: 'Undone' })).toBeDisabled()

		expect(errors).toEqual([])
		expect(await unmockedCommands(page)).toEqual([])
	})
})

test.describe('Assisted physical organisation, in French and at 1000×600', () => {
	test.use({ viewport: { width: 1000, height: 600 } })

	test('the screen reads in French, names every control and fits the window', async ({ page }) => {
		const errors = await openOrganiser(page, { lang: 'fr' })
		const organiser = page.locator('dialog[open]').last()
		await expect(organiser.getByText('Organiser les fichiers sur le disque')).toBeVisible()
		await expect(organiser.getByRole('button', { name: "Lancer l'aperçu" })).toBeDisabled()
		await settle(page)

		const report = await runAudit(page)
		expect(report.unnamedControls).toEqual([])
		expect(report.outOfWindow).toEqual([])

		expect(errors).toEqual([])
	})
})

async function calls(page: Page): Promise<{ command: string; args: Record<string, unknown> }[]> {
	return page.evaluate(
		() =>
			(window as unknown as { __harness: { calls: { command: string; args: Record<string, unknown> }[] } }).__harness
				.calls
	)
}
