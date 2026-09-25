/**
 * Energy Level Visual Definition (1-10 scale)
 * Provides gradual color progression and dynamic symbols based on intensity
 */

export interface EnergyInfo {
	level: number
	symbol: string
	descriptor: string
	color: string
	bg: string
	border: string
	glow: string
}

export const ENERGY_CONFIG: Record<number, EnergyInfo> = {
	1: {
		level: 1,
		symbol: '🌱',
		descriptor: 'Très calme / Ambient',
		color: '#38BDF8', // Sky 400
		bg: 'rgba(14, 165, 233, 0.12)',
		border: 'rgba(56, 189, 248, 0.35)',
		glow: '0 0 8px rgba(56, 189, 248, 0.25)',
	},
	2: {
		level: 2,
		symbol: '🍃',
		descriptor: 'Calme / Chill',
		color: '#2DD4BF', // Teal 400
		bg: 'rgba(20, 184, 166, 0.12)',
		border: 'rgba(45, 212, 191, 0.35)',
		glow: '0 0 8px rgba(45, 212, 191, 0.25)',
	},
	3: {
		level: 3,
		symbol: '🌊',
		descriptor: 'Doux / Warm-up',
		color: '#34D399', // Emerald 400
		bg: 'rgba(16, 185, 129, 0.12)',
		border: 'rgba(52, 211, 153, 0.35)',
		glow: '0 0 8px rgba(52, 211, 153, 0.25)',
	},
	4: {
		level: 4,
		symbol: '✨',
		descriptor: 'Groovy / Progressif',
		color: '#A3E635', // Lime 400
		bg: 'rgba(132, 204, 22, 0.14)',
		border: 'rgba(163, 230, 53, 0.40)',
		glow: '0 0 8px rgba(163, 230, 53, 0.25)',
	},
	5: {
		level: 5,
		symbol: '⚡',
		descriptor: 'Dynamique / Entraînant',
		color: '#FACC15', // Yellow 400
		bg: 'rgba(234, 179, 8, 0.15)',
		border: 'rgba(250, 204, 21, 0.45)',
		glow: '0 0 8px rgba(250, 204, 21, 0.30)',
	},
	6: {
		level: 6,
		symbol: '⚡',
		descriptor: 'Énergique / Club',
		color: '#FB923C', // Orange 400
		bg: 'rgba(249, 115, 22, 0.16)',
		border: 'rgba(251, 146, 60, 0.45)',
		glow: '0 0 10px rgba(251, 146, 60, 0.35)',
	},
	7: {
		level: 7,
		symbol: '🔥',
		descriptor: 'Puissant / Peak Time',
		color: '#F97316', // Orange 500
		bg: 'rgba(234, 88, 12, 0.20)',
		border: 'rgba(249, 115, 22, 0.55)',
		glow: '0 0 12px rgba(249, 115, 22, 0.40)',
	},
	8: {
		level: 8,
		symbol: '🔥',
		descriptor: 'Très énergique / Banger',
		color: '#EF4444', // Red 500
		bg: 'rgba(239, 68, 68, 0.22)',
		border: 'rgba(239, 68, 68, 0.60)',
		glow: '0 0 12px rgba(239, 68, 68, 0.45)',
	},
	9: {
		level: 9,
		symbol: '💥',
		descriptor: 'Explosif / Climax',
		color: '#F43F5E', // Rose 500
		bg: 'rgba(244, 63, 94, 0.25)',
		border: 'rgba(244, 63, 94, 0.65)',
		glow: '0 0 14px rgba(244, 63, 94, 0.50)',
	},
	10: {
		level: 10,
		symbol: '🚀',
		descriptor: 'Ultra Énergique / Maximum',
		color: '#E11D48', // Rose 600 / Neon Ruby
		bg: 'rgba(225, 29, 72, 0.30)',
		border: 'rgba(251, 113, 133, 0.80)',
		glow: '0 0 16px rgba(244, 63, 94, 0.60)',
	},
}

/**
 * Returns Energy Info for an energy value (1-10)
 */
export function getEnergyInfo(energy: number | null | undefined): EnergyInfo | null {
	if (energy === null || energy === undefined) return null
	const level = Math.max(1, Math.min(10, Math.round(energy)))
	return ENERGY_CONFIG[level] ?? null
}
