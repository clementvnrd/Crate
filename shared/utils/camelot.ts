/**
 * Official Camelot Wheel 11 Pro Color Definitions
 * Matches Mixed In Key 11 Pro and DJ.Studio Camelot notation palette
 */

export interface CamelotColorInfo {
	bg: string
	text: string
	border: string
	dot: string
	name: string
}

export const CAMELOT_COLORS: Record<string, CamelotColorInfo> = {
	// 1: Cyan / Turquoise
	'1A': { bg: '#50EBCB', text: '#0B2D26', border: '#2FDDB8', dot: '#00E5C0', name: 'Ab minor / G# minor' },
	'1B': { bg: '#1AE8C2', text: '#062921', border: '#05D6AF', dot: '#00E5C0', name: 'B major' },

	// 2: Fresh Green
	'2A': { bg: '#7AEC9F', text: '#0D3319', border: '#54E384', dot: '#33E066', name: 'Eb minor / D# minor' },
	'2B': { bg: '#33E066', text: '#052A0E', border: '#1AC94F', dot: '#33E066', name: 'F# major / Gb major' },

	// 3: Lime Green
	'3A': { bg: '#B6F385', text: '#213809', border: '#9FE960', dot: '#88E838', name: 'Bb minor / A# minor' },
	'3B': { bg: '#88E838', text: '#152E03', border: '#72CE27', dot: '#88E838', name: 'Db major / C# major' },

	// 4: Warm Yellow / Gold
	'4A': { bg: '#FFDD7E', text: '#3E2E02', border: '#FFD154', dot: '#FFCA36', name: 'F minor' },
	'4B': { bg: '#FFCA36', text: '#352400', border: '#F5B916', dot: '#FFCA36', name: 'Ab major / G# major' },

	// 5: Vibrant Orange / Peach
	'5A': { bg: '#FFB899', text: '#3E1908', border: '#FFA078', dot: '#FF8C4B', name: 'C minor' },
	'5B': { bg: '#FF8C4B', text: '#3B1502', border: '#F3752D', dot: '#FF8C4B', name: 'Eb major / D# major' },

	// 6: Coral / Salmon Red
	'6A': { bg: '#FFA1AC', text: '#3E0C13', border: '#FF8593', dot: '#FF6073', name: 'G minor' },
	'6B': { bg: '#FF6073', text: '#3A060E', border: '#F4455B', dot: '#FF6073', name: 'Bb major / A# major' },

	// 7: Magenta / Rose Pink
	'7A': { bg: '#F99CCB', text: '#3B0924', border: '#F77EB9', dot: '#F554A4', name: 'D minor' },
	'7B': { bg: '#F554A4', text: '#34041D', border: '#E7378D', dot: '#F554A4', name: 'F major' },

	// 8: Purple / Violet
	'8A': { bg: '#ECA0F3', text: '#360A3C', border: '#E482EC', dot: '#DB57E6', name: 'A minor' },
	'8B': { bg: '#DB57E6', text: '#300435', border: '#C939D5', dot: '#DB57E6', name: 'C major' },

	// 9: Deep Violet / Indigo
	'9A': { bg: '#CBA5FF', text: '#240C44', border: '#B989FF', dot: '#AA5EFF', name: 'E minor' },
	'9B': { bg: '#AA5EFF', text: '#1E063B', border: '#9644F5', dot: '#AA5EFF', name: 'G major' },

	// 10: Periwinkle / Soft Blue
	'10A': { bg: '#ADC1FF', text: '#0B1A45', border: '#8DA9FF', dot: '#7290FF', name: 'B minor' },
	'10B': { bg: '#7290FF', text: '#061239', border: '#5476F8', dot: '#7290FF', name: 'D major' },

	// 11: Sky Blue
	'11A': { bg: '#8FE1FF', text: '#042738', border: '#68D4FF', dot: '#38C8FF', name: 'F# minor / Gb minor' },
	'11B': { bg: '#38C8FF', text: '#022131', border: '#1EB9F5', dot: '#38C8FF', name: 'A major' },

	// 12: Bright Aqua / Cyan
	'12A': { bg: '#5CEBEB', text: '#032B2B', border: '#36E0E0', dot: '#00E5E5', name: 'Db minor / C# minor' },
	'12B': { bg: '#00E5E5', text: '#012828', border: '#00CDCD', dot: '#00E5E5', name: 'E major' },
}

