import { defineConfig } from 'vitest/config'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { svelteTesting } from '@testing-library/svelte/vite'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const dirname = path.dirname(fileURLToPath(import.meta.url))

export default defineConfig({
	plugins: [svelte({ hot: !process.env.VITEST }), svelteTesting()],
	test: {
		environment: 'jsdom',
		setupFiles: ['./vitest.setup.ts'],
		globals: true,
		include: ['**/*.{test,spec}.{js,ts}'],
		exclude: ['**/node_modules/**', '**/src-tauri/**'],
	},
	resolve: {
		alias: {
			$shared: path.resolve(dirname, './shared'),
			$lib: path.resolve(dirname, './apps/desktop/src/lib'),
		},
	},
})
