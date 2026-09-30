// Scans Svelte components for violations of the design rules in DESIGN.md.
// Usage: yarn design:scan [paths…] [--details] [--json] [--strict]
//   paths      files or folders to scan (default: apps/desktop/src)
//   --details  print every finding as file:line
//   --json     machine-readable output
//   --strict   exit with code 1 when anything is found (for CI once the debt is paid)
//   --locales  also check en.json/fr.json typography (always on when no path is given)
// A line containing `design-scan-ignore` is skipped (justify it in the same comment).
import { readFileSync, readdirSync, statSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), '..')
const args = process.argv.slice(2)
const flags = new Set(args.filter((a) => a.startsWith('--')))
const targets = args.filter((a) => !a.startsWith('--'))
const checkLocales = targets.length === 0 || flags.has('--locales')
if (targets.length === 0) targets.push('apps/desktop/src')

const PALETTE =
	'(?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)'

// Each rule: id, label, regex tested on template lines, optional `unless` regex that clears the line.
const RULES = [
	{
		id: 'palette',
		label: 'Tailwind palette class instead of a token',
		re: new RegExp(`\\b[a-z-]+-${PALETTE}-\\d{2,3}\\b`),
	},
	{ id: 'hex', label: 'Hexadecimal colour in a component', re: /#[0-9a-fA-F]{3,8}\b/ },
	{ id: 'dark-variant', label: 'dark: variant (the theme is [data-theme])', re: /\bdark:/ },
	{ id: 'text-arbitrary', label: 'Arbitrary font size text-[Npx]', re: /\btext-\[\d+(\.\d+)?px\]/ },
	{ id: 'transition-all', label: 'transition-all (list the properties)', re: /\btransition-all\b/ },
	{
		id: 'radius',
		label: 'Radius outside the system (rounded-xl to 3xl)',
		re: /\brounded(-[trblse]{1,2})?-(xl|2xl|3xl)\b/,
	},
	{
		id: 'blur-glow',
		label: 'Blur, glow or coloured shadow',
		re: /\bbackdrop-blur|\bblur-(sm|md|lg|xl|2xl|3xl)\b|drop-shadow-\[|shadow-\[0_0|\bshadow-2xl\b|\bshadow-[a-z]+-\d{3}/,
	},
	{
		id: 'gradient',
		label: 'Decorative gradient',
		re: /\bbg-gradient-to-|\bbg-linear-|\bbg-radial|linear-gradient\(|radial-gradient\(/,
	},
	{ id: 'font-weight', label: 'Font weight outside the system (black, extrabold)', re: /\bfont-(black|extrabold)\b/ },
	{ id: 'a11y-ignore', label: 'svelte-ignore a11y (inaccessible element hidden)', re: /svelte-ignore a11y/ },
	{
		id: 'outline-none',
		label: 'outline-none without a visible replacement focus',
		re: /\boutline-none\b/,
		unless: /focus(-visible)?:(ring|outline|border)/,
	},
	{
		id: 'infinite-motion',
		label: 'Infinite animation without motion-reduce',
		re: /\banimate-(spin|pulse|ping|bounce)\b/,
		unless: /motion-reduce:/,
	},
	{ id: 'z-arbitrary', label: 'Arbitrary z-index z-[N]', re: /\bz-\[\d+\]/ },
	{
		id: 'fixed-height',
		label: 'Viewport or fixed height (h-screen, h-[Npx] ≥ 100)',
		re: /\bh-screen\b|\bh-\[[1-9]\d{2,}px\]/,
	},
	{ id: 'emoji', label: 'Emoji in the template', re: /\p{Extended_Pictographic}/u },
	{
		id: 'hardcoded-text',
		label: 'Hard-coded visible string (heuristic)',
		re: />\s*[A-Za-zÀ-ÿ][^<>{}]*[A-Za-zÀ-ÿ]\s*</,
		unless: /\$translate\(|<(script|style)\b/,
	},
	{
		id: 'hardcoded-attr',
		label: 'Hard-coded text attribute (title, placeholder, aria-label, alt)',
		re: /\b(title|placeholder|aria-label|alt)="[^"{]*[A-Za-zÀ-ÿ]{3,}[^"]*"/,
	},
]

function walk(p, out) {
	const abs = path.resolve(root, p)
	const st = statSync(abs)
	if (st.isDirectory()) {
		for (const name of readdirSync(abs)) {
			if (name === 'node_modules' || name.startsWith('.')) continue
			walk(path.join(abs, name), out)
		}
	} else if (abs.endsWith('.svelte')) {
		out.push(abs)
	}
	return out
}

const findings = Object.fromEntries(RULES.map((r) => [r.id, []]))

for (const file of targets.flatMap((t) => walk(t, []))) {
	const rel = path.relative(root, file)
	let inBlock = false // inside <script> or <style>: only template lines are checked
	readFileSync(file, 'utf8')
		.split('\n')
		.forEach((line, i) => {
			if (/<(script|style)\b/.test(line)) inBlock = true
			if (inBlock) {
				if (/<\/(script|style)>/.test(line)) inBlock = false
				return
			}
			if (line.includes('design-scan-ignore') || /^\s*(<!--|\/\/)/.test(line)) return
			for (const rule of RULES) {
				if (rule.re.test(line) && !(rule.unless && rule.unless.test(line))) {
					findings[rule.id].push(`${rel}:${i + 1}`)
				}
			}
		})
}

// Locale typography: ASCII "..." instead of "…" in the two maintained locales.
findings['ellipsis'] = []
for (const locale of checkLocales ? ['en', 'fr'] : []) {
	const rel = `shared/i18n/locales/${locale}.json`
	readFileSync(path.join(root, rel), 'utf8')
		.split('\n')
		.forEach((line, i) => {
			if (/:\s*".*\.\.\./.test(line)) findings['ellipsis'].push(`${rel}:${i + 1}`)
		})
}
const labels = {
	...Object.fromEntries(RULES.map((r) => [r.id, r.label])),
	ellipsis: '"..." instead of "…" (en.json, fr.json)',
}

const total = Object.values(findings).reduce((n, list) => n + list.length, 0)

if (flags.has('--json')) {
	console.log(JSON.stringify({ total, findings }, null, 2))
} else {
	console.log(`design-scan: ${total} offending line(s) in ${targets.join(', ')}\n`)
	for (const [id, list] of Object.entries(findings)) {
		if (list.length === 0) continue
		console.log(`${String(list.length).padStart(5)}  ${id.padEnd(16)} ${labels[id]}`)
		if (flags.has('--details')) for (const loc of list) console.log(`         ${loc}`)
	}
	if (total === 0) console.log('No violations.')
}

if (flags.has('--strict') && total > 0) process.exit(1)
