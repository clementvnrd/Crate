// In-page UI audit for Crate, run with:
//   playwright-cli run-code --filename=.claude/skills/crate-visual-check/scripts/ui-audit.js
// Reports what the design rules forbid and a screenshot cannot prove on its own:
// WCAG contrast of every visible text, unnamed controls, pointer-only elements, overlapping
// controls, dialogs or content outside the window, crushed truncated columns, text under 12px.
// Single function expression: playwright-cli wraps it in (...) and calls it with the page.
/* eslint-disable @typescript-eslint/no-unused-expressions -- run-code requires a bare function expression */
async (page) => {
	return await page.evaluate(() => {
		const MAX = 40
		const vw = window.innerWidth
		const vh = window.innerHeight
		// A hidden or unsized browser reports 0×0 and every block element measures 0 wide.
		if (vw === 0 || vh === 0)
			return { error: 'Fenêtre de 0×0 : donner une taille (playwright-cli resize 1000 600) puis relancer.' }

		const parse = (c) => {
			const m = c.match(/rgba?\(([^)]+)\)/)
			if (!m) return null
			const [r, g, b, a = 1] = m[1]
				.split(/[\s,/]+/)
				.filter(Boolean)
				.map(Number)
			return { r, g, b, a }
		}
		const blend = (top, bottom) => ({
			r: top.r * top.a + bottom.r * (1 - top.a),
			g: top.g * top.a + bottom.g * (1 - top.a),
			b: top.b * top.a + bottom.b * (1 - top.a),
			a: 1,
		})
		const lum = ({ r, g, b }) => {
			const f = (v) => {
				v /= 255
				return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4
			}
			return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b)
		}
		const ratio = (a, b) => {
			const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x)
			return (hi + 0.05) / (lo + 0.05)
		}
		const visible = (el) => {
			const r = el.getBoundingClientRect()
			const s = getComputedStyle(el)
			return (
				r.width > 0 && r.height > 0 && s.visibility !== 'hidden' && s.display !== 'none' && Number(s.opacity) > 0.05
			)
		}
		const label = (el) => {
			const cls = (el.getAttribute('class') || '').split(/\s+/).filter(Boolean).slice(0, 4).join('.')
			const text = (el.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 40)
			return `${el.tagName.toLowerCase()}${cls ? '.' + cls : ''}${text ? ` « ${text} »` : ''}`
		}
		// Effective background: walk up and blend translucent layers; null when an image/gradient is in the way.
		const background = (el) => {
			const layers = []
			for (let n = el; n && n.nodeType === 1; n = n.parentElement) {
				const s = getComputedStyle(n)
				if (s.backgroundImage && s.backgroundImage !== 'none') return null
				const c = parse(s.backgroundColor)
				if (c && c.a > 0) {
					layers.push(c)
					if (c.a >= 1) break
				}
			}
			let bg = { r: 255, g: 255, b: 255, a: 1 }
			const body = parse(getComputedStyle(document.body).backgroundColor)
			if (body && body.a > 0) bg = body
			for (let i = layers.length - 1; i >= 0; i--) bg = blend(layers[i], bg)
			return bg
		}

		const report = {
			window: `${vw}×${vh}`,
			theme: document.documentElement.getAttribute('data-theme'),
			accent: document.documentElement.getAttribute('data-accent'),
			pageOverflowX: document.documentElement.scrollWidth > vw + 1,
			lowContrast: [],
			unmeasuredContrast: 0,
			smallText: [],
			unnamedControls: [],
			pointerOnly: [],
			overlaps: [],
			outOfWindow: [],
			crushedColumns: [],
		}

		const all = [...document.body.querySelectorAll('*')].filter(visible)

		for (const el of all) {
			const ownText = [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim())
			if (!ownText) continue
			const s = getComputedStyle(el)
			const size = parseFloat(s.fontSize)
			if (size < 12 && report.smallText.length < MAX) report.smallText.push(`${size}px ${label(el)}`)
			const fg = parse(s.color)
			const bg = background(el)
			if (!fg || !bg) {
				report.unmeasuredContrast++
				continue
			}
			const r = ratio(blend(fg, bg), bg)
			const large = size >= 24 || (size >= 18.66 && Number(s.fontWeight) >= 700)
			const min = large ? 3 : 4.5
			if (r < min && report.lowContrast.length < MAX)
				report.lowContrast.push(`${r.toFixed(2)}:1 (min ${min}) ${label(el)}`)
		}

		const controls = all.filter((el) =>
			el.matches(
				'button, a[href], [role="button"], [role="tab"], input, select, textarea, [tabindex]:not([tabindex="-1"])'
			)
		)
		for (const el of controls) {
			const name =
				el.getAttribute('aria-label') ||
				el.getAttribute('aria-labelledby') ||
				el.getAttribute('title') ||
				(el.textContent || '').trim() ||
				(el.labels && el.labels.length ? 'label' : '') ||
				el.getAttribute('placeholder')
			if (!name && report.unnamedControls.length < MAX)
				report.unnamedControls.push(label(el) || el.outerHTML.slice(0, 80))
		}

		for (const el of all) {
			if (report.pointerOnly.length >= MAX) break
			if (el.matches('button, a, input, select, textarea, label, [role], [tabindex]')) continue
			if (getComputedStyle(el).cursor !== 'pointer') continue
			if (el.closest('button, a, [role="button"], label')) continue
			if (el.parentElement && getComputedStyle(el.parentElement).cursor === 'pointer') continue
			report.pointerOnly.push(label(el))
		}

		const boxes = controls.slice(0, 400).map((el) => ({ el, r: el.getBoundingClientRect() }))
		for (let i = 0; i < boxes.length && report.overlaps.length < MAX; i++) {
			for (let j = i + 1; j < boxes.length; j++) {
				const a = boxes[i]
				const b = boxes[j]
				if (a.el.contains(b.el) || b.el.contains(a.el)) continue
				const w = Math.min(a.r.right, b.r.right) - Math.max(a.r.left, b.r.left)
				const h = Math.min(a.r.bottom, b.r.bottom) - Math.max(a.r.top, b.r.top)
				if (w > 2 && h > 2) report.overlaps.push(`${label(a.el)} ⟷ ${label(b.el)}`)
			}
		}

		for (const el of document.querySelectorAll('dialog[open], [role="dialog"], [role="menu"], [role="tooltip"]')) {
			if (!visible(el)) continue
			const r = el.getBoundingClientRect()
			if (r.left < -1 || r.top < -1 || r.right > vw + 1 || r.bottom > vh + 1)
				report.outOfWindow.push(
					`${label(el)} [${Math.round(r.left)},${Math.round(r.top)} → ${Math.round(r.right)},${Math.round(r.bottom)}]`
				)
		}

		for (const el of all) {
			if (report.crushedColumns.length >= MAX) break
			const s = getComputedStyle(el)
			if (s.textOverflow !== 'ellipsis') continue
			if (el.scrollWidth > el.clientWidth && el.clientWidth < 48)
				report.crushedColumns.push(`${el.clientWidth}px ${label(el)}`)
		}

		return report
	})
}
