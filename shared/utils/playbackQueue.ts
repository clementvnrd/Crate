/**
 * Pure next / previous / shuffle logic for the player queues (CRA-167).
 *
 * Extracted from `createAppSetup` so it can be unit-tested with a deterministic
 * random source. Nothing here touches a store or the player: the caller passes
 * the candidates in, gets the chosen item back, and does the loading and playing
 * itself — `navigateQueue` takes that loading as an injected `start` function.
 *
 * Since CRA-180 / CRA-181 the module also owns the rules that make the queue
 * coherent whatever starts playback: a track that fails to load is skipped
 * (bounded, never counted as played), the shuffle session follows every start
 * (`ShuffleSession.sync`), "previous" with nothing to go back to restarts the
 * current track, and a one-track list loops in both modes.
 */

/** Returns a number in [0, 1), like `Math.random`. Injected so tests are deterministic. */
export type RandomSource = () => number

/** Which way a navigation moves through the queue. */
export type QueueDirection = 'next' | 'previous'

/**
 * How many tracks in a row may fail to load before a navigation gives up (CRA-180). Ten missing files in a row means
 * something is wrong as a whole (the drive holding the music is unplugged, the output device is gone): stop with one
 * clear message instead of churning through a 50,000-track library. A shorter queue stops once every other track
 * was tried.
 */
export const MAX_CONSECUTIVE_LOAD_FAILURES = 10

// ---------------------------------------------------------------------------
// Sequential navigation (shuffle off)
// ---------------------------------------------------------------------------

/**
 * Index of the item after `currentIndex`, wrapping to the start. An unknown
 * current item (`currentIndex < 0`) starts at the first item. `length` must be > 0.
 */
export function sequentialNextIndex(currentIndex: number, length: number): number {
	return currentIndex >= 0 ? (currentIndex + 1) % length : 0
}

/**
 * Index of the item before `currentIndex`, wrapping to the end. An unknown
 * current item (`currentIndex < 0`) starts at the last item. `length` must be > 0.
 */
export function sequentialPreviousIndex(currentIndex: number, length: number): number {
	return currentIndex >= 0 ? (currentIndex - 1 + length) % length : length - 1
}

/**
 * The items to try, in order, when moving `direction` from `currentIndex` with shuffle off: every other item once,
 * wrapping around the list. The current item is never offered again, so a list whose other tracks all fail ends
 * instead of restarting what is playing. A one-item list offers its only item (sequential play loops a single
 * track); an unknown current item (`currentIndex < 0`) offers every item, from the first (next) or the last
 * (previous).
 *
 * Returns a picker: each call gives the next candidate, then `null` once every candidate was offered.
 */
export function sequentialPicker<T>(
	items: readonly T[],
	currentIndex: number,
	direction: QueueDirection
): () => T | null {
	const length = items.length
	const step = direction === 'next' ? sequentialNextIndex : sequentialPreviousIndex
	const known = currentIndex >= 0 && currentIndex < length
	const total = known && length > 1 ? length - 1 : length
	let cursor = known ? currentIndex : -1
	let offered = 0
	return () => {
		if (offered >= total) return null
		cursor = step(cursor, length)
		offered++
		return items[cursor]
	}
}

/**
 * The only item of `items` when it is the item playing now (`currentKey`), else `null`. On such a list "next" and
 * "previous" replay that item from its start, in sequential and in shuffle mode alike (CRA-181).
 */
export function soleCurrentItem<T>(
	items: readonly T[],
	currentKey: string | null,
	keyOf: (item: T) => string
): T | null {
	return items.length === 1 && currentKey !== null && keyOf(items[0]) === currentKey ? items[0] : null
}

// ---------------------------------------------------------------------------
// Shuffle session (no repeat until the pool is exhausted, history for previous)
// ---------------------------------------------------------------------------

export interface ShuffleSessionState {
	/** Keys in the order they were played. */
	history: readonly string[]
	/** Index in `history` of the item currently playing, -1 when there is no session. */
	position: number
	/** Keys already played since the pool was last (re)started. */
	played: readonly string[]
}

export interface ShuffleNextOptions {
	/**
	 * Also keep the current item out of the fresh pick, whether or not it is in
	 * the played set. The library does (a track played from outside the session
	 * may not be in the set); discovery relies on the played set alone.
	 */
	excludeCurrent?: boolean
}

