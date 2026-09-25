<script lang="ts">
	import { toasts, toastStore } from '$shared/stores/toast'
	import Toast from './Toast.svelte'
</script>

{#if $toasts.length > 0}
	<div class="fixed right-6 bottom-24 z-50 flex flex-col-reverse gap-2 pointer-events-none select-none">
		{#each $toasts as toast (toast.id)}
			<div class="pointer-events-auto shadow-2xl">
				<Toast {toast} onDismiss={() => toastStore.dismiss(toast.id)} />
			</div>
		{/each}
	</div>
{/if}
