import { readdirSync, readFileSync, existsSync } from 'node:fs'
import { UNMOCKED_DIR } from './app'
import { compare, loadMeasurements, readBaseline, UPDATE_COMMAND, writeBaseline } from './baseline'

/**
 * Runs once after every test. With `UPDATE_BASELINE=1` it rewrites `baseline.json` from what this run measured
 * (only the cases that ran, so a filtered run does not drop the others); otherwise it reports the cases whose
 * findings went down, so the baseline can be lowered, and the harness commands that are still unmocked.
 */
export default function globalTeardown(): void {
	const measured = loadMeasurements()
	const baseline = readBaseline()

	if (process.env.UPDATE_BASELINE === '1') {
		writeBaseline({ ...baseline.cases, ...measured })
		console.log(`\n[e2e] baseline.json updated: ${Object.keys(measured).length} case(s) written.`)
		return
	}

	const improved = Object.entries(measured).filter(([name, counts]) => {
		const reference = baseline.cases[name]
		return reference !== undefined && compare(reference, counts).improvements.length > 0
	})
	if (improved.length > 0) {
		console.log(
			`\n[e2e] ${improved.length} case(s) have fewer audit findings than the baseline: lower it with \`${UPDATE_COMMAND}\`.`
		)
	}

	if (existsSync(UNMOCKED_DIR)) {
		const lines = readdirSync(UNMOCKED_DIR)
			.filter((file) => file.endsWith('.json'))
			.map((file) => ({
				view: file.replace(/\.json$/, ''),
				commands: JSON.parse(readFileSync(`${UNMOCKED_DIR}${file}`, 'utf-8')) as string[],
			}))
			.filter(({ commands }) => commands.length > 0)
			.map(({ view, commands }) => `  ${view}: ${commands.join(', ')}`)
		if (lines.length > 0) console.log(`\n[e2e] harness commands still unmocked, per view:\n${lines.join('\n')}`)
	}
}
