<script lang="ts">
	type Props = {
		checked: boolean
		onchange?: (checked: boolean) => void
		label?: string
		description?: string
		disabled?: boolean
		class?: string
	}

	let {
		checked = $bindable(false),
		onchange,
		label,
		description,
		disabled = false,
		class: className = '',
	}: Props = $props()

	function handleClick() {
		if (disabled) return
		const next = !checked
		if (onchange) {
			onchange(next)
		} else {
			checked = next
		}
	}
</script>

<div class="flex items-center justify-between py-2 {disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'} {className}" onclick={handleClick} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && handleClick()}>
	<div class="flex flex-col pr-4 select-none">
		{#if label}
			<span class="text-sm font-medium text-text-primary">{label}</span>
		{/if}
		{#if description}
			<span class="text-xs text-text-tertiary">{description}</span>
		{/if}
	</div>

	<!-- iOS / MIK style Pill Switch -->
	<button
		type="button"
		role="switch"
		aria-label={label || 'Toggle'}
		aria-checked={checked}
		{disabled}
		class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none {checked ? 'bg-sky-500 shadow-[0_0_8px_rgba(14,165,233,0.4)]' : 'bg-surface-2 border-stroke'}"
	>
		<span
			aria-hidden="true"
			class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-sm ring-0 transition duration-200 ease-in-out {checked ? 'translate-x-4' : 'translate-x-0'}"
		></span>
	</button>
</div>
