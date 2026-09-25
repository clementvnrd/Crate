import { writable } from 'svelte/store'

export const splashVisible = writable(true)

let isDismissed = false

export function dismissSplash() {
	if (isDismissed) return
	isDismissed = true
	splashVisible.set(false)
}

// Failsafe auto-dismiss: ensure splash screen NEVER traps the user under any circumstance
if (typeof window !== 'undefined') {
	setTimeout(() => {
		dismissSplash()
	}, 1500)
}

