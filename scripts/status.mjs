// Recomputes the progress table of tracking/STATUS.md from its checkboxes.
// Usage: yarn status (from the repository root)
import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

const file = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', 'tracking', 'STATUS.md')
const text = readFileSync(file, 'utf8')

const steps = []
let current = null
for (const line of text.split('\n')) {
	const heading = line.match(/^### (Step \d+ — .+)$/)
	if (heading) {
		current = { name: heading[1], done: 0, total: 0 }
		steps.push(current)
		continue
	}
	if (line.startsWith('## ')) current = null
	const box = line.match(/^- \[([ x])\] \*\*[A-Z]\d+\*\*/)
	if (current && box) {
		current.total++
		if (box[1] === 'x') current.done++
	}
}

const bar = (done, total) => {
	const filled = total ? Math.round((done / total) * 10) : 0
	return '█'.repeat(filled) + '░'.repeat(10 - filled)
}
if (steps.length === 0) {
	console.error('status: no "### Step N — …" heading found in tracking/STATUS.md')
	process.exit(1)
}
if (!/<!-- progress:start -->[\s\S]*<!-- progress:end -->/.test(text)) {
	console.error('status: <!-- progress:start --> / <!-- progress:end --> markers missing in tracking/STATUS.md')
	process.exit(1)
}

const done = steps.reduce((n, s) => n + s.done, 0)
const total = steps.reduce((n, s) => n + s.total, 0)
const rows = steps.map((s) => `| ${s.name} | ${s.done} / ${s.total} | ${bar(s.done, s.total)} |`)
const table = [
	`**Overall progress: ${done} / ${total} defects fixed (${Math.round((done / total) * 100)}%)**`,
	'',
	'| Step | Fixed | Progress |',
	'| --- | --- | --- |',
	...rows,
].join('\n')

const updated = text.replace(
	/<!-- progress:start -->[\s\S]*<!-- progress:end -->/,
	`<!-- progress:start -->\n${table}\n<!-- progress:end -->`
)
writeFileSync(file, updated)
console.log(`status: ${done}/${total} defects fixed`)
