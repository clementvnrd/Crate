// Deterministic cover art: inline SVG data URLs, so the harness never touches the network or the disk.
// The app's `getArtworkUrl()` passes `data:` URLs through untouched, so they can stand in for `artwork_path`.

/** A gradient cover with a simple geometric motif, derived only from the seed. */
export function artworkDataUrl(seed: number): string {
	const hue = (seed * 47 + 13) % 360
	const hue2 = (hue + 50) % 360
	const motifs = [
		`<circle cx='78' cy='42' r='26' fill='hsl(${hue2} 80% 70%)' opacity='.55'/>`,
		`<rect x='22' y='58' width='76' height='36' rx='6' fill='hsl(${hue2} 80% 75%)' opacity='.45'/>`,
		`<path d='M0 96 L40 44 L72 80 L96 56 L120 92 V120 H0 Z' fill='hsl(${hue2} 70% 20%)' opacity='.55'/>`,
	]
	const motif = motifs[seed % motifs.length]
	const svg =
		`<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 120 120'>` +
		`<defs><linearGradient id='g' x1='0' y1='0' x2='1' y2='1'>` +
		`<stop offset='0' stop-color='hsl(${hue} 70% 46%)'/>` +
		`<stop offset='1' stop-color='hsl(${hue2} 75% 26%)'/>` +
		`</linearGradient></defs>` +
		`<rect width='120' height='120' fill='url(#g)'/>${motif}</svg>`
	return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`
}
