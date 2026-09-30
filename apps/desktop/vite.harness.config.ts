// Vite config of the browser harness (`yarn harness`): the regular app config, plus a fake Tauri backend injected
// into the page, on its own port. See harness/README.md.
import { readFileSync } from 'node:fs'
import type { ServerResponse } from 'node:http'
import { fileURLToPath } from 'node:url'
import { defineConfig, mergeConfig, transformWithEsbuild, type Plugin } from 'vite'
import baseConfig from './vite.config'

/** Port of the harness. 1420 and 1421 belong to `yarn dev`, so both can run side by side. */
const HARNESS_PORT = 1430

type WriteArgs = unknown[]

/**
 * Injects the harness at the very top of <head> of the page the dev server returns: first the appearance
 * parameters (a classic script, it must run before app.html's own theme script), then the module that installs
 * the fake backend (module scripts run before the app code, which SvelteKit loads by dynamic import).
 *
 * SvelteKit does not run Vite's `transformIndexHtml` hook on src/app.html, so the tags are added to the HTML
 * response itself, by a middleware placed in front of SvelteKit's.
 */
function crateHarness(): Plugin {
	const earlyPath = fileURLToPath(new URL('./harness/early.ts', import.meta.url))
	let tags = ''

	function inject(html: string): string {
		return html.replace(/<head[^>]*>/i, (head) => `${head}${tags}`)
	}

	return {
		name: 'crate-harness',
		apply: 'serve',
		async configResolved() {
			const early = await transformWithEsbuild(readFileSync(earlyPath, 'utf-8'), earlyPath, { format: 'iife' })
			tags = `<script>${early.code}</script><script type="module" src="/harness/install.ts"></script>`
		},
		configureServer(server) {
			server.middlewares.use((req, res: ServerResponse, next) => {
				if (req.method !== 'GET' || !(req.headers.accept ?? '').includes('text/html')) return next()

				const write = res.write.bind(res) as (...args: WriteArgs) => boolean
				const end = res.end.bind(res) as (...args: WriteArgs) => ServerResponse
				const setHeader = res.setHeader.bind(res)
				const chunks: Buffer[] = []
				let buffering: boolean | undefined

				// The injected tags change the body length: drop the Content-Length SvelteKit computed.
				res.setHeader = (name, value) => (name.toLowerCase() === 'content-length' ? res : setHeader(name, value))

				const shouldBuffer = () => (buffering ??= String(res.getHeader('content-type') ?? '').includes('text/html'))
				const toBuffer = (chunk: unknown) =>
					chunk instanceof Uint8Array ? Buffer.from(chunk) : Buffer.from(String(chunk))

				res.write = ((chunk: unknown, ...rest: WriteArgs) => {
					if (!shouldBuffer()) return write(chunk, ...rest)
					chunks.push(toBuffer(chunk))
					return true
				}) as typeof res.write
				res.end = ((chunk?: unknown, ...rest: WriteArgs) => {
					if (!shouldBuffer()) return end(chunk, ...rest)
					if (typeof chunk === 'string' || chunk instanceof Uint8Array) chunks.push(toBuffer(chunk))
					return end(inject(Buffer.concat(chunks).toString('utf-8')))
				}) as typeof res.end

				next()
			})
		},
	}
}

export default defineConfig(async (env) => {
	const base = await (typeof baseConfig === 'function' ? baseConfig(env) : baseConfig)
	return mergeConfig(base, {
		plugins: [crateHarness()],
		// SvelteKit only lets Vite serve src/, node_modules and the aliased shared/: allow harness/ too.
		server: {
			port: HARNESS_PORT,
			strictPort: true,
			fs: { allow: [fileURLToPath(new URL('./harness', import.meta.url))] },
		},
		// Not imported by the app itself: pre-bundle it so the first page load does not trigger a reload.
		optimizeDeps: { include: ['@tauri-apps/api/mocks'] },
	})
})
