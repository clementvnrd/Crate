import { expect, test, type Page } from '@playwright/test'
import { harnessUrl, preparePage, waitForApp, watchErrors } from './app'

// The in-app update experience (CRA-200): an update never opens a window by itself; a one-line banner under the
// toolbar offers it, announced as a status. The harness offers Crate 0.4.0 with `?update=available` (see
// apps/desktop/harness/README.md); without the parameter there is never an update.

async function openApp(page: Page, params: Record<string, string> = {}): Promise<string[]> {
	const errors = watchErrors(page)
	await preparePage(page)
	await page.goto(harnessUrl({ params }))
	await waitForApp(page)
	return errors
}

/** Settings → About → "Check for Updates". */
async function checkFromAbout(page: Page): Promise<void> {
	await page.getByRole('button', { name: 'Settings' }).click()
	const settings = page.locator('dialog[open]')
	await settings.getByRole('button', { name: 'About', exact: true }).click()
	await settings.getByRole('button', { name: 'Check for Updates' }).click()
}

test.describe('in-app update', () => {
	test('a found update shows the banner, never a window; notes on request; Later hides it', async ({ page }) => {
		const errors = await openApp(page, { update: 'available' })
		const banner = page.locator('#update-banner')

		// Nothing is offered before a check.
		await expect(banner).toHaveText('')

		await checkFromAbout(page)
		const settings = page.locator('dialog[open]')
		await expect(settings.locator('#about-update-status')).toHaveText('Crate 0.4.0 is available')
		await expect(settings.getByText(/^Last checked/)).toBeVisible()
		await expect(settings.getByText('Update channel')).toBeVisible()
		await expect(settings.getByText('Stable', { exact: true })).toBeVisible()

		// The only open dialog is Settings: the update did not open one of its own.
		await expect(page.locator('dialog[open]')).toHaveCount(1)
		await page.keyboard.press('Escape')
		await expect(page.locator('dialog[open]')).toHaveCount(0)

		// Announced as a status: the message only, not the button labels.
		const status = banner.getByRole('status')
		await expect(status).toHaveAttribute('aria-live', 'polite')
		await expect(status).toHaveText('Crate 0.4.0 is available')
		await expect(banner).toContainText('Crate 0.4.0 is available')
		await expect(banner.getByRole('button', { name: 'Update now' })).toBeVisible()
		await expect(banner.getByRole('button', { name: 'Hide this message' })).toBeVisible()

		await banner.getByRole('button', { name: "See what's new" }).click()
		const notes = page.locator('dialog[open]')
		await expect(notes.getByText("What's new in Crate 0.4.0")).toBeVisible()
		await expect(notes.getByText(/Set mode: build a set from the selected tracks/)).toBeVisible()
		await notes.getByRole('button', { name: 'Close' }).click()
		await expect(page.locator('dialog[open]')).toHaveCount(0)

		await banner.getByRole('button', { name: 'Later' }).click()
		await expect(banner).toHaveText('')

		expect(errors).toEqual([])
	})

	test('"Update now" shows the download progress, then the install, in the banner', async ({ page }) => {
		const errors = await openApp(page, { update: 'available' })
		await checkFromAbout(page)
		await page.keyboard.press('Escape')

		const banner = page.locator('#update-banner')
		await banner.getByRole('button', { name: 'Update now' }).click()
		await expect(banner).toContainText('Downloading Crate 0.4.0…')
		const progress = banner.getByRole('progressbar', { name: 'Download progress' })
		await expect(progress).toBeVisible()
		// Progress events really flow through the updater's channel (not just a bar on screen).
		await expect(progress).toHaveAttribute('aria-valuenow', /^([1-9]\d?|100)$/)
		await expect(banner).toContainText('Installing Crate 0.4.0…', { timeout: 10_000 })
		await expect(page.locator('dialog[open]')).toHaveCount(0)

		expect(errors).toEqual([])
	})

	test('while music plays, the update installs for the next launch and never relaunches', async ({ page }) => {
		const errors = await openApp(page, { update: 'available', playing: 'trk-03' })
		// The player bar comes last in the page; its Play button starts the harness's fake engine.
		await page.getByRole('button', { name: 'Play', exact: true }).last().click()
		await expect(page.getByRole('button', { name: 'Pause', exact: true }).last()).toBeVisible()

		await checkFromAbout(page)
		await page.keyboard.press('Escape')

		const banner = page.locator('#update-banner')
		await expect(banner.getByRole('button', { name: 'Update now' })).toHaveCount(0)
		await banner.getByRole('button', { name: 'Install when I quit' }).click()
		await expect(banner).toContainText('Crate 0.4.0 will open next time you launch Crate.', { timeout: 10_000 })
		// Nothing asked the app to relaunch, and the music is still playing.
		const restarts = await page.evaluate(() =>
			(window as unknown as { __harness: { calls: { command: string }[] } }).__harness.calls
				.map((call) => call.command)
				.filter((command) => command === 'plugin:process|restart')
		)
		expect(restarts).toEqual([])
		await expect(page.getByRole('button', { name: 'Pause', exact: true }).last()).toBeVisible()

		expect(errors).toEqual([])
	})

	test('a failed download says what to do and offers Retry', async ({ page }) => {
		const errors = await openApp(page, { update: 'download-fails' })
		await checkFromAbout(page)
		await page.keyboard.press('Escape')

		const banner = page.locator('#update-banner')
		await banner.getByRole('button', { name: 'Update now' }).click()
		await expect(banner).toContainText("Couldn't download Crate 0.4.0. Check your connection, then retry.", {
			timeout: 10_000,
		})
		await expect(banner.getByRole('button', { name: 'Retry' })).toBeVisible()
		await expect(banner.getByRole('button', { name: 'Details' })).toBeVisible()

		expect(errors).toEqual([])
	})

	test('without an update, "Check for Updates" reports up to date and no banner appears', async ({ page }) => {
		const errors = await openApp(page)
		await checkFromAbout(page)

		const settings = page.locator('dialog[open]')
		await expect(settings.locator('#about-update-status')).toHaveText('Up to date')
		await expect(settings.getByText('Last checked just now')).toBeVisible()
		await expect(page.locator('#update-banner')).toHaveText('')

		expect(errors).toEqual([])
	})
})
