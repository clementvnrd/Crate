import type { Locator, Page } from '@playwright/test'
import { mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

// Helpers shared by the specs: opening the app in the harness, reaching each view, waiting for a steady
// state, and collecting what the page reports.

export const SCREENSHOT_DIR = fileURLToPath(new URL('./screenshots/', import.meta.url))
export const UNMOCKED_DIR = fileURLToPath(new URL('./screenshots/unmocked/', import.meta.url))

// -----------------------------------------------------------------------------
// Page setup
// -----------------------------------------------------------------------------

/** Everything a page needs before it opens the app: no motion, and no dependency on the network. */
export async function preparePage(page: Page): Promise<void> {
	await page.emulateMedia({ reducedMotion: 'reduce' })
	await blockWebFonts(page)
}

/**
 * The app loads its fonts from Google Fonts. Answering with an empty stylesheet keeps the runs offline-safe and
 * identical everywhere: text is measured in the system sans-serif fallback, never in whichever font happened to
 * load. (The harness in a normal browser still loads the real fonts.)
 */
async function blockWebFonts(page: Page): Promise<void> {
	await page.route(/https:\/\/fonts\.(googleapis|gstatic)\.com\//, (route) =>
		route.fulfill({ status: 200, contentType: 'text/css', body: '' })
	)
}

interface AllowedConsoleError {
	pattern: RegExp
	reason: string
}

/**
 * `console.error` messages that do not fail a test. Keep this list short and justified: every entry hides a
 * defect from the run. (Empty on purpose: the harness and the app currently log no error.)
 */
export const ALLOWED_CONSOLE_ERRORS: AllowedConsoleError[] = []

/** Collects uncaught exceptions and `console.error` calls (minus the allow-list) for the life of the page. */
export function watchErrors(page: Page): string[] {
	const errors: string[] = []
	page.on('pageerror', (error) => errors.push(`uncaught: ${error.message}`))
	page.on('console', (message) => {
		if (message.type() !== 'error') return
		const text = message.text()
		if (ALLOWED_CONSOLE_ERRORS.some((allowed) => allowed.pattern.test(text))) return
		errors.push(`console.error: ${text}`)
	})
	return errors
}

export interface HarnessUrlOptions {
	theme?: 'dark' | 'light' | 'system'
	lang?: string
	accent?: string
	/** Raw harness parameters, e.g. `{ beatport: 'out', library: 'empty' }`. */
	params?: Record<string, string>
}

export function harnessUrl({
	theme = 'dark',
	lang = 'en',
	accent = 'blue',
	params = {},
}: HarnessUrlOptions = {}): string {
	return `/?${new URLSearchParams({ theme, lang, accent, ...params }).toString()}`
}

/** Wait until the layout is shown (splash gone, settings loaded) and nothing finite is still animating. */
export async function waitForApp(page: Page): Promise<void> {
	await page.locator('#wizard-view-switcher').waitFor({ state: 'visible', timeout: 15_000 })
	try {
		await page.waitForFunction(
			() => {
				const layout = document.querySelector('#wizard-view-switcher')?.closest('.transition-opacity')
				const splash = document.querySelector('[class~="z-[9999]"]')
				return layout !== null && layout !== undefined && getComputedStyle(layout).opacity === '1' && splash === null
			},
			undefined,
			{ timeout: 10_000 }
		)
	} catch (error) {
		if (await page.getByText('Something went wrong').isVisible()) {
			throw new Error('The app shows its crash screen instead of the library.')
		}
		throw error
	}
	await settle(page)
}

/** Wait until no finite CSS/Web animation is running (modal scale-in, fades…). Looping animations are ignored. */
export async function settle(page: Page): Promise<void> {
	await page.waitForTimeout(150)
	await page.waitForFunction(
		() =>
			document.getAnimations().every((animation) => {
				if (animation.playState !== 'running' && !animation.pending) return true
				const endTime = animation.effect?.getComputedTiming().endTime
				return typeof endTime === 'number' && !Number.isFinite(endTime)
			}),
		undefined,
		{ timeout: 5_000 }
	)
}

// -----------------------------------------------------------------------------
// Views
// -----------------------------------------------------------------------------

export interface ViewSpec {
	id: string
	/** Go from the library (where the app starts) to the view. */
	open: (page: Page) => Promise<void>
	/** Something only that view shows, identical in English and French. */
	landmark: (page: Page) => Locator
}

// Toolbar buttons are named through i18n (`nav.toolbar.*`), so the names below cover English and French. The
// Upgrader button keeps its product name in both languages.
export const VIEWS: ViewSpec[] = [
	{
		id: 'library',
		open: async () => {},
		landmark: (page) => page.getByText('Afterglow Protocol', { exact: true }).first(),
	},
	{
		id: 'player',
		open: (page) => page.locator('#wizard-view-switcher button', { hasText: 'Player' }).click(),
		landmark: (page) => page.getByText('Afterglow Protocol (Club Mix)').first(),
	},
	{
		id: 'beatport',
		open: (page) => page.locator('#wizard-view-switcher button', { hasText: 'Beatport' }).click(),
		landmark: (page) => page.getByText('Peak Time Techno Top 100'),
	},
	{
		id: 'discovery',
		// The button is named by the i18n key `nav.discovery`, so its name depends on the language.
		open: (page) => page.getByRole('button', { name: /^(Discovery|Découvertes)$/ }).click(),
		landmark: (page) => page.getByText('Signal Loss EP').first(),
	},
	{
		id: 'pulse',
		open: (page) => page.getByRole('button', { name: /^(Statistics|Statistiques)$/ }).click(),
		landmark: (page) => page.getByRole('heading', { name: 'Crate Pulse & Stats' }),
	},
	{
		id: 'settings',
		// The gear button is named by the i18n key `settings.title` (register defect D10). Clicking it, rather than the
		// Cmd/Ctrl+, shortcut, keeps the screenshot as a mouse user sees it (no keyboard focus ring in the dialog).
		open: (page) => page.getByRole('button', { name: /^(Settings|Paramètres)$/ }).click(),
		landmark: (page) => page.locator('dialog[open]'),
	},
	{
		id: 'duplicates',
		open: (page) => page.getByRole('button', { name: /^(Duplicate management|Gestion des doublons)$/ }).click(),
		landmark: (page) => page.locator('dialog[open]').getByText('Duplicate Killer'),
	},
	{
		id: 'upgrader',
		open: (page) => page.getByRole('button', { name: 'Beatport Quality Upgrader' }).click(),
		landmark: (page) => page.locator('dialog[open]').getByText('Beatport Quality Upgrader').first(),
	},
]

// -----------------------------------------------------------------------------
// What the harness reports
// -----------------------------------------------------------------------------

/** Commands the app called that have no precise handler in the harness (see harness/README.md). */
export async function unmockedCommands(page: Page): Promise<string[]> {
	return page.evaluate(() => {
		const harness = (window as unknown as { __harness?: { unmocked: Set<string> } }).__harness
		return harness ? [...harness.unmocked].sort() : []
	})
}

export function resetUnmockedReports(): void {
	rmSync(UNMOCKED_DIR, { recursive: true, force: true })
}

/** Leave the unmocked commands of a view in `e2e/screenshots/unmocked/<view>.json` for the run's report. */
export function saveUnmockedReport(view: string, commands: string[]): void {
	mkdirSync(UNMOCKED_DIR, { recursive: true })
	writeFileSync(`${UNMOCKED_DIR}${view}.json`, JSON.stringify(commands))
}
