<script lang="ts">
	import type { Snippet } from 'svelte'

	type Props = {
		checked?: boolean
		onchange?: (checked: boolean) => void
		label?: string
		ariaLabel?: string
		disabled?: boolean
		/**
		 * `crate` (default): Crate's own box, accent-filled when checked.
		 * `native`: the system checkbox, the look the confirmation dialogs, Duplicate Killer and the Upgrader
		 * have (macOS draws it; the classes in `inputClass` only size it). The label is the `children` snippet,
		 * laid out by `labelClass`, so each call site keeps its own text and colours.
		 */
		appearance?: 'crate' | 'native'
		/** `native` only: classes of the `<label>` around the box and its text. */
		labelClass?: string
		/** `native` only: classes of the `<input>`. */
		inputClass?: string
		children?: Snippet
	}

	let {
		checked = $bindable(false),
		onchange,
		label,
		ariaLabel,
		disabled = false,
		appearance = 'crate',
		labelClass = 'inline-flex cursor-pointer items-center gap-2',
		inputClass = 'h-4 w-4 rounded border-stroke bg-surface-2',
		children,
	}: Props = $props()

	function handleChange(event: MouseEvent) {
		event.stopPropagation()
		if (onchange) {
			// Let parent control state via callback
			onchange(!checked)
		} else {
			// Only toggle internally when using bind:checked
			checked = !checked
		}
	}

	function handleNativeChange(event: Event) {
		const value = (event.currentTarget as HTMLInputElement).checked
		if (onchange) onchange(value)
		else checked = value
	}
</script>

{#if appearance === 'native'}
	<label class={labelClass}>
		<input
			type="checkbox"
			{checked}
			{disabled}
			aria-label={label ?? ariaLabel}
			onchange={handleNativeChange}
			class={inputClass}
		/>
		{@render children?.()}
	</label>
{:else}
	<label
		class="inline-flex cursor-pointer items-center gap-2"
		class:opacity-50={disabled}
		class:cursor-not-allowed={disabled}
	>
		<button
			type="button"
			role="checkbox"
			aria-checked={checked}
			aria-label={label ?? ariaLabel}
			{disabled}
			onclick={handleChange}
			class="flex h-4 w-4 items-center justify-center rounded border transition-colors focus:ring-1 focus:ring-brand-primary focus:ring-offset-1 focus:ring-offset-surface-1 focus:outline-none
			{checked ? 'border-brand-primary bg-brand-primary' : 'border-stroke bg-surface-2 hover:border-text-tertiary'}"
		>
			<svg class="h-3 w-3 text-white {checked ? 'opacity-100' : 'opacity-0'}" viewBox="0 0 12 12" fill="none">
				<path d="M2 6L5 9L10 3" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
			</svg>
		</button>
		{#if label}
			<span class="text-sm text-text-primary select-none">{label}</span>
		{/if}
	</label>
{/if}
