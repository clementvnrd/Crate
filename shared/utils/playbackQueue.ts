/**
 * Pure next / previous / shuffle logic for the player queues (CRA-167).
 *
 * Extracted verbatim from `createAppSetup` so it can be unit-tested with a
 * deterministic random source. Nothing here touches a store or the player: the
 * caller passes the candidates in, gets the chosen item back, and does the
 * loading and playing itself.
 *
 * Behaviour is deliberately identical to what `createAppSetup` did before the
 * extraction, quirks included (see the notes on each function).
 */

/** Returns a number in [0, 1), like `Math.random`. Injected so tests are deterministic. */
export type RandomSource = () => number

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
	/** Copy of the internal state, for tests and debugging. */
	snapshot(): ShuffleSessionState
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

	function reset(key: string | null) {
		history = key ? [key] : []
		position = key ? 0 : -1
		played = new Set(key ? [key] : [])
	}

	function next(candidates: readonly T[], currentKey: string, options: ShuffleNextOptions = {}): T | null {
		// Replay forward through history if the user previously went back.
		if (position < history.length - 1) {
			const forwardKey = history[position + 1]
			const forward = candidates.find((c) => keyOf(c) === forwardKey)
			if (forward !== undefined) {
				position++
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
		position = history.length - 1
		return pick
	}

	function previous(candidates: readonly T[]): T | null {
		if (position > 0) {
			const previousKey = history[position - 1]
			const prev = candidates.find((c) => keyOf(c) === previousKey)
			if (prev !== undefined) {
				position--
				return prev
			}
		}
		return null
	}

	function snapshot(): ShuffleSessionState {
		return { history: [...history], position, played: [...played] }
	}

	return { reset, next, previous, snapshot }
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
