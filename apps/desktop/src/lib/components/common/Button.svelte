<script lang="ts">
	import type { Snippet } from 'svelte'

	/**
	 * Fill of the `primary` variant. `accent` (default) is Crate's own colour and the Button's usual look; its text
	 * is `brand-on`, black or white per accent so it reads at 4.5:1 (owner decision CRA-141).
	 * `deck` (Player), `beatport` (Beatport view), `spotify` (Spotify's own connect button) and `rekordbox` (the
	 * Rekordbox sync button of Pulse) render the family look those views have: bold text, a larger radius, an
	 * optional coloured glow and press scale, the global focus outline, and padding given by the call site
	 * through `size="bare"` + `class`. Giving a `shape` to an `accent` button gives it that family look in the
	 * accent colour (the Player's "Add to library" pill). Other variants ignore `tone`.
	 */
	type Tone = 'accent' | 'deck' | 'beatport' | 'spotify' | 'rekordbox'
	/** Coloured shadow of a family button: shadow size / tone opacity, as each view writes it. */
	type Glow = 'md/10' | 'md/20' | 'lg/20' | 'lg/25'

	type Props = {
		variant?: 'primary' | 'secondary' | 'ghost' | 'danger' | 'ghost-danger' | 'outline'
		tone?: Tone
		/** `bare`: no padding or text size, the call site gives them in `class` (family tones). */
		size?: 'sm' | 'md' | 'lg' | 'bare'
		/** Family look only: radius of the button (`xl` by default for the family tones). */
		shape?: 'lg' | 'xl' | 'full'
		/** Family look only: text weight (`bold` by default). */
		weight?: 'bold' | 'semibold'
		/** Family look only: `flex` where the call site needs a block-level button (default `inline-flex`). */
		display?: 'inline-flex' | 'flex'
		/** Family tones only: coloured shadow in the tone's colour. */
		glow?: Glow
		/** Family tones only: shrink slightly while pressed (`active:scale-95`). */
		press?: boolean
		/** Family tones only: grow slightly on hover (`hover:scale-105`). */
		lift?: boolean
		/**
		 * `primary` with the default look only: replaces the accent fill (background, text and hover classes), for
		 * the few primary buttons that are not accent-coloured (the Upgrader's green, the destructive reds). A fill
		 * given in `class` competes with the accent fill, and which one wins depends on the generated CSS order.
		 */
		fill?: string
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
		shape,
		weight = 'bold',
		display = 'inline-flex',
		glow,
		press = false,
		lift = false,
		fill,
		disabled = false,
		type = 'button',
		class: className = '',
		onclick,
		'aria-label': ariaLabel,
		children,
	}: Props = $props()

	const baseStyles =
		'inline-flex items-center justify-center font-medium rounded-md transition-[background-color,color,filter,opacity] duration-150 hover:cursor-pointer focus:ring-1 focus:ring-brand-primary focus:outline-none disabled:opacity-50 disabled:cursor-not-allowed'

	// The family look, as the Player, Beatport, Spotify and Rekordbox buttons are written.
	const familyBase =
		'items-center justify-center transition-all hover:cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed'

	// Family colours come from the tokens of style.css (DESIGN.md, Graphic charter).
	const familyFills: Record<Tone, string> = {
		accent: 'bg-brand-primary text-brand-on hover:brightness-110',
		deck: 'bg-deck-live-tint text-black hover:bg-deck-live',
		beatport: 'bg-beatport text-black hover:bg-beatport-hover',
		spotify: 'bg-source-spotify text-black hover:bg-source-spotify-hover',
		rekordbox: 'bg-source-rekordbox text-white hover:bg-source-rekordbox-hover',
	}

	const familyGlows: Record<Tone, Record<Glow, string>> = {
		accent: {
			'md/10': 'shadow-md shadow-brand-primary/10',
			'md/20': 'shadow-md shadow-brand-primary/20',
			'lg/20': 'shadow-lg shadow-brand-primary/20',
			'lg/25': 'shadow-lg shadow-brand-primary/25',
		},
		deck: {
			'md/10': 'shadow-md shadow-deck-live-tint/10',
			'md/20': 'shadow-md shadow-deck-live-tint/20',
			'lg/20': 'shadow-lg shadow-deck-live-tint/20',
			'lg/25': 'shadow-lg shadow-deck-live-tint/25',
		},
		beatport: {
			'md/10': 'shadow-md shadow-beatport/10',
			'md/20': 'shadow-md shadow-beatport/20',
			'lg/20': 'shadow-lg shadow-beatport/20',
			'lg/25': 'shadow-lg shadow-beatport/25',
		},
		spotify: {
			'md/10': 'shadow-md shadow-source-spotify/10',
			'md/20': 'shadow-md shadow-source-spotify/20',
			'lg/20': 'shadow-lg shadow-source-spotify/20',
			'lg/25': 'shadow-lg shadow-source-spotify/25',
		},
		rekordbox: {
			'md/10': 'shadow-md shadow-source-rekordbox/10',
			'md/20': 'shadow-md shadow-source-rekordbox/20',
			'lg/20': 'shadow-lg shadow-source-rekordbox/20',
			'lg/25': 'shadow-lg shadow-source-rekordbox/25',
		},
	}

	const variantStyles = {
		primary: 'bg-brand-primary text-brand-on hover:bg-brand-hover',
		secondary: 'bg-surface-2 text-text-primary hover:brightness-95',
		ghost: 'bg-transparent text-text-secondary hover:bg-surface-2 hover:text-text-primary',
		danger: 'bg-danger text-white hover:bg-danger/90',
		'ghost-danger': 'bg-transparent text-red-500 hover:bg-red-500/10',
		outline: 'bg-surface-2 border border-stroke text-text-primary hover:brightness-95',
	}

	const shapes = { lg: 'rounded-lg', xl: 'rounded-xl', full: 'rounded-full' }

	const sizeStyles = {
		sm: 'px-2.5 py-1.5 text-xs',
		md: 'px-3 py-2 text-sm',
		lg: 'px-4 py-2 text-base',
		bare: '',
	}

	const classes = $derived.by(() => {
		if (variant === 'primary' && (tone !== 'accent' || shape !== undefined)) {
			return [
				display,
				familyBase,
				weight === 'semibold' ? 'font-semibold' : 'font-bold',
				familyFills[tone],
				shapes[shape ?? 'xl'],
				glow ? familyGlows[tone][glow] : '',
				press ? 'active:scale-95' : '',
				lift ? 'hover:scale-105' : '',
				sizeStyles[size],
				className,
			].join(' ')
		}
		const look = variant === 'primary' && fill ? fill : variantStyles[variant]
		return `${baseStyles} ${look} ${sizeStyles[size]} ${className}`
	})
</script>

<button {type} {disabled} aria-label={ariaLabel} class={classes} {onclick}>
	{@render children()}
</button>
