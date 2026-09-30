import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { AUDIT_CATEGORIES, type AuditCategory, type AuditCounts } from './audit'

// The audit ratchet. `baseline.json` holds, per case (`<view>-<theme>-<width>-<lang>`), how many findings each
// audit category had when the baseline was last written. A test fails when a count goes UP; a count that goes
// DOWN is reported so the baseline can be lowered. Design debt can only shrink.

export const BASELINE_PATH = fileURLToPath(new URL('./baseline.json', import.meta.url))
/** Where every test leaves its counts so the teardown can merge them (tests run in parallel workers). */
const MEASUREMENT_DIR = fileURLToPath(new URL('./screenshots/.measurements/', import.meta.url))

export const UPDATE_COMMAND = 'yarn test:e2e:update-baseline'

export interface BaselineFile {
	/** Explains the file to whoever opens it (JSON has no comments). */
	about: string
	cases: Record<string, AuditCounts>
}

const ABOUT =
	'Audit findings per case (view, theme, window width, language). `yarn test:e2e` fails when a count rises above ' +
	'its value here; after fixing findings, lower it with `yarn test:e2e:update-baseline`. Do not edit by hand.'

export function readBaseline(): BaselineFile {
	if (!existsSync(BASELINE_PATH)) return { about: ABOUT, cases: {} }
	return JSON.parse(readFileSync(BASELINE_PATH, 'utf-8')) as BaselineFile
}

export function writeBaseline(cases: Record<string, AuditCounts>): void {
	const sorted = Object.fromEntries(Object.entries(cases).sort(([a], [b]) => a.localeCompare(b)))
	writeFileSync(BASELINE_PATH, `${JSON.stringify({ about: ABOUT, cases: sorted }, null, '\t')}\n`)
}

export interface Comparison {
	regressions: { category: AuditCategory; baseline: number; actual: number }[]
	improvements: { category: AuditCategory; baseline: number; actual: number }[]
}

export function compare(baseline: AuditCounts, actual: AuditCounts): Comparison {
	const result: Comparison = { regressions: [], improvements: [] }
	for (const category of AUDIT_CATEGORIES) {
		if (actual[category] > baseline[category]) {
			result.regressions.push({ category, baseline: baseline[category], actual: actual[category] })
		} else if (actual[category] < baseline[category]) {
			result.improvements.push({ category, baseline: baseline[category], actual: actual[category] })
		}
	}
	return result
}

// -----------------------------------------------------------------------------
// Measurements left by the tests for the global teardown
// -----------------------------------------------------------------------------

export function resetMeasurements(): void {
	rmSync(MEASUREMENT_DIR, { recursive: true, force: true })
	mkdirSync(MEASUREMENT_DIR, { recursive: true })
}

export function saveMeasurement(caseName: string, counts: AuditCounts): void {
	mkdirSync(MEASUREMENT_DIR, { recursive: true })
	writeFileSync(`${MEASUREMENT_DIR}${caseName}.json`, JSON.stringify(counts))
}

export function loadMeasurements(): Record<string, AuditCounts> {
	if (!existsSync(MEASUREMENT_DIR)) return {}
	return Object.fromEntries(
		readdirSync(MEASUREMENT_DIR)
			.filter((file) => file.endsWith('.json'))
			.map((file) => [
				file.replace(/\.json$/, ''),
				JSON.parse(readFileSync(`${MEASUREMENT_DIR}${file}`, 'utf-8')) as AuditCounts,
			])
	)
}