export interface ShuffleSession<T> {
	/** Start a session anchored on `key` (the item playing now), or an empty one when `null`. */
	reset(key: string | null): void
	/**
	 * Tell the session which item the player has just loaded, whatever loaded it (a row, the Suggested panel, a
	 * modal, the restored track at launch, or the queue itself). The item the session itself handed out keeps the
	 * session as it is; any other item starts a new session anchored on it, so "previous" and the no-repeat pool
	 * always describe what was actually heard (CRA-181). `null` (nothing loaded) changes nothing.
	 */
	sync(key: string | null): void
	/**
	 * Next item to play, or `null` when there is nothing to play (the caller does nothing).
	 *
	 * 1. When the user went back, replay forward through the history. If the
	 *    forward item is no longer in `candidates`, fall through to a fresh pick.
	 * 2. Otherwise pick at random among candidates not yet played. When none is
	 *    left, restart the pool with only the current item played and pick again.
	 *
	 * A fresh pick is appended to the history and the position moves to it (the
	 * history is never truncated, even when the user had gone back).
	 */
	next(candidates: readonly T[], currentKey: string, options?: ShuffleNextOptions): T | null
	/**
	 * Step back through the history. `null` when already at the start, with no
	 * session, or when the previous item is no longer in `candidates`.
	 */
	previous(candidates: readonly T[]): T | null
	/**
	 * The item `next` or `previous` just handed out could not be played (its file is missing or unreadable): undo
	 * that move and forget the item — it leaves the history and the played set, so the following call offers
	 * something else and "previous" never lands on a track that was never heard (CRA-180).
	 */
	unplayable(key: string): void
	/** Copy of the internal state, for tests and debugging. */
	snapshot(): ShuffleSessionState
}

/** The last move of a session: which item it handed out, where the position was, and where it went. */
interface ShuffleMove {
	key: string
	fromPosition: number
	index: number
}

/**
 * Create a shuffle session over items identified by `keyOf`. `candidates` passed
 * to `next` / `previous` must already be restricted to playable items.
 */
export function createShuffleSession<T>(
	keyOf: (item: T) => string,
	random: RandomSource = Math.random
): ShuffleSession<T> {
	let history: string[] = []
	let position = -1
	let played = new Set<string>()
	// The last move, so `unplayable` can undo it exactly.
	let lastMove: ShuffleMove | null = null
	// Items handed out by next / previous whose start the player has not confirmed (`sync`) or refused
	// (`unplayable`) yet. Two quick "next" presses can confirm out of order; a confirmation of either keeps the session.
	const pending = new Set<string>()

	function reset(key: string | null) {
		history = key ? [key] : []
		position = key ? 0 : -1
		played = new Set(key ? [key] : [])
		lastMove = null
		pending.clear()
	}

	function moveTo(key: string, index: number) {
		lastMove = { key, fromPosition: position, index }
		position = index
		pending.add(key)
	}

	function sync(key: string | null) {
		if (key === null) return
		if (position >= 0 && history[position] === key) {
			pending.delete(key)
			return
		}
		if (pending.has(key)) {
			const index = history.lastIndexOf(key)
			if (index >= 0) {
				position = index
				pending.delete(key)
				return
			}
		}
		reset(key)
	}

	function next(candidates: readonly T[], currentKey: string, options: ShuffleNextOptions = {}): T | null {
		// Replay forward through history if the user previously went back.
		if (position < history.length - 1) {
			const forwardKey = history[position + 1]
			const forward = candidates.find((c) => keyOf(c) === forwardKey)
			if (forward !== undefined) {
				moveTo(forwardKey, position + 1)
				return forward
			}
		}

		// Fresh pick from the current bag.
		const excludeCurrent = options.excludeCurrent ?? false
		const isFresh = (c: T) => {
			const key = keyOf(c)
			return !(excludeCurrent && key === currentKey) && !played.has(key)
		}
		let pool = candidates.filter(isFresh)
		if (pool.length === 0) {
			// Bag exhausted: reshuffle, excluding only the current item.
			played = new Set([currentKey])
			pool = candidates.filter((c) => !played.has(keyOf(c)))
		}
		if (pool.length === 0) return null

		const pick = pool[Math.floor(random() * pool.length)]
		const pickKey = keyOf(pick)
		played.add(pickKey)
		history.push(pickKey)
		moveTo(pickKey, history.length - 1)
		return pick
	}

	function previous(candidates: readonly T[]): T | null {
		if (position > 0) {
			const previousKey = history[position - 1]
			const prev = candidates.find((c) => keyOf(c) === previousKey)
			if (prev !== undefined) {
				moveTo(previousKey, position - 1)
				return prev
			}
		}
		return null
	}

	function unplayable(key: string) {
		pending.delete(key)
		played.delete(key)
		if (lastMove && lastMove.key === key && position === lastMove.index && history[lastMove.index] === key) {
			// Undo the move exactly: drop the entry, and go back to the item still loaded (the one moved from,
			// shifted down by one when it sat after the dropped entry).
			const { fromPosition, index } = lastMove
			history.splice(index, 1)
			position = index < fromPosition ? fromPosition - 1 : fromPosition
			lastMove = null
			return
		}
		// A later move superseded this one: drop the most recent entry of the item, unless it is where we are now.
		const index = history.lastIndexOf(key)
		if (index >= 0 && index !== position) {
			history.splice(index, 1)
			if (index < position) position--
		}
	}

	function snapshot(): ShuffleSessionState {
		return { history: [...history], position, played: [...played] }
	}

	return { reset, sync, next, previous, unplayable, snapshot }
}

