<script lang="ts">
	import type { Snippet } from 'svelte'

	/**
	 * Fill of the `primary` variant. `accent` (default) is Crate's own colour and the Button's usual look.
	 * `deck` (Player), `beatport` (Beatport view) and `spotify` (Spotify's own connect button) render the
	 * family look those views have today, which the owner froze (CRA-141): bold text, a larger radius, an
	 * optional coloured glow and press scale, the global focus outline, and padding given by the call site
	 * through `size="bare"` + `class`. Other variants ignore `tone`.
	 */
	type Tone = 'accent' | 'deck' | 'beatport' | 'spotify'
	type FamilyTone = Exclude<Tone, 'accent'>
	/** Coloured shadow of a family button: shadow size / tone opacity, as each view writes it today. */
	type Glow = 'md/10' | 'md/20' | 'lg/20' | 'lg/25'

	type Props = {
		variant?: 'primary' | 'secondary' | 'ghost' | 'danger' | 'ghost-danger' | 'outline'
		tone?: Tone
		/** `bare`: no padding or text size, the call site gives them in `class` (family tones). */
		size?: 'sm' | 'md' | 'lg' | 'bare'
		/** Family tones only: radius of the button. */
		shape?: 'lg' | 'xl'
		/** Family tones only: coloured shadow in the tone's colour. */
		glow?: Glow
		/** Family tones only: shrink slightly while pressed (`active:scale-95`). */
		press?: boolean
		/** Family tones only: grow slightly on hover (`hover:scale-105`). */
		lift?: boolean
		disabled?: boolean
		type?: 'button' | 'submit' | 'reset'
		class?: string
		onclick?: (e: MouseEvent) => void
		/** Accessible name, required when the visible label can be hidden (icon-only layouts) */
		'aria-label'?: string
		children: Snippet
	}

	let {
		variant = 'secondary',
		tone = 'accent',
		size = 'md',
		shape = 'xl',
		glow,
		press = false,
		lift = false,
		disabled = false,
		type = 'button',
		class: className = '',
		onclick,
		'aria-label': ariaLabel,
		children,
	}: Props = $props()

	const baseStyles =
		'inline-flex items-center justify-center font-medium rounded-md transition-[background-color,color,filter,opacity] duration-150 hover:cursor-pointer focus:ring-1 focus:ring-brand-primary focus:outline-none disabled:opacity-50 disabled:cursor-not-allowed'

	// The family look, as the Player, Beatport and Spotify buttons are written today (their tokens are D3).
	const familyBase =
		'inline-flex items-center justify-center font-bold transition-all hover:cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed'

	const familyFills: Record<FamilyTone, string> = {
		deck: 'bg-cyan-500 text-black hover:bg-cyan-400',
		beatport: 'bg-[#00FF96] text-black hover:bg-[#00e687]',
		spotify: 'bg-[#1DB954] text-black hover:bg-[#1ed760]',
	}

	const familyGlows: Record<FamilyTone, Record<Glow, string>> = {
		deck: {
			'md/10': 'shadow-md shadow-cyan-500/10',
			'md/20': 'shadow-md shadow-cyan-500/20',
			'lg/20': 'shadow-lg shadow-cyan-500/20',
			'lg/25': 'shadow-lg shadow-cyan-500/25',
		},
		beatport: {
			'md/10': 'shadow-md shadow-[#00FF96]/10',
			'md/20': 'shadow-md shadow-[#00FF96]/20',
			'lg/20': 'shadow-lg shadow-[#00FF96]/20',
			'lg/25': 'shadow-lg shadow-[#00FF96]/25',
		},
		spotify: {
			'md/10': 'shadow-md shadow-[#1DB954]/10',
			'md/20': 'shadow-md shadow-[#1DB954]/20',
			'lg/20': 'shadow-lg shadow-[#1DB954]/20',
			'lg/25': 'shadow-lg shadow-[#1DB954]/25',
		},
	}

	const variantStyles = {
		primary: 'bg-brand-primary text-white hover:bg-brand-hover',
		secondary: 'bg-surface-2 text-text-primary hover:brightness-95',
		ghost: 'bg-transparent text-text-secondary hover:bg-surface-2 hover:text-text-primary',
		danger: 'bg-danger text-white hover:bg-danger/90',
		'ghost-danger': 'bg-transparent text-red-500 hover:bg-red-500/10',
		outline: 'bg-surface-2 border border-stroke text-text-primary hover:brightness-95',
	}

	const sizeStyles = {
		sm: 'px-2.5 py-1.5 text-xs',
		md: 'px-3 py-2 text-sm',
		lg: 'px-4 py-2 text-base',
		bare: '',
	}

	const classes = $derived.by(() => {
		if (variant === 'primary' && tone !== 'accent') {
			return [
				familyBase,
				familyFills[tone],
				shape === 'lg' ? 'rounded-lg' : 'rounded-xl',
				glow ? familyGlows[tone][glow] : '',
				press ? 'active:scale-95' : '',
				lift ? 'hover:scale-105' : '',
				sizeStyles[size],
				className,
			].join(' ')
		}
		return `${baseStyles} ${variantStyles[variant]} ${sizeStyles[size]} ${className}`
	})
</script>

<button {type} {disabled} aria-label={ariaLabel} class={classes} {onclick}>
	{@render children()}
</button>
