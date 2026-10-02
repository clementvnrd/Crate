<script lang="ts">
	import { translate } from '$shared/i18n'

	type Props = {
		onResize?: (delta: number) => void
		onResizeStart?: () => void
		onResizeEnd?: () => void
		/** Current size of the resized panel, in px (announced as the separator's value). */
		value?: number
		/** Smallest and largest size of the panel, in px. */
		min?: number
		max?: number
		/** Accessible name (default: "Resize panel"). */
		label?: string
	}

	let { onResize, onResizeStart, onResizeEnd, value, min, max, label }: Props = $props()

	/** Keyboard step in px; Shift moves four times as far. */
	const KEY_STEP = 16

	let isDragging = $state(false)
	let startX = $state(0)

	function handleMouseDown(e: MouseEvent) {
		e.preventDefault()
		isDragging = true
		startX = e.clientX
		document.body.style.cursor = 'col-resize'
		document.body.style.userSelect = 'none'
		window.addEventListener('mousemove', handleMouseMove)
		window.addEventListener('mouseup', handleMouseUp)
		onResizeStart?.()
	}

	function handleMouseMove(e: MouseEvent) {
		if (!isDragging) return
		const delta = e.clientX - startX
		startX = e.clientX
		onResize?.(delta)
	}

	function handleMouseUp() {
		isDragging = false
		document.body.style.cursor = ''
		document.body.style.userSelect = ''
		window.removeEventListener('mousemove', handleMouseMove)
		window.removeEventListener('mouseup', handleMouseUp)
		onResizeEnd?.()
	}

	// Left and Right move the handle like a drag of KEY_STEP px (the mouse never focuses it: mousedown is cancelled).
	// Handled keys are stopped so the global arrow shortcuts (seek, volume) do not also run.
	function handleKeydown(e: KeyboardEvent) {
		if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return
		e.preventDefault()
		e.stopPropagation()
		const step = e.shiftKey ? KEY_STEP * 4 : KEY_STEP
		onResize?.(e.key === 'ArrowLeft' ? -step : step)
	}
</script>

<!-- A focusable separator with a value is the WAI-ARIA "window splitter" widget, but Svelte's a11y rules class every
     separator as static, so they flag its tabindex and handlers; the handle is keyboard operable (arrows) and named. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
	role="separator"
	aria-orientation="vertical"
	aria-label={label ?? $translate('common.resizePanel')}
	aria-valuenow={value}
	aria-valuemin={min}
	aria-valuemax={max}
	tabindex="0"
	class="group relative h-full w-[0px] cursor-col-resize"
	onmousedown={handleMouseDown}
	onkeydown={handleKeydown}
>
	<div
		class="absolute top-1.5 bottom-0 left-0 w-px bg-transparent transition-colors {isDragging
			? 'bg-brand-primary'
			: 'group-hover:bg-brand-primary group-focus-visible:bg-brand-primary'}"
	></div>
	<div class="absolute inset-y-0 -left-1 w-3"></div>
</div>
