// Classic script (not a module) injected first in <head> by vite.harness.config.ts, compiled to an IIFE with
// esbuild. It must run BEFORE the inline theme script of app.html, which reads these localStorage keys to paint the
// first frame in the right theme. It copies the appearance URL parameters into localStorage:
//   ?theme=dark|light|system   ?accent=blue|amber|…   ?lang=en|fr|…   ?font=open-sans|fira-code|…
// Everything else (`?beatport=out`, `?library=empty`, …) is read by install.ts.

;(function applyAppearanceParams() {
	const keys: Record<string, string> = {
		theme: 'crate-theme',
		accent: 'crate-accent',
		lang: 'crate-language',
		font: 'crate-font',
	}
	try {
		const query = new URLSearchParams(window.location.search)
		for (const param of Object.keys(keys)) {
			const value = query.get(param)
			if (value) window.localStorage.setItem(keys[param], value)
		}
	} catch {
		// Storage unavailable (private mode…): the app falls back to its defaults.
	}
})()

export {}