// ---------------------------------------------------------------------------
// Navigation with skips (library and album queues)
// ---------------------------------------------------------------------------

export interface QueueNavigation<T> {
	/** The queue, as it is now (see `resolveQueue`). */
	items: readonly T[]
	/** Key of the item loaded in the player, `null` when nothing is. */
	currentKey: string | null
	keyOf: (item: T) => string
	/** The shuffle session when shuffle is on, `null` when it is off. Shuffle needs a current item. */
	shuffle: ShuffleSession<T> | null
	/** Passed to `ShuffleSession.next` (the library keeps the current track out of a fresh pick). */
	excludeCurrent?: boolean
	/** Loads and plays an item; resolves `false` when it could not be played (and leaves the playing audio alone). */
	start: (item: T) => Promise<boolean>
	/** Brings the current item back to its start: what "previous" does when there is nothing to go back to. */
	restartCurrent: () => Promise<unknown> | unknown
	/** Checked before each retry: a newer navigation or a playback started elsewhere stops this one. */
	isCancelled?: () => boolean
	/** Failures in a row before giving up (default `MAX_CONSECUTIVE_LOAD_FAILURES`). */
	maxFailures?: number
}

export interface NavigationOutcome<T> {
	/** The item that started, `null` when none did. */
	started: T | null
	/** Items that could not be played, in the order they were tried. */
	skipped: T[]
	/** `maxFailures` items in a row failed. */
	gaveUp: boolean
	/** "Previous" had nothing to go back to and restarted the current item instead. */
	restarted: boolean
	/** A newer navigation took over before an item started. */
	cancelled: boolean
}

/**
 * Move to the next or previous item of a queue and start it, skipping items that fail to load (CRA-180).
 *
 * - Shuffle off: `sequentialPicker` order (each other item once, wrapping).
 * - Shuffle on: the session's `next` (which needs a current item; without one, sequential order) or `previous`; a
 *   failed item is reported to the session (`unplayable`) and left out of the candidates for the rest of this
 *   navigation.
 * - A one-item list whose item is the current one replays it, in both modes (CRA-181).
 * - "Previous" with nothing to go back to (shuffle with no history) restarts the current item (CRA-181).
 *
 * Stops after `maxFailures` failures in a row, when every candidate was tried, or when `isCancelled` turns true. The
 * audio that is playing is never touched by a failed attempt: `start` only replaces it once an item really started.
 */
export async function navigateQueue<T>(
	direction: QueueDirection,
	nav: QueueNavigation<T>
): Promise<NavigationOutcome<T>> {
	const { items, currentKey, keyOf, start } = nav
	const maxFailures = nav.maxFailures ?? MAX_CONSECUTIVE_LOAD_FAILURES
	const skipped: T[] = []
	const outcome = (started: T | null, flags: Partial<NavigationOutcome<T>> = {}): NavigationOutcome<T> => ({
		started,
		skipped,
		gaveUp: false,
		restarted: false,
		cancelled: false,
		...flags,
	})

	const pick = queuePicker(direction, nav)
	for (;;) {
		if (skipped.length >= maxFailures) return outcome(null, { gaveUp: true })
		if (skipped.length > 0 && nav.isCancelled?.()) return outcome(null, { cancelled: true })
		const item = pick.next()
		if (item === null) {
			if (skipped.length === 0 && direction === 'previous' && items.length > 0 && currentKey !== null) {
				await nav.restartCurrent()
				return outcome(null, { restarted: true })
			}
			return outcome(null)
		}
		if (await start(item)) return outcome(item)
		skipped.push(item)
		pick.failed(item)
	}
}

