// Recomputes the progress table of suivi/AVANCEMENT.md from its checkboxes.
// Usage: yarn suivi
import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

const file = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', 'suivi', 'AVANCEMENT.md')
const text = readFileSync(file, 'utf8')

const steps = []
let current = null
for (const line of text.split('\n')) {
	const heading = line.match(/^### (Étape \d+ — .+)$/)
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
const done = steps.reduce((n, s) => n + s.done, 0)
const total = steps.reduce((n, s) => n + s.total, 0)
const rows = steps.map((s) => `| ${s.name} | ${s.done} / ${s.total} | ${bar(s.done, s.total)} |`)
const table = [
	`**Progression globale : ${done} / ${total} défauts corrigés (${Math.round((done / total) * 100)} %)**`,
	'',
	'| Étape | Corrigés | Progression |',
	'| --- | --- | --- |',
	...rows,
].join('\n')

const updated = text.replace(
	/<!-- progression:start -->[\s\S]*<!-- progression:end -->/,
	`<!-- progression:start -->\n${table}\n<!-- progression:end -->`
)
writeFileSync(file, updated)
console.log(`suivi: ${done}/${total} défauts corrigés`)
