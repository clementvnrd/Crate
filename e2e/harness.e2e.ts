import { expect, test, type Page } from '@playwright/test'
import { VIEWS, preparePage, harnessUrl, unmockedCommands, waitForApp, watchErrors } from './app'

// Checks of the harness itself: its URL parameters, its events, and that the paths a designer walks (start-up,
// every settings tab, the secondary screens) never fall back on a command nobody mocked.

test.use({ viewport: { width: 1400, height: 900 } })

async function start(page: Page, options: Parameters<typeof harnessUrl>[0] = {}): Promise<string[]> {
	const errors = watchErrors(page)
	await preparePage(page)
	await page.goto(harnessUrl(options))
	await waitForApp(page)
	return errors
}

test('start-up reaches the library with no error and no unmocked command', async ({ page }) => {
	const errors = await start(page)
	await expect(page.getByText('16 tracks')).toBeVisible()
	expect(errors).toEqual([])
	expect(await unmockedCommands(page)).toEqual([])
})

test('every view opens without an unmocked command or an error', async ({ page }) => {
	const errors = await start(page)
	for (const view of VIEWS) {
		await page.reload()
		await waitForApp(page)
		await view.open(page)
		await expect(view.landmark(page), `view ${view.id}`).toBeVisible()
	}
	expect(errors).toEqual([])
	expect(await unmockedCommands(page)).toEqual([])
})

test('every settings tab opens without an unmocked command or an error', async ({ page }) => {
	const errors = await start(page)
	await page.keyboard.press('Control+,')
	const dialog = page.locator('dialog[open]')
	await expect(dialog).toBeVisible()
	const tabs = [
		'General',
		'Display',
		'Appearance',
		'Discovery',
		'Library',
		'Sound',
		'Cloud Sync',
		'Beatport',
		'Diagnostics',
		'About',
	]
	for (const tab of tabs) {
		await dialog.getByRole('button', { name: tab, exact: true }).click()
		await page.waitForTimeout(200)
		await expect(dialog).toBeVisible()
	}
	expect(errors).toEqual([])
	expect(await unmockedCommands(page)).toEqual([])
})

test('appearance parameters reach the app', async ({ page }) => {
	await start(page, { theme: 'light', lang: 'fr', accent: 'amber', params: { font: 'fira-code' } })
	const html = page.locator('html')
	await expect(html).toHaveAttribute('data-theme', 'light')
	await expect(html).toHaveAttribute('data-accent', 'amber')
	await expect(html).toHaveAttribute('data-font', 'fira-code')
	await expect(page.getByText('16 pistes')).toBeVisible()
})

test('?beatport=out starts logged out, the default is logged in', async ({ page }) => {
	await start(page, { params: { beatport: 'out' } })
	await page.locator('#wizard-view-switcher button', { hasText: 'Beatport' }).click()
	await expect(page.getByRole('button', { name: /Sign in with Beatport|Se connecter avec Beatport/ })).toBeVisible()

	await page.goto(harnessUrl())
	await waitForApp(page)
	await page.locator('#wizard-view-switcher button', { hasText: 'Beatport' }).click()
	await expect(page.getByText('Peak Time Techno Top 100')).toBeVisible()
})

test('?library=empty shows the empty state', async ({ page }) => {
	const errors = await start(page, { params: { library: 'empty' } })
	await expect(page.getByText('No tracks yet')).toBeVisible()
	expect(errors).toEqual([])
})

test('?playing=<id> restores that track in the player bar', async ({ page }) => {
	await start(page, { params: { playing: 'trk-03' } })
	// The library row comes first in the page, the player bar last.
	await expect(page.getByText('Marlo Vance & The Static Choir').last()).toBeVisible()
	await expect(page.getByText('3:19')).toBeVisible()
})

test('?onboarding=1 shows the onboarding wizard', async ({ page }) => {
	const errors = watchErrors(page)
	await preparePage(page)
	await page.goto(harnessUrl({ params: { onboarding: '1' } }))
	await expect(page.getByText('Welcome to Crate')).toBeVisible()
	expect(errors).toEqual([])
})

test('backend events reach the app: a device leaving raises a toast', async ({ page }) => {
	await start(page)
	await page.evaluate(() =>
		(window as unknown as { __harness: { emit: (event: string, payload: unknown) => Promise<void> } }).__harness.emit(
			'devices-changed',
			[]
		)
	)
	await expect(page.getByText('CDJ-STICK disconnected')).toBeVisible()
})

test('the call log records what the app asked for', async ({ page }) => {
	await start(page)
	const commands = await page.evaluate(() =>
		(window as unknown as { __harness: { calls: { command: string }[] } }).__harness.calls.map((call) => call.command)
	)
	expect(commands).toContain('get_tracks')
	expect(commands).toContain('get_settings')
})
