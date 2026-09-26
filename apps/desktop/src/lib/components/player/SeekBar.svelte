<script lang="ts">
	import { formatDuration } from '$shared/utils'
	import { Text } from '$lib/components/common'

	type Props = {
		position: number
		duration: number
		disabled?: boolean
		onSeek?: (position: number) => void
	}

	let { position, duration, disabled = false, onSeek }: Props = $props()

	let isDragging = $state(false)
	let dragPosition = $state(0)
	let barElement = $state<HTMLDivElement | null>(null)

	let displayPosition = $derived(isDragging ? dragPosition : position)
	let effectivePosition = $derived(disabled ? 0 : displayPosition)
	let effectiveDuration = $derived(disabled ? 0 : duration)
	let progress = $derived(!disabled && effectiveDuration > 0 ? (effectivePosition / effectiveDuration) * 100 : 0)

	function handleMouseDown(e: MouseEvent) {
		if (disabled || duration === 0) return
		isDragging = true
		updatePositionFromEvent(e)
		window.addEventListener('mousemove', handleMouseMove)
		window.addEventListener('mouseup', handleMouseUp)
	}

	function handleMouseMove(e: MouseEvent) {
		if (!isDragging) return
		updatePositionFromEvent(e)
	}

	function handleMouseUp() {
		if (isDragging) {
			onSeek?.(dragPosition)
			isDragging = false
		}
		window.removeEventListener('mousemove', handleMouseMove)
		window.removeEventListener('mouseup', handleMouseUp)
	}

	function updatePositionFromEvent(e: MouseEvent) {
		if (!barElement || duration <= 0) return
		const rect = barElement.getBoundingClientRect()
		if (rect.width <= 0) return
		const x = Math.max(0, Math.min(e.clientX - rect.left, rect.width))
		const percent = x / rect.width
		dragPosition = Math.floor(percent * duration)
	}
</script>

<div class="flex w-full items-center gap-2">
	<Text variant="caption" color="secondary" tabular class="w-10 text-right">
		{formatDuration(effectivePosition)}
	</Text>

	<div
		bind:this={barElement}
		role="slider"
		tabindex="0"
		aria-label="Seek"
		aria-valuemin={0}
		aria-valuemax={effectiveDuration}
		aria-valuenow={effectivePosition}
		class="seek-bar group relative h-1.5 flex-1 rounded-full bg-surface-2 transition-all {disabled
			? 'cursor-default opacity-40'
			: 'cursor-pointer'}"
		onmousedown={handleMouseDown}
	>
		<!-- Progress -->
		{#if !disabled}
			<div class="absolute inset-y-0 left-0 rounded-full bg-brand-primary" style="width: {progress}%"></div>

			<!-- Thumb -->
			<div
				class="absolute top-1/2 h-3 w-3 -translate-y-1/2 rounded-full border border-stroke bg-white opacity-0 transition-opacity group-hover:opacity-100"
				style="left: calc({progress}% - 6px)"
			></div>
		{/if}
	</div>

	<Text variant="caption" color="secondary" tabular class="w-10">
		{formatDuration(effectiveDuration)}
	</Text>
</div>
