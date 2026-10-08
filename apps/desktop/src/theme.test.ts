import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

// Guards for the colour tokens of style.css (DESIGN.md, "Graphic charter", owner decision CRA-141):
// the text on an accent fill is chosen per accent for contrast, every family token exists in both themes,
// and every light-theme text token is readable on the light surfaces.

const css = readFileSync(join(__dirname, 'style.css'), 'utf-8')
const tailwindTheme = readFileSync(join(__dirname, '../../../node_modules/tailwindcss/theme.css'), 'utf-8')

type Rgb = [number, number, number]

function hexToRgb(hex: string): Rgb {
	const h = hex.replace('#', '')
	const full = h.length === 3 ? [...h].map((c) => c + c).join('') : h.slice(0, 6)
	return [0, 2, 4].map((i) => parseInt(full.slice(i, i + 2), 16) / 255) as Rgb
}

/** oklch(L% C H) → sRGB, the conversion browsers apply to Tailwind 4's palette. */
function oklchToRgb(l: number, c: number, h: number): Rgb {
	const a = c * Math.cos((h * Math.PI) / 180)
	const b = c * Math.sin((h * Math.PI) / 180)
	const l_ = (l + 0.3963377774 * a + 0.2158037573 * b) ** 3
	const m_ = (l - 0.1055613458 * a - 0.0638541728 * b) ** 3
	const s_ = (l - 0.0894841775 * a - 1.291485548 * b) ** 3
	const linear = [
		4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_,
		-1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_,
		-0.0041960863 * l_ - 0.7034186147 * m_ + 1.707614701 * s_,
	]
	return linear.map((v) => {
		const clamped = Math.min(1, Math.max(0, v))
		return clamped <= 0.0031308 ? 12.92 * clamped : 1.055 * clamped ** (1 / 2.4) - 0.055
	}) as Rgb
}

function luminance([r, g, b]: Rgb): number {
	const f = (v: number) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4)
	return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b)
}

function contrast(x: Rgb, y: Rgb): number {
	const [a, b] = [luminance(x), luminance(y)].sort((p, q) => q - p)
	return (a + 0.05) / (b + 0.05)
}

/** Declarations of the first rule whose selector is exactly `selector`. */
function block(selector: string, from = 0): Record<string, string> {
	const start = css.indexOf(`${selector} {`, from)
	if (start < 0) throw new Error(`No "${selector}" block in style.css`)
	const body = css.slice(css.indexOf('{', start) + 1, css.indexOf('}', start))
	const declarations: Record<string, string> = {}
	for (const match of body.matchAll(/(--[\w-]+):\s*([^;]+);/g)) declarations[match[1]] = match[2].trim()
	return declarations
}

/** The family blocks come after the "Colour families" banner, the neutral theme blocks before it. */
const familiesStart = css.indexOf('Colour families (DESIGN.md, "Graphic charter"; owner decision')
const darkFamilies = block(":root,\n[data-theme='dark']", familiesStart)
const lightFamilies = block("[data-theme='light']", familiesStart)
const darkNeutrals = block(":root,\n[data-theme='dark']")
const lightNeutrals = block("[data-theme='light']")

function resolve(value: string, scope: Record<string, string>): Rgb {
	const tailwind = value.match(/^var\(--color-([\w-]+)\)$/)
	if (tailwind) {
		const declaration = tailwindTheme.match(
			new RegExp(`--color-${tailwind[1]}: oklch\\(([\\d.]+)% ([\\d.]+) ([\\d.]+)\\)`)
		)
		if (!declaration) throw new Error(`Unknown Tailwind colour ${tailwind[1]}`)
		return oklchToRgb(Number(declaration[1]) / 100, Number(declaration[2]), Number(declaration[3]))
	}
	const reference = value.match(/^var\((--[\w-]+)\)$/)
	if (reference) return resolve(scope[reference[1]], scope)
	if (value.startsWith('#')) return hexToRgb(value)
	throw new Error(`Cannot resolve ${value}`)
}

