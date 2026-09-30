import { expect, test } from '@playwright/test'
import {
	SCREENSHOT_DIR,
	VIEWS,
	preparePage,
	harnessUrl,
	saveUnmockedReport,
	settle,
	unmockedCommands,
	waitForApp,
	watchErrors,
} from './app'
import { countFindings, runAudit } from './audit'
import { UPDATE_COMMAND, compare, readBaseline, saveMeasurement } from './baseline'

// The matrix: every view, in both themes, at the smallest supported window and a comfortable one, in both
// complete languages. For each case the page must load without an uncaught error or a console error, show the
// view, produce a screenshot, and not have more audit findings than the committed baseline.

const THEMES = ['dark', 'light'] as const
const WINDOWS = [
	{ width: 1000, height: 600 },
	{ width: 1400, height: 900 },
] as const
const LANGUAGES = ['en', 'fr'] as const

for (const theme of THEMES) {
	for (const size of WINDOWS) {
		for (const lang of LANGUAGES) {
			test.describe(`${theme} ${size.width}x${size.height} ${lang}`, () => {
				test.use({
					viewport: { width: size.width, height: size.height },
					colorScheme: theme,
					locale: lang,
				})

				for (const view of VIEWS) {
					test(view.id, async ({ page }) => {
						const caseName = `${view.id}-${theme}-${size.width}-${lang}`
						const errors = watchErrors(page)
						await preparePage(page)

						await page.goto(harnessUrl({ theme, lang }))
						await waitForApp(page)
						await view.open(page)
						await expect(view.landmark(page)).toBeVisible()
						await settle(page)

						await page.screenshot({ path: `${SCREENSHOT_DIR}${caseName}.png` })
						const counts = countFindings(await runAudit(page))
						saveMeasurement(caseName, counts)
						saveUnmockedReport(view.id, await unmockedCommands(page))

						// 1. The page is healthy.
						await expect(page.getByText('Something went wrong')).toHaveCount(0)
						expect(errors, 'uncaught errors or console.error while loading the view').toEqual([])

						// 2. The audit ratchet (skipped while the baseline is being rewritten).
						if (process.env.UPDATE_BASELINE === '1') return
						const reference = readBaseline().cases[caseName]
						expect(reference, `no baseline for ${caseName}: write it with \`${UPDATE_COMMAND}\``).toBeDefined()
						const { regressions, improvements } = compare(reference, counts)
						for (const { category, baseline, actual } of improvements) {
							console.log(
								`[e2e] ${caseName}: ${category} went down (${baseline} -> ${actual}). Lower the baseline: ${UPDATE_COMMAND}`
							)
						}
						expect(
							regressions.map(({ category, baseline, actual }) => `${category}: ${baseline} -> ${actual}`),
							`${caseName}: audit findings went UP (see the screenshot and run the audit to find them)`
						).toEqual([])
					})
				}
			})
		}
	}
}