const STANDARD_TO_CAMELOT: Record<string, string> = {
	ABM: '1A',
	'G#M': '1A',
	B: '1B',
	EBM: '2A',
	'D#M': '2A',
	'F#': '2B',
	GB: '2B',
	BBM: '3A',
	'A#M': '3A',
	DB: '3B',
	'C#': '3B',
	FM: '4A',
	AB: '4B',
	'G#': '4B',
	CM: '5A',
	EB: '5B',
	'D#': '5B',
	GM: '6A',
	BB: '6B',
	'A#': '6B',
	DM: '7A',
	F: '7B',
	AM: '8A',
	C: '8B',
	EM: '9A',
	G: '9B',
	BM: '10A',
	D: '10B',
	'F#M': '11A',
	GBM: '11A',
	A: '11B',
	'C#M': '12A',
	DBM: '12A',
	E: '12B',
}

/**
 * Returns Camelot Color Info for a key string (either Camelot notation '8A' or standard 'Am')
 */
export function getCamelotColor(key: string | null | undefined): CamelotColorInfo | null {
	if (!key) return null
	const clean = key.trim().toUpperCase()
	if (!clean) return null
	const camelot = STANDARD_TO_CAMELOT[clean] ?? clean
	return CAMELOT_COLORS[camelot] ?? null
}

/**
 * Formats a key to Camelot notation, with optional leading zero (e.g. '05A' vs '5A')
 */
export function formatCamelotKey(key: string | null | undefined, zeroPad = false): string {
	if (!key) return '-'
	const clean = key.trim().toUpperCase()
	if (!clean) return '-'
	const camelot = STANDARD_TO_CAMELOT[clean] ?? clean
	if (zeroPad && camelot.length === 2 && /^[1-9][AB]$/i.test(camelot)) {
		return `0${camelot}`
	}
	return camelot
}

/**
 * Calculates harmonically compatible Camelot keys:
 * - Exact key (e.g. 8A)
 * - Relative major/minor (e.g. 8B)
 * - Adjacent keys (+1 / -1 on the wheel, e.g. 7A, 9A)
 * - Energy boost (+2 on the wheel, e.g. 10A)
 */
export function getHarmonicKeys(key: string | null | undefined, includeEnergyBoost = true): string[] {
	if (!key) return []
	const clean = key.trim().toUpperCase()
	const camelot = STANDARD_TO_CAMELOT[clean] ?? clean
	const match = camelot.match(/^([1-9]|1[0-2])([AB])$/i)
	if (!match) return [camelot]

	const num = parseInt(match[1], 10)
	const letter = match[2].toUpperCase()
	const oppositeLetter = letter === 'A' ? 'B' : 'A'

	const plus1 = num === 12 ? 1 : num + 1
	const minus1 = num === 1 ? 12 : num - 1
	const plus2 = num === 11 ? 1 : num === 12 ? 2 : num + 2

	const keys = [
		`${num}${letter}`, // Same key
		`${num}${oppositeLetter}`, // Relative major/minor
		`${minus1}${letter}`, // Adjacent down
		`${plus1}${letter}`, // Adjacent up
	]

	if (includeEnergyBoost) {
		keys.push(`${plus2}${letter}`) // Energy boost (+2)
	}

	return keys
}

/**
 * Checks if two tracks/keys are harmonically compatible
 */
export function isHarmonicallyCompatible(key1: string | null | undefined, key2: string | null | undefined): boolean {
	if (!key1 || !key2) return false
	const compatible = getHarmonicKeys(key1)
	const clean2 = key2.trim().toUpperCase()
	const camelot2 = STANDARD_TO_CAMELOT[clean2] ?? clean2
	return compatible.includes(camelot2)
}