const BLACK: Rgb = [0, 0, 0]
const WHITE: Rgb = [1, 1, 1]

describe('text on an accent fill (--brand-on)', () => {
	const accents = [...css.matchAll(/\[data-accent='(\w+)'\] \{/g)].map((m) => m[1])

	it('covers the ten accents and the default', () => {
		expect(accents).toHaveLength(10)
		expect(block(':root:not([data-accent])')['--brand-on']).toBeDefined()
	})

	it.each(accents)('%s: black or white, whichever reads better, at 4.5:1 or more', (accent) => {
		const declarations = block(`[data-accent='${accent}']`)
		const fill = hexToRgb(declarations['--brand-primary'])
		const on = hexToRgb(declarations['--brand-on'])
		const best = contrast(fill, BLACK) >= contrast(fill, WHITE) ? BLACK : WHITE
		expect(on).toEqual(best)
		expect(contrast(fill, on)).toBeGreaterThanOrEqual(4.5)
	})

	it.each(accents)('%s: the same text still reads at 4.5:1 or more on the hover fill (--brand-hover)', (accent) => {
		const declarations = block(`[data-accent='${accent}']`)
		const hover = hexToRgb(declarations['--brand-hover'])
		const on = hexToRgb(declarations['--brand-on'])
		expect(contrast(hover, on)).toBeGreaterThanOrEqual(4.5)
	})
})

describe('colour family tokens', () => {
	const familyTokens = Object.keys(darkFamilies)

	it('declares every family token in both themes', () => {
		expect(familyTokens.length).toBeGreaterThan(40)
		expect(Object.keys(lightFamilies).sort()).toEqual([...familyTokens].sort())
	})

	it('registers every family token as a Tailwind colour', () => {
		for (const token of familyTokens) {
			expect(css, `${token} is not registered in @theme inline`).toContain(`--color-${token.slice(2)}: var(${token});`)
		}
	})

	const textTokens = familyTokens.filter((token) => /-text(-[a-z]+)?$/.test(token))

	it.each(textTokens)('%s reads at 4.5:1 or more on the light surfaces', (token) => {
		const colour = resolve(lightFamilies[token], { ...lightNeutrals, ...lightFamilies })
		for (const surface of ['--surface-0', '--surface-1', '--surface-2']) {
			expect(contrast(colour, hexToRgb(lightNeutrals[surface]))).toBeGreaterThanOrEqual(4.5)
		}
	})

	// Dark values frozen by the owner's decision (CRA-141: the dark theme does not change) that stay under 4.5:1 on
	// `surface-2`, listed in DESIGN.md "Known gaps". They must still pass on the surfaces they sit on most.
	const darkSurface2Gaps: Record<string, number> = {
		'--danger-text-muted': 3.85, // 3.90:1, upstream red-500 text (danger menu items, ghost-danger buttons, error text)
	}

	it.each(textTokens)('%s reads at 4.5:1 or more on the dark surfaces', (token) => {
		const colour = resolve(darkFamilies[token], { ...darkNeutrals, ...darkFamilies })
		for (const surface of ['--surface-0', '--surface-1', '--surface-2']) {
			const floor = surface === '--surface-2' && token in darkSurface2Gaps ? darkSurface2Gaps[token] : 4.5
			expect(contrast(colour, hexToRgb(darkNeutrals[surface]))).toBeGreaterThanOrEqual(floor)
		}
	})

	// Toolbar count badges keep white text in both themes; the light theme darkens their fill instead (D3).
	it.each(['--danger-badge', '--beatport-badge'])('white text reads at 4.5:1 or more on the light %s', (token) => {
		const fill = resolve(lightFamilies[token], { ...lightNeutrals, ...lightFamilies })
		expect(contrast(fill, WHITE)).toBeGreaterThanOrEqual(4.5)
	})
})
