import { writable } from 'svelte/store'

export const splashVisible = writable(true)

let isDismissed = false

/**
 * Hides the splash screen. Called when initialization (settings included) has finished, so the
 * onboarding never flashes before the settings say whether it was completed. The only failsafe
 * lives in SplashScreen.svelte (10 s) — no module-level timer racing with initialization.
 */
export function dismissSplash() {
	if (isDismissed) return
	isDismissed = true
	splashVisible.set(false)
}
