import { expect, test, type Locator, type Page } from '@playwright/test'
import { preparePage, harnessUrl, settle, unmockedCommands, waitForApp, watchErrors } from './app'

// The discrepancy report (CRA-129): a read-only comparison of Crate against Mixed In Key and, once an
// XML export is given, Rekordbox. Replaces the old silent automatic purge of tracks missing from Mixed
// In Key with a reviewable report (see [C3]).

async function openFromSettings(page: Page): Promise<void> {
	await page.getByRole('button', { name: 'Settings' }).click()
	await page.getByRole('button', { name: 'Library', exact: true }).click()
	await page.getByRole('button', { name: 'Open the discrepancy report' }).click()
}

async function openDiscrepancyReport(page: Page): Promise<{ errors: string[]; dialog: Locator }> {
	const errors = watchErrors(page)
	await preparePage(page)
	await page.goto(harnessUrl())
	await waitForApp(page)
	await openFromSettings(page)
	const dialog = page.locator('dialog[open]')
	await expect(dialog.getByText('Discrepancy report')).toBeVisible()
	await settle(page)
	return { errors, dialog }
}

test.describe('Discrepancy report', () => {
	test.use({ viewport: { width: 1400, height: 900 } })

	test('shows missing files, distinguishing an unplugged drive from a truly missing file', async ({ page }) => {
		const { errors, dialog } = await openDiscrepancyReport(page)

		await expect(dialog.getByText('16 tracks in Crate')).toBeVisible()

		const missing = dialog.locator('section', { has: page.getByRole('heading', { name: /Missing files/ }) })
		// `.first()`: the full (untruncated) file path text also contains the title, since fixture file names are
		// "Artist - Title.ext", so the plain title string matches both the title line and the path line below it.
		await expect(missing.getByText('Neon Rain', { exact: false }).first()).toBeVisible()
		await expect(missing.getByText('Missing', { exact: true })).toBeVisible()
		await expect(missing.getByText('Undertow', { exact: false }).first()).toBeVisible()
		await expect(missing.getByText('Drive not connected')).toBeVisible()

		expect(errors).toEqual([])
		expect(await unmockedCommands(page)).toEqual([])
	})

	test('shows the Mixed In Key comparison: matches, differences and both missing lists', async ({ page }) => {
		const { dialog } = await openDiscrepancyReport(page)

		const mik = dialog.locator('section', { has: page.getByRole('heading', { name: 'Mixed In Key' }) })
		await expect(mik.getByText('14 tracks match')).toBeVisible()
		await expect(mik.getByText('Low Gravity', { exact: false })).toBeVisible()
		await expect(mik.getByText('Half/double tempo')).toBeVisible()
		await expect(mik.getByText('Ghost Frequencies', { exact: false })).toBeVisible()
		await expect(mik.getByText('Analog Dreams', { exact: false })).toBeVisible()
	})

	test('prompts for a Rekordbox export when none was given, and the native picker can be cancelled', async ({
		page,
	}) => {
		const { errors, dialog } = await openDiscrepancyReport(page)

		const rekordbox = dialog.locator('section', { has: page.getByRole('heading', { name: 'Rekordbox' }) })
		await expect(rekordbox.getByText('Compare with Rekordbox', { exact: false })).toBeVisible()
		const chooseButton = rekordbox.getByRole('button', { name: 'Choose a Rekordbox XML export…' })
		await expect(chooseButton).toBeVisible()

		// The harness always answers a native file dialog with "cancelled": the prompt must stay as is.
		await chooseButton.click()
		await settle(page)
		await expect(rekordbox.getByText('Compare with Rekordbox', { exact: false })).toBeVisible()

		expect(errors).toEqual([])
	})

	test('an empty library is reported as clean, not as a wall of empty sections', async ({ page }) => {
		const errors = watchErrors(page)
		await preparePage(page)
		await page.goto(harnessUrl({ params: { library: 'empty' } }))
		await waitForApp(page)
		await openFromSettings(page)
		const dialog = page.locator('dialog[open]')
		await expect(dialog.getByText('Discrepancy report')).toBeVisible()
		await settle(page)

		await expect(dialog.getByText('Nothing to report')).toBeVisible()
		await expect(dialog.getByText('Every file is where Crate expects it.')).toBeVisible()
		expect(errors).toEqual([])
	})

	test('a failed report can be retried', async ({ page }) => {
		const errors = watchErrors(page)
		await preparePage(page)
		await page.goto(harnessUrl({ params: { discrepancyReport: 'fail' } }))
		await waitForApp(page)
		await openFromSettings(page)
		const dialog = page.locator('dialog[open]')
		await expect(dialog.getByText('Could not build the report')).toBeVisible()

		expect(errors).toEqual([])
	})
})
