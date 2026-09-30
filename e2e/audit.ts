import type { Page } from '@playwright/test'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

// Loads the in-page audit of the visual-check skill instead of copying it: the script is the single source of
// truth for what is measured (contrast, small text, unnamed controls, overlaps…), for `playwright-cli run-code`
// as well as for these tests.

const AUDIT_PATH = fileURLToPath(new URL('../.claude/skills/crate-visual-check/scripts/ui-audit.js', import.meta.url))

/** The report the script returns (only the fields the ratchet reads). */
export interface AuditReport {
	error?: string
	window: string
	lowContrast: string[]
	smallText: string[]
	unnamedControls: string[]
	pointerOnly: string[]
	overlaps: string[]
	outOfWindow: string[]
	crushedColumns: string[]
	pageOverflowX: boolean
}

/** The categories that are counted and held by the ratchet (`pageOverflowX` counts 0 or 1). */
export const AUDIT_CATEGORIES = [
	'lowContrast',
	'smallText',
	'unnamedControls',
	'pointerOnly',
	'overlaps',
	'outOfWindow',
	'crushedColumns',
	'pageOverflowX',
] as const

export type AuditCategory = (typeof AUDIT_CATEGORIES)[number]

export type AuditCounts = Record<AuditCategory, number>

type AuditFunction = (page: Page) => Promise<AuditReport>

let audit: AuditFunction | undefined

/**
 * The script is a bare `async (page) => {…}` expression. Its lists stop at 40 entries (`const MAX = 40`), which
 * is right for reading a report but would hide any growth once a category reaches 40, so the cap is lifted in
 * memory. The file itself is not modified; if the line ever changes, this fails loudly rather than silently
 * counting up to 40 only.
 */
function loadAudit(): AuditFunction {
	const source = readFileSync(AUDIT_PATH, 'utf-8')
	const uncapped = source.replace(/const MAX = \d+/, 'const MAX = Number.MAX_SAFE_INTEGER')
	if (uncapped === source) {
		throw new Error(`Could not lift the report cap: no "const MAX = <number>" line in ${AUDIT_PATH}`)
	}
	return new Function(`return (${uncapped}\n)`)() as AuditFunction
}

/** Run the audit on the page as it is now and return the full report. */
export async function runAudit(page: Page): Promise<AuditReport> {
	audit ??= loadAudit()
	const report = await audit(page)
	if (report.error) throw new Error(`UI audit failed: ${report.error}`)
	return report
}

export function countFindings(report: AuditReport): AuditCounts {
	return {
		lowContrast: report.lowContrast.length,
		smallText: report.smallText.length,
		unnamedControls: report.unnamedControls.length,
		pointerOnly: report.pointerOnly.length,
		overlaps: report.overlaps.length,
		outOfWindow: report.outOfWindow.length,
		crushedColumns: report.crushedColumns.length,
		pageOverflowX: report.pageOverflowX ? 1 : 0,
	}
}
