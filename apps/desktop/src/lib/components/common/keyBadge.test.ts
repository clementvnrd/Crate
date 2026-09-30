import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'
import { CAMELOT_COLORS } from '$shared/utils/camelot'
import { keyBadgeAppearance } from './keyBadge'

const SRC = join(__dirname, '../../..')

function svelteFiles(dir: string): string[] {
	return readdirSync(dir).flatMap((entry) => {
		const path = join(dir, entry)
		if (statSync(path).isDirectory()) return svelteFiles(path)
		return path.endsWith('.svelte') ? [path] : []
	})
}

describe('keyBadgeAppearance', () => {
	it('takes its colours from the Camelot palette', () => {
		const look = keyBadgeAppearance('8A')
		expect(look.style).toBe(`background-color: ${CAMELOT_COLORS['8A'].bg}; color: ${CAMELOT_COLORS['8A'].text};`)
		expect(look.name).toBe(CAMELOT_COLORS['8A'].name)
		expect(look.label).toBe('8A')
	})

	it('adds the palette border shade for the badges that draw a border', () => {
		const info = CAMELOT_COLORS['8A']
		expect(keyBadgeAppearance('8A', { bordered: true }).style).toBe(
			`background-color: ${info.bg}; color: ${info.text}; border-color: ${info.border};`
		)
	})

	it('maps standard notation onto the same colours', () => {
		expect(keyBadgeAppearance('Am').style).toBe(keyBadgeAppearance('8A').style)
		expect(keyBadgeAppearance('Am').label).toBe('8A')
	})

	it('zero-pads the Camelot notation when asked', () => {
		expect(keyBadgeAppearance('5A', { zeroPad: true }).label).toBe('05A')
	})

	it('shows a caller label (the notation setting) with the Camelot colours', () => {
		const look = keyBadgeAppearance('8A', { label: 'Am' })
		expect(look.label).toBe('Am')
		expect(look.style).toBe(keyBadgeAppearance('8A').style)
	})

	it('is neutral for a key outside the wheel', () => {
		const look = keyBadgeAppearance('H major')
		expect(look.style).toBeNull()
		expect(look.name).toBeNull()
		expect(look.label).toBe('H MAJOR')
	})

	it('is neutral when the key was not analysed by Mixed In Key', () => {
		const look = keyBadgeAppearance('8A', { analysis: 'other' })
		expect(look.style).toBeNull()
		expect(look.mikRing).toBe(false)
		expect(look.name).toBe(CAMELOT_COLORS['8A'].name)
	})

	it('draws the Mixed In Key ring only on a coloured Mixed In Key badge', () => {
		expect(keyBadgeAppearance('8A', { analysis: 'mik' }).mikRing).toBe(true)
		expect(keyBadgeAppearance('H major', { analysis: 'mik' }).mikRing).toBe(false)
		expect(keyBadgeAppearance('8A').mikRing).toBe(false)
	})

	it('shows a dash when there is no key', () => {
		for (const value of [null, undefined, '', '   ']) {
			const look = keyBadgeAppearance(value)
			expect(look.empty).toBe(true)
			expect(look.label).toBe('-')
			expect(look.style).toBeNull()
		}
	})
})

describe('data badges are never rebuilt by hand (D11)', () => {
	// Every Camelot badge is a KeyBadge (whose logic, keyBadge.ts, is the one caller of getCamelotColor) and every
	// energy badge an EnergyBadge, so each palette is applied in one place. A component that needs a key or energy
	// colour for something other than a badge is listed here with the reason.
	const allowed: Record<string, string[]> = {
		// The harmonic wheel fills each key's play-share bar with the key's colour (a chart, not a badge).
		getCamelotColor: ['lib/components/stats/StatsHarmonicWheel.svelte'],
		getEnergyInfo: ['lib/components/common/EnergyBadge.svelte'],
	}

	for (const [helper, files] of Object.entries(allowed)) {
		it(`no component calls ${helper}() except the allowed ones`, () => {
			const offenders = svelteFiles(SRC)
				.filter((file) => readFileSync(file, 'utf8').includes(`${helper}(`))
				.map((file) => file.slice(SRC.length + 1))
				.filter((file) => !files.includes(file))
			expect(offenders).toEqual([])
		})
	}
})