interface QueuePicker<T> {
	next(): T | null
	failed(item: T): void
}

function queuePicker<T>(direction: QueueDirection, nav: QueueNavigation<T>): QueuePicker<T> {
	const { items, currentKey, keyOf, shuffle } = nav
	const sole = soleCurrentItem(items, currentKey, keyOf)
	if (sole !== null) {
		let offered = false
		return {
			next: () => {
				if (offered) return null
				offered = true
				return sole
			},
			failed: () => {},
		}
	}
	// A fresh shuffle pick needs the current item; walking back the history does not (e.g. after Stop).
	if (shuffle === null || (currentKey === null && direction === 'next')) {
		const index = currentKey === null ? -1 : items.findIndex((item) => keyOf(item) === currentKey)
		return { next: sequentialPicker(items, index, direction), failed: () => {} }
	}
	const failed = new Set<string>()
	const candidates = () => (failed.size === 0 ? items : items.filter((item) => !failed.has(keyOf(item))))
	return {
		next: () =>
			currentKey !== null && direction === 'next'
				? shuffle.next(candidates(), currentKey, { excludeCurrent: nav.excludeCurrent })
				: shuffle.previous(candidates()),
		failed: (item) => {
			const key = keyOf(item)
			failed.add(key)
			shuffle.unplayable(key)
		},
	}
}

/** What to tell the user after a navigation that skipped tracks. `first` is the first track that failed. */
export type SkipNotice<T> =
	/** Playback moved on to another track: `first` was skipped, plus `others` more. */
	| { kind: 'skipped'; first: T; others: number }
	/** Nothing could be started, but the track that was playing still plays (a manual next / previous). */
	| { kind: 'kept'; first: T; count: number }
	/** Nothing could be started and nothing plays (the end of a track): `count` tracks in a row failed. */
	| { kind: 'stopped'; first: T; count: number }

/**
 * The notice for a navigation, `null` when nothing was skipped or when a newer navigation took over (that one
 * reports for both: holding the "next" key over missing files must not stack one toast per key repeat).
 * `stillPlaying` is whether the player is still playing the track it had before the navigation.
 */
export function skipNotice<T>(outcome: NavigationOutcome<T>, stillPlaying: boolean): SkipNotice<T> | null {
	if (outcome.skipped.length === 0 || outcome.cancelled) return null
	const first = outcome.skipped[0]
	if (outcome.started !== null) return { kind: 'skipped', first, others: outcome.skipped.length - 1 }
	return { kind: stillPlaying ? 'kept' : 'stopped', first, count: outcome.skipped.length }
}

/**
 * Lets a notice through unless the same one (same `key`) was let through or held back less than `windowMs` ago.
 * Every repeat restarts the window, so a burst of identical notices — key repeat over the same missing track — shows
 * once.
 */
export function createNoticeGate(windowMs: number, now: () => number = () => Date.now()): (key: string) => boolean {
	let lastKey: string | null = null
	let lastAt = Number.NEGATIVE_INFINITY
	return (key) => {
		const at = now()
		const repeat = key === lastKey && at - lastAt < windowMs
		lastKey = key
		lastAt = at
		return !repeat
	}
}

// ---------------------------------------------------------------------------
// Queue context (which list next / previous walk)
// ---------------------------------------------------------------------------

/** Where the user is: the view, and the playlist selected in it (`null` for the whole library). */
export interface QueueLocation {
	view: string
	playlistId: string | null
}

/** Where playback was started from, and the list as it was displayed there. */
export interface QueueContext<T> {
	location: QueueLocation
	snapshot: readonly T[]
}

function sameLocation(a: QueueLocation, b: QueueLocation): boolean {
	return a.view === b.view && a.playlistId === b.playlistId
}

/**
 * The list next / previous walk (CRA-181). While the user is still where playback started (same view, same
 * playlist), it is the live list as displayed — filtered and sorted — so a search, a filter or a new sort order
 * mid-play changes what comes next, on purpose: the queue is what the user sees. Once they navigate elsewhere, it is
 * the list as they left it. Without a context (playback not started from a list), the live list.
 */
export function resolveQueue<T>(
	context: QueueContext<T> | null,
	here: QueueLocation,
	live: readonly T[]
): readonly T[] {
	if (context === null) return live
	return sameLocation(context.location, here) ? live : context.snapshot
}

/**
 * The context to keep when the displayed list changes: its snapshot follows the list only while the user is still
 * where playback started, so it freezes as they navigate away.
 */
