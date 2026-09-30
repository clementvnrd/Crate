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

	const uid = $props.id()

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

<!-- One control: the whole row is the switch (label and description name and describe it). The pill is drawn
     with spans; the button's defaults are reset by preflight, plus w-full and text-left to keep the row's layout. -->
<button
	type="button"
	role="switch"
	aria-checked={checked}
	aria-labelledby={label ? `${uid}-label` : undefined}
	aria-describedby={description ? `${uid}-description` : undefined}
	{disabled}
	class="flex w-full items-center justify-between py-2 text-left {disabled
		? 'cursor-not-allowed opacity-50'
		: 'cursor-pointer'} {className}"
	onclick={handleClick}
>
	<span class="flex flex-col pr-4 select-none">
		{#if label}
			<span id="{uid}-label" class="text-sm font-medium text-text-primary">{label}</span>
		{/if}
		{#if description}
			<span id="{uid}-description" class="text-xs text-text-tertiary">{description}</span>
		{/if}
	</span>

	<!-- iOS / MIK style Pill Switch -->
	<span
		aria-hidden="true"
		class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out {checked
			? 'bg-sky-500 shadow-[0_0_8px_rgba(14,165,233,0.4)]'
			: 'border-stroke bg-surface-2'}"
	>
		<span
			class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-sm ring-0 transition duration-200 ease-in-out {checked
				? 'translate-x-4'
				: 'translate-x-0'}"
		></span>
	</span>
</button>
