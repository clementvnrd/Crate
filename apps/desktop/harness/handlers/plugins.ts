import type { HandlerMap } from '../types'

// Tauri plugin commands (`plugin:<name>|<command>`). Native dialogs always answer "cancelled" so a test never
// blocks on a file picker; clipboard and opener calls do nothing. (`plugin:event|*` is handled by `mockIPC`.)

export function pluginHandlers(): HandlerMap {
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
		'plugin:process|restart': () => null,
		'plugin:process|exit': () => null,
		// No update available.
		'plugin:updater|check': () => null,
	}
}