export function refreshQueueSnapshot<T>(
	context: QueueContext<T> | null,
	here: QueueLocation,
	live: readonly T[]
): QueueContext<T> | null {
	if (context === null || !sameLocation(context.location, here)) return context
	return { location: context.location, snapshot: live }
}

// ---------------------------------------------------------------------------
// Discovery previews (release -> tracks)
// ---------------------------------------------------------------------------

export interface PreviewRelease {
	id: string
	tracks: readonly unknown[]
}

export interface PreviewTarget<R extends PreviewRelease> {
	release: R
	trackIndex: number
}

/** Whether track `trackIndex` of `release` can be previewed. */
export type TrackPlayable<R extends PreviewRelease> = (release: R, trackIndex: number) => boolean

/** Shuffle key of a discovery track. */
export function discoveryTrackKey(releaseId: string, trackIndex: number): string {
	return `${releaseId}:${trackIndex}`
}

/** Every playable track of the queue, in release order then track order, as shuffle candidates. */
export function buildDiscoveryCandidates<R extends PreviewRelease>(
	releases: readonly R[],
	canPlay: TrackPlayable<R>
): Array<PreviewTarget<R>> {
	const candidates: Array<PreviewTarget<R>> = []
	for (const release of releases) {
		for (let i = 0; i < release.tracks.length; i++) {
			if (canPlay(release, i)) candidates.push({ release, trackIndex: i })
		}
	}
	return candidates
}

/** Shuffle key of a discovery candidate. */
export function previewTargetKey<R extends PreviewRelease>(target: PreviewTarget<R>): string {
	return discoveryTrackKey(target.release.id, target.trackIndex)
}

/** First or last playable track index of a release, -1 when none. */
export function findPlayableTrackIndex<R extends PreviewRelease>(
	release: R,
	direction: 'first' | 'last',
	canPlay: TrackPlayable<R>
): number {
	if (direction === 'first') {
		return release.tracks.findIndex((_, i) => canPlay(release, i))
	}
	for (let i = release.tracks.length - 1; i >= 0; i--) {
		if (canPlay(release, i)) return i
	}
	return -1
}

/**
 * Next preview with shuffle off: the next playable track of the current release
 * (`current.release`, the object being played), then the first playable track of
 * the following releases of the queue, wrapping around (the current release
 * itself is the last one tried). `null` when the current release is not in the
 * queue, the queue is empty, or nothing is playable.
 */
export function nextPreviewTarget<R extends PreviewRelease>(
	current: PreviewTarget<R>,
	releases: readonly R[],
	canPlay: TrackPlayable<R>
): PreviewTarget<R> | null {
	let nextIndex = current.trackIndex + 1
	while (nextIndex < current.release.tracks.length && !canPlay(current.release, nextIndex)) {
		nextIndex++
	}
	if (nextIndex < current.release.tracks.length) {
		return { release: current.release, trackIndex: nextIndex }
	}

	const releaseIdx = releases.findIndex((r) => r.id === current.release.id)
	if (releaseIdx === -1 || releases.length === 0) return null

	for (let i = 1; i <= releases.length; i++) {
		const nextRelease = releases[(releaseIdx + i) % releases.length]
		const trackIdx = findPlayableTrackIndex(nextRelease, 'first', canPlay)
		if (trackIdx !== -1) return { release: nextRelease, trackIndex: trackIdx }
	}
	return null
}

/** Mirror of {@link nextPreviewTarget}: previous playable track, then the last playable track of the previous releases. */
export function previousPreviewTarget<R extends PreviewRelease>(
	current: PreviewTarget<R>,
	releases: readonly R[],
	canPlay: TrackPlayable<R>
): PreviewTarget<R> | null {
	let prevIndex = current.trackIndex - 1
	while (prevIndex >= 0 && !canPlay(current.release, prevIndex)) {
		prevIndex--
	}
	if (prevIndex >= 0) {
		return { release: current.release, trackIndex: prevIndex }
	}

	const releaseIdx = releases.findIndex((r) => r.id === current.release.id)
	if (releaseIdx === -1 || releases.length === 0) return null

	for (let i = 1; i <= releases.length; i++) {
		const prevRelease = releases[(releaseIdx - i + releases.length) % releases.length]
		const trackIdx = findPlayableTrackIndex(prevRelease, 'last', canPlay)
		if (trackIdx !== -1) return { release: prevRelease, trackIndex: trackIdx }
	}
	return null
}
