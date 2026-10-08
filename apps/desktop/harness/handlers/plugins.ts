import type { HandlerMap, HarnessParams } from '../types'
import { FAKE_UPDATE, FAKE_UPDATE_SIZE } from '../fixtures/updater'

// Tauri plugin commands (`plugin:<name>|<command>`). Native dialogs always answer "cancelled" so a test never
// blocks on a file picker; clipboard and opener calls do nothing. (`plugin:event|*` is handled by `mockIPC`.)

/** What `@tauri-apps/api/mocks` installs: the callback registry a `Channel` listens on. */
interface TauriInternals {
	runCallback: (id: number, data: unknown) => void
}

/** Download events are sent in this many chunks, one every `CHUNK_INTERVAL_MS` (about three seconds in all). */
const CHUNKS = 20
const CHUNK_INTERVAL_MS = 150

const UPDATE_RID = 4242
const DOWNLOADED_BYTES_RID = 4243

const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))

/**
 * Send one message through the `Channel` the app passed as `onEvent`, the way the Rust side does: each message
 * carries its index so the channel can keep them in order.
 */
function channelSender(channel: unknown): (message: unknown) => void {
	const id = (channel as { id?: number } | null)?.id
	const internals = (window as unknown as { __TAURI_INTERNALS__?: TauriInternals }).__TAURI_INTERNALS__
	let index = 0
	return (message) => {
		if (typeof id === 'number' && internals) internals.runCallback(id, { index: index++, message })
	}
}

/** `plugin:updater|download`: Started, Progress chunks over ~3 s, Finished; or a rejection halfway through. */
async function download(onEvent: unknown, fails: boolean): Promise<number> {
	const send = channelSender(onEvent)
	const chunkLength = Math.ceil(FAKE_UPDATE_SIZE / CHUNKS)
	send({ event: 'Started', data: { contentLength: FAKE_UPDATE_SIZE } })
	for (let chunk = 0; chunk < CHUNKS; chunk++) {
		await wait(CHUNK_INTERVAL_MS)
		if (fails && chunk === CHUNKS / 2) {
			throw 'error sending request for url (https://github.com/clementvnrd/Crate/releases/download/v0.4.0/Crate.app.tar.gz): operation timed out'
		}
		send({ event: 'Progress', data: { chunkLength } })
	}
	send({ event: 'Finished' })
	return DOWNLOADED_BYTES_RID
}

export function pluginHandlers(params: HarnessParams): HandlerMap {
	const offersUpdate = params.update !== 'none' && params.update !== 'check-fails'
	return {
		'plugin:dialog|open': () => null,
		'plugin:dialog|save': () => null,
		'plugin:dialog|message': () => 'Ok',
		'plugin:dialog|ask': () => false,
		'plugin:dialog|confirm': () => false,
		'plugin:opener|open_url': () => null,
		'plugin:opener|open_path': () => null,
		'plugin:opener|reveal_item_in_dir': () => null,
		'plugin:clipboard-manager|write_text': () => null,
		'plugin:clipboard-manager|read_text': () => '',
		// Relaunching does nothing: the page stays as it is.
		'plugin:process|restart': () => null,
		'plugin:process|exit': () => null,
		// No update unless `?update=…` asks for one (see README.md).
		'plugin:updater|check': () => {
			if (params.update === 'check-fails') {
				throw 'error sending request for url (https://github.com/clementvnrd/Crate/releases/latest/download/latest.json): dns error'
			}
			return offersUpdate ? { ...FAKE_UPDATE, rid: UPDATE_RID } : null
		},
		'plugin:updater|download': ({ onEvent }) => download(onEvent, params.update === 'download-fails'),
		'plugin:updater|install': async () => {
			await wait(CHUNK_INTERVAL_MS * 4)
			if (params.update === 'install-fails') {
				throw 'failed to replace the application bundle: Permission denied (os error 13)'
			}
			return null
		},
		'plugin:resources|close': () => null,
	}
}
