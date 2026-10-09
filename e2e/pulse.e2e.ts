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

	test('the period bar offers 3 and 6 months and each previous year with data (CRA-184)', async ({ page }) => {
		const errors = await openPulse(page)
		const bar = page.getByRole('radiogroup', { name: 'Period', exact: true })
		const summaryRanges = async () =>
			(await calls(page)).filter((call) => call.command === 'get_stats_summary').map((call) => call.args.timeRange)

		// The rolling months travel as the backend's `3m` and `6m` (CRA-172).
		await bar.getByRole('radio', { name: '3 months' }).click()
		await expect(bar.getByRole('radio', { name: '3 months' })).toHaveAttribute('aria-checked', 'true')
		await expect.poll(summaryRanges).toContain('3m')
		await bar.getByRole('radio', { name: '6 months' }).click()
		await expect.poll(summaryRanges).toContain('6m')

		// The harness has data in its reference year (2026) and the two before; only the years before the current one
		// are offered, newest first, between "This year" and "All time".
		expect((await calls(page)).map((call) => call.command)).toContain('get_listening_years')
		const current = new Date().getFullYear()
		const years = [2026, 2025, 2024].filter((year) => year < current).map(String)
		const labels = (await bar.getByRole('radio').allTextContents()).map((label) => label.trim())
		expect(labels).toEqual(['Today', '7 days', '30 days', '3 months', '6 months', 'This year', ...years, 'All time'])

		// A previous year sends the whole calendar year; the recap opens that year, and the funnel, which cannot end a
		// period yet (CRA-186), counts all time and says so.
		await bar.getByRole('radio', { name: '2025', exact: true }).click()
		await expect.poll(summaryRanges).toContain('year:2025')
		await expect(page.getByRole('region', { name: 'Your year' })).toBeVisible()
		expect(await calls(page)).toContainEqual(
			expect.objectContaining({ command: 'get_recap', args: { period: 'year', offset: current - 2025 } })
		)
		const funnel = page.getByRole('region', { name: 'Crate to booth' })
		await expect(funnel.getByText('Every release found, not limited to 2025 yet')).toBeVisible()
		expect((await calls(page)).filter((call) => call.command === 'get_discovery_funnel').at(-1)?.args).toEqual({
			since: null,
		})

		// The export follows the period too: the save dialog proposes a file named after it.
		await page.getByRole('button', { name: 'Export history' }).click()
		await expect
			.poll(async () =>
				JSON.stringify((await calls(page)).filter((call) => call.command === 'plugin:dialog|save').at(-1)?.args)
			)
			.toContain('crate-listening-history-year-2025.csv')
		expect(errors).toEqual([])
		expect(await unmockedCommands(page)).toEqual([])
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
		// No listening data: no previous year is offered.
		await expect(
			page.getByRole('radiogroup', { name: 'Period', exact: true }).getByRole('radio', { name: /^\d{4}$/ })
		).toHaveCount(0)
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

// The period bar scrolls instead of wrapping (CRA-173): arrows and edge fades show hidden presets, the selected one
// is always in view. Since CRA-184 (3 and 6 months, previous years) it may not fit at 1000 px; the arrows are checked
// in a narrowed box, where they always show.
test.describe('Pulse period bar', () => {
	test.use({ viewport: { width: 1000, height: 600 } })

	const periodBar = (page: Page) => page.getByRole('radiogroup', { name: /^(Period|Période)$/ })
	const previousArrow = (page: Page) => page.getByRole('button', { name: 'Show previous options' })
	const nextArrow = (page: Page) => page.getByRole('button', { name: 'Show more options' })

	for (const lang of ['en', 'fr']) {
		test(`stays on one line beside the title at 1000×600 (${lang})`, async ({ page }) => {
			const errors = await openPulse(page, { lang })
			const bar = await periodBar(page).boundingBox()
			const title = await page.getByRole('heading', { name: 'Crate Pulse & Stats' }).boundingBox()
			expect(bar && title).toBeTruthy()
			// Same header row: the title's middle sits within the bar's height.
			const titleMiddle = title!.y + title!.height / 2
			expect(titleMiddle).toBeGreaterThan(bar!.y)
			expect(titleMiddle).toBeLessThan(bar!.y + bar!.height)
			// One line of presets: every option shares the same vertical middle (the selected one has a border).
			const middles = await periodBar(page)
				.getByRole('radio')
				.evaluateAll((radios) =>
					radios.map((radio) => {
						const box = radio.getBoundingClientRect()
						return Math.round(box.top + box.height / 2)
					})
				)
			expect(new Set(middles).size).toBe(1)
			// The selected period is in view inside the bar, whatever scrolls behind the arrows.
			const [box, selected] = [
				await periodBar(page).boundingBox(),
				await periodBar(page).getByRole('radio', { checked: true }).boundingBox(),
			]
			expect(selected!.x).toBeGreaterThanOrEqual(box!.x)
			expect(selected!.x + selected!.width).toBeLessThanOrEqual(box!.x + box!.width)
			expect((await runAudit(page)).pageOverflowX).toBe(false)
			expect(errors).toEqual([])
		})
	}

	test('in a narrow box, arrows reveal the hidden presets and the selection stays in view', async ({ page }) => {
		const errors = await openPulse(page)
		const bar = periodBar(page)
		await bar.evaluate((scroller) => {
			;(scroller.parentElement as HTMLElement).style.width = '180px'
		})
		await page.getByRole('radio', { name: 'Today' }).click()
		await expect(nextArrow(page)).toBeVisible()
		await expect(previousArrow(page)).toHaveCount(0)
		await expect(bar).toHaveAttribute('data-fade-end', 'true')

		// End selects the last preset, which scrolls into view clear of the arrow; the arrows swap sides.
		await page.getByRole('radio', { name: 'Today' }).press('End')
		const last = page.getByRole('radio', { name: 'All time' })
		await expect(last).toHaveAttribute('aria-checked', 'true')
		await expect(last).toBeFocused()
		await expect(previousArrow(page)).toBeVisible()
		await expect(nextArrow(page)).toHaveCount(0)
		await expect(bar).toHaveAttribute('data-fade-start', 'true')
		const inside = async () => {
			const [box, item] = [await bar.boundingBox(), await last.boundingBox()]
			return item!.x >= box!.x && item!.x + item!.width <= box!.x + box!.width
		}
		expect(await inside()).toBe(true)

		// The arrows only scroll: the selection does not change and returning to the start hides the left arrow.
		for (let press = 0; press < 12 && (await previousArrow(page).count()) > 0; press++) {
			await previousArrow(page).click()
			await settle(page)
		}
		await expect(previousArrow(page)).toHaveCount(0)
		await expect(nextArrow(page)).toBeVisible()
		await expect(last).toHaveAttribute('aria-checked', 'true')

		// The arrows stay out of the tab order and the page never scrolls sideways.
		await expect(nextArrow(page)).toHaveAttribute('tabindex', '-1')
		expect((await runAudit(page)).pageOverflowX).toBe(false)
		expect(errors).toEqual([])
	})
})
