import { REFERENCE_NOW } from './reference'

// The update offered by `?update=available` (and the failing variants): the metadata `plugin:updater|check`
// resolves with, which `@tauri-apps/plugin-updater` turns into an `Update` (the rid is added by the handler).

/** Size of the fake bundle, in bytes: what the download's `Started` event announces. */
export const FAKE_UPDATE_SIZE = 18_400_000

export const FAKE_UPDATE = {
	currentVersion: '0.0.0-harness',
	version: '0.4.0',
	date: REFERENCE_NOW,
	body: [
		'## Polish & Unify 0.4',
		'',
		'- Updates no longer interrupt you: a quiet banner under the toolbar, never during playback.',
		'- **Set mode**: build a set from the selected tracks, with harmonic and energy suggestions.',
		'- Discrepancy report: compare Crate with Mixed In Key and a Rekordbox export before changing anything.',
		'- Smart playlists can now filter on energy and on the date a track was added.',
		'',
		'## Fixes',
		'',
		'- The Beatport table keeps its title column readable at 1000 px.',
		'- French numbers and durations use the right separators everywhere.',
	].join('\n'),
	rawJson: {},
}
