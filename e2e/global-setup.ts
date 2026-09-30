import { chromium, type FullConfig } from '@playwright/test'
import { preparePage, harnessUrl, resetUnmockedReports, waitForApp } from './app'
import { resetMeasurements } from './baseline'

/**
 * Runs once, after the harness server is up. Clears the previous run's leftovers, then loads the app once so Vite
 * finishes any dependency pre-bundling (which reloads the page) before the first real test starts.
 */
export default async function globalSetup(config: FullConfig): Promise<void> {
	resetMeasurements()
	resetUnmockedReports()

	const baseURL = config.projects[0].use.baseURL
	const browser = await chromium.launch()
	try {
		const page = await browser.newPage({ baseURL })
		await preparePage(page)
		await page.goto(harnessUrl())
		await waitForApp(page)
	} finally {
		await browser.close()
	}
}
