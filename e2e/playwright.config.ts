import { defineConfig, devices } from '@playwright/test'
import { fileURLToPath } from 'node:url'

// End-to-end checks of the interface, run against the browser harness (apps/desktop/harness): the real Svelte app
// with a fake Tauri backend. Run from the repository root: `yarn test:e2e`. Only Chromium is used, and the
// browser build that Playwright picks must already be installed (this repository never downloads one).

const repositoryRoot = fileURLToPath(new URL('..', import.meta.url))

export default defineConfig({
	testDir: '.',
	// `*.e2e.ts` keeps these files out of Vitest's `**/*.{test,spec}.{js,ts}` glob.
	testMatch: '**/*.e2e.ts',
	outputDir: './test-results',
	globalSetup: './global-setup.ts',
	globalTeardown: './global-teardown.ts',
	fullyParallel: true,
	retries: 0,
	timeout: 45_000,
	reporter: [['list']],
	use: {
		baseURL: 'http://localhost:1430',
		trace: 'off',
	},
	projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
	webServer: {
		command: 'yarn harness',
		cwd: repositoryRoot,
		url: 'http://localhost:1430',
		reuseExistingServer: true,
		timeout: 120_000,
	},
})
