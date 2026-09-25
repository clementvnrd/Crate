<script lang="ts">
	type Variant = 'full' | 'badge' | 'icon'
	type Size = 'sm' | 'md' | 'lg'

	type Props = {
		variant?: Variant
		size?: Size
		class?: string
		showPro?: boolean
		animated?: boolean
	}

	let {
		variant = 'full',
		size = 'md',
		class: className = '',
		showPro = true,
		animated = false,
	}: Props = $props()

	const sizeClasses: Record<Size, { icon: string; text: string; pro: string; gap: string }> = {
		sm: { icon: 'w-4 h-4', text: 'text-[10px]', pro: 'text-[8px] px-1 py-0.2' , gap: 'gap-1.5' },
		md: { icon: 'w-5 h-5', text: 'text-xs', pro: 'text-[9px] px-1.5 py-0.5', gap: 'gap-2' },
		lg: { icon: 'w-7 h-7', text: 'text-sm', pro: 'text-[10px] px-2 py-0.5', gap: 'gap-2.5' },
	}
</script>

<div class="inline-flex items-center {sizeClasses[size].gap} {className} select-none">
	<!-- Mixed In Key Glowing Double-Ring Icon -->
	<svg
		viewBox="0 0 100 100"
		class="{sizeClasses[size].icon} shrink-0 drop-shadow-[0_0_8px_rgba(0,195,255,0.6)] {animated ? 'animate-pulse' : ''}"
		fill="none"
		xmlns="http://www.w3.org/2000/svg"
	>
		<defs>
			<!-- Outer Ring Gradient -->
			<linearGradient id="mikOuterGrad" x1="0%" y1="0%" x2="100%" y2="100%">
				<stop offset="0%" stop-color="#38E1FF" />
				<stop offset="50%" stop-color="#00A2FF" />
				<stop offset="100%" stop-color="#0066FF" />
			</linearGradient>
			<!-- Inner Ring Gradient -->
			<linearGradient id="mikInnerGrad" x1="0%" y1="100%" x2="100%" y2="0%">
				<stop offset="0%" stop-color="#00F5FF" />
				<stop offset="100%" stop-color="#00B4FF" />
			</linearGradient>
		</defs>

		<!-- Outer Circle -->
		<circle cx="50" cy="50" r="44" stroke="url(#mikOuterGrad)" stroke-width="12" stroke-linecap="round" />
		<!-- Gap Circle (Dark Stroke) -->
		<circle cx="50" cy="50" r="32" stroke="#090d16" stroke-width="6" />
		<!-- Inner Glowing Circle -->
		<circle cx="50" cy="50" r="24" stroke="url(#mikInnerGrad)" stroke-width="6" />
		<!-- Center Hole -->
		<circle cx="50" cy="50" r="16" fill="#090d16" />
	</svg>

	{#if variant !== 'icon'}
		<div class="flex items-center {sizeClasses[size].gap}">
			<div class="flex flex-col leading-none">
				<span class="font-extrabold tracking-wider text-text-primary {sizeClasses[size].text}" style="font-family: 'Jost', var(--font-family), sans-serif;">
					MIXED
				</span>
				<span class="font-extrabold tracking-wider text-sky-500 dark:text-sky-400 {sizeClasses[size].text}" style="font-family: 'Jost', var(--font-family), sans-serif;">
					IN KEY
				</span>
			</div>

			{#if showPro}
				<span
					class="rounded bg-gradient-to-r from-cyan-400 to-sky-500 font-black tracking-wider text-black shadow-[0_0_10px_rgba(56,189,248,0.4)] {sizeClasses[size].pro}"
					style="font-family: 'Jost', var(--font-family), sans-serif;"
				>
					PRO
				</span>
			{/if}
		</div>
	{/if}
</div>
