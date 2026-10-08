import type { Args, CallRecord, HandlerMap, HarnessParams } from './types'
import { defaultResponse } from './defaults'
import { createState } from './state'
import { beatportHandlers } from './handlers/beatport'
import { discoveryHandlers } from './handlers/discovery'
import { libraryHandlers } from './handlers/library'
import { maintenanceHandlers } from './handlers/maintenance'
import { noopHandlers } from './handlers/noops'
import { organiseHandlers } from './handlers/organise'
import { playerHandlers } from './handlers/player'
import { pluginHandlers } from './handlers/plugins'
import { statsHandlers } from './handlers/stats'
import { systemHandlers } from './handlers/system'

const MAX_RECORDED_CALLS = 300

export interface Backend {
	/** Answer one `invoke` call the way the real Tauri backend would. */
	handle: (command: string, rawArgs: unknown) => Promise<unknown>
	/** Names of the commands answered by a pattern default instead of a precise handler. */
	unmocked: Set<string>
	calls: CallRecord[]
}

function toArgs(rawArgs: unknown): Args {
	return rawArgs !== null && typeof rawArgs === 'object' && !Array.isArray(rawArgs) ? (rawArgs as Args) : {}
}

/** Fixtures are cloned on the way out so the app can mutate what it receives without corrupting them. */
function clone<T>(value: T): T {
	return value === undefined || value === null ? value : structuredClone(value)
}

export function createBackend(params: HarnessParams): Backend {
	const state = createState(params)
	const handlers: HandlerMap = {
		...noopHandlers(),
		...pluginHandlers(params),
		...libraryHandlers(state),
		...organiseHandlers(state),
		...playerHandlers(state),
		...beatportHandlers(state),
		...discoveryHandlers(state),
		...statsHandlers(state),
		...systemHandlers(state),
		...maintenanceHandlers(state),
	}
	const unmocked = new Set<string>()
	const calls: CallRecord[] = []

	async function handle(command: string, rawArgs: unknown): Promise<unknown> {
		const args = toArgs(rawArgs)
		const handler = handlers[command]
		calls.push({ command, args, mocked: handler !== undefined })
		if (calls.length > MAX_RECORDED_CALLS) calls.shift()

		if (params.latencyMs > 0) await new Promise((resolve) => setTimeout(resolve, params.latencyMs))

		// A handler may be async (the updater's download reports progress over time).
		if (handler) return clone(await handler(args))

		if (!unmocked.has(command)) {
			unmocked.add(command)
			console.warn(`[harness] unmocked command: ${command}`)
		}
		return defaultResponse(command)
	}

	return { handle, unmocked, calls }
}
