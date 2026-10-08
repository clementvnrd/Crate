import { describe, it, expect, vi } from 'vitest'
import {
	MAX_CONSECUTIVE_LOAD_FAILURES,
	buildDiscoveryCandidates,
	createNoticeGate,
	createShuffleSession,
	discoveryTrackKey,
	findPlayableTrackIndex,
	navigateQueue,
	nextPreviewTarget,
	previewTargetKey,
	previousPreviewTarget,
	refreshQueueSnapshot,
	resolveQueue,
	sequentialNextIndex,
	sequentialPicker,
	sequentialPreviousIndex,
	skipNotice,
	soleCurrentItem,
	type NavigationOutcome,
	type PreviewRelease,
	type PreviewTarget,
	type QueueContext,
	type QueueNavigation,
	type RandomSource,
	type ShuffleSession,
} from './playbackQueue'

interface T {
	id: string
}

const tr = (...ids: string[]): T[] => ids.map((id) => ({ id }))

/** Random source that returns the given values in order, then repeats the last one. */
function scripted(...values: number[]): RandomSource {
	let i = 0
	return () => values[Math.min(i++, values.length - 1)]
}

/** Always picks the first item of the pool. */
const first: RandomSource = () => 0
/** Always picks the last item of the pool. */
const last: RandomSource = () => 0.999999

const keyOf = (t: T) => t.id

describe('sequential navigation', () => {
	it('moves to the next index', () => {
		expect(sequentialNextIndex(0, 3)).toBe(1)
		expect(sequentialNextIndex(1, 3)).toBe(2)
	})

	it('wraps from the last item to the first', () => {
		expect(sequentialNextIndex(2, 3)).toBe(0)
	})

	it('starts at the first item when the current one is unknown', () => {
		expect(sequentialNextIndex(-1, 3)).toBe(0)
	})

	it('stays on the only item of a one-item list', () => {
		expect(sequentialNextIndex(0, 1)).toBe(0)
		expect(sequentialPreviousIndex(0, 1)).toBe(0)
	})

	it('moves to the previous index', () => {
		expect(sequentialPreviousIndex(2, 3)).toBe(1)
		expect(sequentialPreviousIndex(1, 3)).toBe(0)
	})

	it('wraps from the first item to the last', () => {
		expect(sequentialPreviousIndex(0, 3)).toBe(2)
	})

	it('starts at the last item when the current one is unknown', () => {
		expect(sequentialPreviousIndex(-1, 3)).toBe(2)
	})
})

describe('createShuffleSession', () => {
	describe('reset', () => {
		it('anchors the session on the given key', () => {
			const s = createShuffleSession<T>(keyOf)
			s.reset('a')
			expect(s.snapshot()).toEqual({ history: ['a'], position: 0, played: ['a'] })
		})

		it('empties the session for null', () => {
			const s = createShuffleSession<T>(keyOf)
			s.reset('a')
			s.reset(null)
			expect(s.snapshot()).toEqual({ history: [], position: -1, played: [] })
		})

		it('discards the history when a track is played from outside', () => {
			const s = createShuffleSession<T>(keyOf, first)
			const tracks = tr('a', 'b', 'c', 'd')
			s.reset('a')
			s.next(tracks, 'a', { excludeCurrent: true }) // b
			s.next(tracks, 'b', { excludeCurrent: true }) // c
			s.reset('d')
			expect(s.snapshot()).toEqual({ history: ['d'], position: 0, played: ['d'] })
			// "previous" has nowhere to go after the reset.
			expect(s.previous(tracks)).toBeNull()
		})

		it('makes the next pick forget what was already played', () => {
			const s = createShuffleSession<T>(keyOf, first)
			const tracks = tr('a', 'b', 'c')
			s.reset('a')
			expect(s.next(tracks, 'a')?.id).toBe('b')
			s.reset('c')
			// Only c is played now, so a (not b) is the first candidate again.
			expect(s.next(tracks, 'c')?.id).toBe('a')
		})
	})

	describe('next', () => {
		it('never picks the current track', () => {
			const tracks = tr('a', 'b', 'c')
			for (const rng of [first, last, scripted(0.5)]) {
				const s = createShuffleSession<T>(keyOf, rng)
				s.reset('b')
				expect(s.next(tracks, 'b', { excludeCurrent: true })?.id).not.toBe('b')
			}
		})

		it('uses the injected random source to choose within the pool', () => {
			const tracks = tr('a', 'b', 'c', 'd')
			const lo = createShuffleSession<T>(keyOf, first)
			lo.reset('a')
			expect(lo.next(tracks, 'a')?.id).toBe('b')
			const hi = createShuffleSession<T>(keyOf, last)
			hi.reset('a')
			expect(hi.next(tracks, 'a')?.id).toBe('d')
			const mid = createShuffleSession<T>(keyOf, () => 0.5)
			mid.reset('a')
			// pool is [b, c, d], floor(0.5 * 3) = 1
			expect(mid.next(tracks, 'a')?.id).toBe('c')
		})

		it('does not repeat a track until the pool is exhausted', () => {
			const tracks = tr('a', 'b', 'c', 'd', 'e')
			const s = createShuffleSession<T>(keyOf, last)
			s.reset('a')
			const seen = ['a']
			let current = 'a'
			for (let i = 0; i < 4; i++) {
				const pick = s.next(tracks, current, { excludeCurrent: true })!
				seen.push(pick.id)
				current = pick.id
			}
			expect([...seen].sort()).toEqual(['a', 'b', 'c', 'd', 'e'])
		})

		it('restarts the pool, excluding only the current track, once exhausted', () => {
			const tracks = tr('a', 'b', 'c')
			const s = createShuffleSession<T>(keyOf, first)
			s.reset('a')
			expect(s.next(tracks, 'a')?.id).toBe('b')
			expect(s.next(tracks, 'b')?.id).toBe('c')
			// a, b, c all played: restart with only c played, so a is the first candidate.
			expect(s.next(tracks, 'c')?.id).toBe('a')
			expect(s.snapshot().played).toEqual(['c', 'a'])
			expect(s.next(tracks, 'a')?.id).toBe('b')
		})

		it('returns null on a one-track list (nothing else to play)', () => {
			const s = createShuffleSession<T>(keyOf, first)
			s.reset('a')
			expect(s.next(tracks1(), 'a', { excludeCurrent: true })).toBeNull()
			// the session is left untouched apart from the pool restart
			expect(s.snapshot().history).toEqual(['a'])
			expect(s.snapshot().position).toBe(0)
		})

		it('returns null on an empty list', () => {
			const s = createShuffleSession<T>(keyOf, first)
			s.reset('a')
			expect(s.next([], 'a')).toBeNull()
		})

		it('returns null on a session with no anchor and an empty list', () => {
			const s = createShuffleSession<T>(keyOf, first)
			expect(s.next([], 'x')).toBeNull()
		})

		it('records the pick in the history and moves the position to it', () => {
			const tracks = tr('a', 'b', 'c')
			const s = createShuffleSession<T>(keyOf, first)
			s.reset('a')
			s.next(tracks, 'a')
			expect(s.snapshot()).toEqual({ history: ['a', 'b'], position: 1, played: ['a', 'b'] })
		})

		it('excludeCurrent keeps an unplayed current track out of the pick', () => {
			const tracks = tr('a', 'b', 'c')
			// 'c' is playing but was never registered in the session (played from outside).
			const s = createShuffleSession<T>(keyOf, last)
			s.reset('a')
			expect(s.next(tracks, 'c', { excludeCurrent: true })?.id).toBe('b')
		})

		it('without excludeCurrent, an unplayed current track can be picked (discovery behaviour)', () => {
			const tracks = tr('a', 'b', 'c')
			const s = createShuffleSession<T>(keyOf, last)
			s.reset('a')
			expect(s.next(tracks, 'c')?.id).toBe('c')
		})
	})

	describe('previous and history', () => {
		it('steps back through the actual play order', () => {
			const tracks = tr('a', 'b', 'c', 'd')
			const s = createShuffleSession<T>(keyOf, last)
			s.reset('a')
			const p1 = s.next(tracks, 'a', { excludeCurrent: true })! // d
			const p2 = s.next(tracks, p1.id, { excludeCurrent: true })! // c
			expect([p1.id, p2.id]).toEqual(['d', 'c'])
			expect(s.previous(tracks)?.id).toBe('d')
			expect(s.previous(tracks)?.id).toBe('a')
			expect(s.snapshot().position).toBe(0)
		})

		it('returns null at the start of the history', () => {
			const tracks = tr('a', 'b')
			const s = createShuffleSession<T>(keyOf, first)
			s.reset('a')
			expect(s.previous(tracks)).toBeNull()
			expect(s.snapshot().position).toBe(0)
		})

		it('returns null when there is no session', () => {
			const s = createShuffleSession<T>(keyOf, first)
			expect(s.previous(tr('a'))).toBeNull()
		})

		it('returns null (and stays put) when the previous track left the candidates', () => {
			const tracks = tr('a', 'b', 'c')
			const s = createShuffleSession<T>(keyOf, first)
			s.reset('a')
			s.next(tracks, 'a') // b
			expect(s.previous(tr('c'))).toBeNull()
			expect(s.snapshot().position).toBe(1)
		})

		it('replays forward through the history after going back, without a new pick', () => {
			const tracks = tr('a', 'b', 'c', 'd')
			let calls = 0
			const s = createShuffleSession<T>(keyOf, () => {
				calls++
				return 0
			})
			s.reset('a')
			s.next(tracks, 'a') // b
			s.next(tracks, 'b') // c
			expect(calls).toBe(2)
			s.previous(tracks) // back to b
			s.previous(tracks) // back to a
			expect(s.next(tracks, 'a')?.id).toBe('b')
			expect(s.next(tracks, 'b')?.id).toBe('c')
			expect(calls).toBe(2)
			expect(s.snapshot()).toEqual({ history: ['a', 'b', 'c'], position: 2, played: ['a', 'b', 'c'] })
			// at the end of the history the next call picks fresh again
			expect(s.next(tracks, 'c')?.id).toBe('d')
			expect(calls).toBe(3)
		})

		it('falls back to a fresh pick when the forward track left the candidates', () => {
			const tracks = tr('a', 'b', 'c', 'd')
			const s = createShuffleSession<T>(keyOf, first)
			s.reset('a')
			s.next(tracks, 'a') // b
			s.previous(tracks) // back to a, forward would be b
			// b is gone from the queue
			const pick = s.next(tr('a', 'c', 'd'), 'a', { excludeCurrent: true })
			expect(pick?.id).toBe('c')
			// the fresh pick is appended after the stale forward entry (history is never truncated)
			expect(s.snapshot().history).toEqual(['a', 'b', 'c'])
			expect(s.snapshot().position).toBe(2)
		})
	})
})

function tracks1(): T[] {
	return tr('a')
}

// ---------------------------------------------------------------------------
// Sequential picker and one-track lists (CRA-180, CRA-181)
// ---------------------------------------------------------------------------

/** Every candidate a picker offers, in order. */
function drain<X>(pick: () => X | null): X[] {
	const out: X[] = []
	for (let item = pick(); item !== null; item = pick()) out.push(item)
	return out
}

const ids = (items: readonly T[]) => items.map((t) => t.id)

describe('sequentialPicker', () => {
	const tracks = tr('a', 'b', 'c', 'd')

	it('offers every other track once after the current one, wrapping, never the current one', () => {
		expect(ids(drain(sequentialPicker(tracks, 1, 'next')))).toEqual(['c', 'd', 'a'])
	})

	it('walks backwards for previous, wrapping', () => {
		expect(ids(drain(sequentialPicker(tracks, 1, 'previous')))).toEqual(['a', 'd', 'c'])
	})

	it('offers every track when the current one is unknown, from the first (next) or the last (previous)', () => {
		expect(ids(drain(sequentialPicker(tracks, -1, 'next')))).toEqual(['a', 'b', 'c', 'd'])
		expect(ids(drain(sequentialPicker(tracks, -1, 'previous')))).toEqual(['d', 'c', 'b', 'a'])
	})

	it('loops once on the only track of a one-track list', () => {
		expect(ids(drain(sequentialPicker(tr('a'), 0, 'next')))).toEqual(['a'])
		expect(ids(drain(sequentialPicker(tr('a'), 0, 'previous')))).toEqual(['a'])
	})

	it('offers nothing on an empty list', () => {
		expect(drain(sequentialPicker([], -1, 'next'))).toEqual([])
	})

	it('starts with what sequentialNextIndex / sequentialPreviousIndex choose', () => {
		for (let current = -1; current < tracks.length; current++) {
			expect(sequentialPicker(tracks, current, 'next')()).toBe(tracks[sequentialNextIndex(current, tracks.length)])
			expect(sequentialPicker(tracks, current, 'previous')()).toBe(
				tracks[sequentialPreviousIndex(current, tracks.length)]
			)
		}
	})
})

describe('soleCurrentItem', () => {
	it('is the only track when it is the one playing', () => {
		expect(soleCurrentItem(tr('a'), 'a', keyOf)?.id).toBe('a')
	})

	it('is null for longer lists, another track, or nothing playing', () => {
		expect(soleCurrentItem(tr('a', 'b'), 'a', keyOf)).toBeNull()
		expect(soleCurrentItem(tr('b'), 'a', keyOf)).toBeNull()
		expect(soleCurrentItem(tr('a'), null, keyOf)).toBeNull()
		expect(soleCurrentItem([], 'a', keyOf)).toBeNull()
	})
})

// ---------------------------------------------------------------------------
// Shuffle session: sync and unplayable (CRA-180, CRA-181)
// ---------------------------------------------------------------------------

describe('createShuffleSession — sync (every playback start)', () => {
	it('anchors a new session on a track started from outside the session', () => {
		const s = createShuffleSession<T>(keyOf, first)
		const tracks = tr('a', 'b', 'c')
		s.sync('a')
		s.next(tracks, 'a', { excludeCurrent: true }) // b
		s.sync('b')
		// Started from the Suggested panel, a modal…: a fresh session on it.
		s.sync('c')
		expect(s.snapshot()).toEqual({ history: ['c'], position: 0, played: ['c'] })
	})

	it("keeps the session for the track it handed out itself, so 'previous' still goes back", () => {
		const s = createShuffleSession<T>(keyOf, first)
		const tracks = tr('a', 'b', 'c')
		s.sync('a')
		const pick = s.next(tracks, 'a', { excludeCurrent: true })!
		s.sync(pick.id)
		expect(s.snapshot()).toEqual({ history: ['a', 'b'], position: 1, played: ['a', 'b'] })
		expect(s.previous(tracks)?.id).toBe('a')
		s.sync('a')
		expect(s.snapshot().position).toBe(0)
	})

	it('ignores "nothing loaded"', () => {
		const s = createShuffleSession<T>(keyOf, first)
		s.sync('a')
		s.sync(null)
		expect(s.snapshot()).toEqual({ history: ['a'], position: 0, played: ['a'] })
	})

	it('keeps the history when two quick picks start out of order', () => {
		const s = createShuffleSession<T>(keyOf, first)
		const tracks = tr('a', 'b', 'c', 'd')
		s.sync('a')
		const b = s.next(tracks, 'a', { excludeCurrent: true })! // b, not started yet
		const c = s.next(tracks, 'a', { excludeCurrent: true })! // c, pressed again before b started
		s.sync(b.id)
		s.sync(c.id)
		expect(s.snapshot()).toEqual({ history: ['a', 'b', 'c'], position: 2, played: ['a', 'b', 'c'] })
	})

	it('launch with shuffle on: the restored track anchors the session, so "previous" goes back to it', () => {
		// The shuffle subscription runs first, with nothing loaded yet, then the last track is restored.
		const s = createShuffleSession<T>(keyOf, first)
		const tracks = tr('a', 'b', 'c')
		s.reset(null)
		s.sync('b') // restored at launch (then started with Space: same track, nothing new to sync)
		const pick = s.next(tracks, 'b', { excludeCurrent: true })!
		s.sync(pick.id)
		expect(s.previous(tracks)?.id).toBe('b')
	})
})

describe('createShuffleSession — unplayable (a pick that failed to load)', () => {
	it('undoes a fresh pick: out of the history and of the played set', () => {
		const s = createShuffleSession<T>(keyOf, first)
		const tracks = tr('a', 'b', 'c')
		s.reset('a')
		expect(s.next(tracks, 'a', { excludeCurrent: true })?.id).toBe('b')
		s.unplayable('b')
		expect(s.snapshot()).toEqual({ history: ['a'], position: 0, played: ['a'] })
	})

	it('after a pool restart, a pick that fails is not counted as played', () => {
		const s = createShuffleSession<T>(keyOf, first)
		const tracks = tr('a', 'b')
		s.reset('a')
		s.next(tracks, 'a', { excludeCurrent: true }) // b
		s.sync('b')
		// Pool exhausted: restart, a is picked, and fails.
		expect(s.next(tracks, 'b', { excludeCurrent: true })?.id).toBe('a')
		s.unplayable('a')
		expect(s.snapshot()).toEqual({ history: ['a', 'b'], position: 1, played: ['b'] })
	})

	it('drops a history entry replayed forward, and the next call moves past it', () => {
		const s = createShuffleSession<T>(keyOf, first)
		const tracks = tr('a', 'b', 'c', 'd')
		s.reset('a')
		s.next(tracks, 'a') // b
		s.next(tracks, 'b') // c
		s.previous(tracks) // b
		s.previous(tracks) // a
		expect(s.next(tracks, 'a')?.id).toBe('b') // forward replay
		s.unplayable('b')
		expect(s.snapshot().history).toEqual(['a', 'c'])
		expect(s.snapshot().position).toBe(0)
		expect(s.next(tr('a', 'c', 'd'), 'a')?.id).toBe('c')
	})

	it('drops a history entry stepped back to, and "previous" then goes further back', () => {
		const s = createShuffleSession<T>(keyOf, first)
		const tracks = tr('a', 'b', 'c', 'd')
		s.reset('a')
		s.next(tracks, 'a') // b
		s.next(tracks, 'b') // c, playing
		expect(s.previous(tracks)?.id).toBe('b')
		s.unplayable('b')
		expect(s.snapshot().history).toEqual(['a', 'c'])
		expect(s.snapshot().position).toBe(1) // still on c, which keeps playing
		expect(s.previous(tr('a', 'c', 'd'))?.id).toBe('a')
	})

	it('only forgets the item when a later move superseded the failed one', () => {
		const s = createShuffleSession<T>(keyOf, first)
		const tracks = tr('a', 'b', 'c', 'd')
		s.reset('a')
		s.next(tracks, 'a') // b (its load is slow…)
		s.next(tracks, 'a') // c (…next pressed again)
		s.unplayable('b')
		expect(s.snapshot()).toEqual({ history: ['a', 'c'], position: 1, played: ['a', 'c'] })
	})
})

// ---------------------------------------------------------------------------
// navigateQueue (CRA-180 skips, CRA-181 fallbacks)
// ---------------------------------------------------------------------------

interface Harness {
	started: string[]
	restarts: number
	nav: QueueNavigation<T>
}

/** A queue over `items` where the tracks in `missing` fail to load; `start` records every attempt. */
function queue(
	items: T[],
	currentKey: string | null,
	options: { missing?: string[]; shuffle?: ShuffleSession<T> | null; maxFailures?: number; cancelAfter?: number } = {}
): Harness {
	const missing = new Set(options.missing ?? [])
	const h: Harness = {
		started: [],
		restarts: 0,
		nav: {
			items,
			currentKey,
			keyOf,
			shuffle: options.shuffle ?? null,
			excludeCurrent: true,
			start: async (item) => {
				h.started.push(item.id)
				return !missing.has(item.id)
			},
			restartCurrent: () => {
				h.restarts++
			},
			isCancelled: options.cancelAfter === undefined ? undefined : () => h.started.length >= options.cancelAfter!,
			maxFailures: options.maxFailures,
		},
	}
	return h
}

describe('navigateQueue — sequential', () => {
	it('plays the next track', async () => {
		const h = queue(tr('a', 'b', 'c'), 'a')
		const out = await navigateQueue('next', h.nav)
		expect(out.started?.id).toBe('b')
		expect(out.skipped).toEqual([])
		expect(h.started).toEqual(['b'])
	})

	it('skips a missing track in the middle of the queue and plays the following one', async () => {
		const h = queue(tr('a', 'b', 'c', 'd'), 'a', { missing: ['b'] })
		const out = await navigateQueue('next', h.nav)
		expect(h.started).toEqual(['b', 'c'])
		expect(out.started?.id).toBe('c')
		expect(ids(out.skipped)).toEqual(['b'])
		expect(out.gaveUp).toBe(false)
	})

	it('skips backwards for previous', async () => {
		const h = queue(tr('a', 'b', 'c', 'd'), 'c', { missing: ['b'] })
		const out = await navigateQueue('previous', h.nav)
		expect(h.started).toEqual(['b', 'a'])
		expect(out.started?.id).toBe('a')
	})

	it('stops once every other track failed, without restarting the current one', async () => {
		const h = queue(tr('a', 'b', 'c'), 'a', { missing: ['b', 'c'] })
		const out = await navigateQueue('next', h.nav)
		expect(h.started).toEqual(['b', 'c'])
		expect(out.started).toBeNull()
		expect(ids(out.skipped)).toEqual(['b', 'c'])
		expect(out.gaveUp).toBe(false)
		expect(h.restarts).toBe(0)
	})

	it(`gives up after ${MAX_CONSECUTIVE_LOAD_FAILURES} failures in a row in a long queue`, async () => {
		const items = Array.from({ length: 40 }, (_, i) => ({ id: `t${i}` }))
		const h = queue(
			items,
			't0',
			{ missing: items.slice(1).map((t) => t.id) } // only the current track is still there
		)
		const out = await navigateQueue('next', h.nav)
		expect(h.started).toHaveLength(MAX_CONSECUTIVE_LOAD_FAILURES)
		expect(out.started).toBeNull()
		expect(out.gaveUp).toBe(true)
		expect(out.skipped).toHaveLength(MAX_CONSECUTIVE_LOAD_FAILURES)
	})

	it('honours a custom bound', async () => {
		const h = queue(tr('a', 'b', 'c', 'd'), 'a', { missing: ['b', 'c', 'd'], maxFailures: 2 })
		const out = await navigateQueue('next', h.nav)
		expect(h.started).toEqual(['b', 'c'])
		expect(out.gaveUp).toBe(true)
	})

	it('stops retrying when a newer navigation took over', async () => {
		const h = queue(tr('a', 'b', 'c', 'd'), 'a', { missing: ['b', 'c'], cancelAfter: 1 })
		const out = await navigateQueue('next', h.nav)
		expect(h.started).toEqual(['b'])
		expect(out.cancelled).toBe(true)
		expect(out.started).toBeNull()
	})

	it('starts at the first track when nothing is loaded', async () => {
		const h = queue(tr('a', 'b'), null)
		expect((await navigateQueue('next', h.nav)).started?.id).toBe('a')
	})

	it('does nothing on an empty queue', async () => {
		const h = queue([], 'a')
		const out = await navigateQueue('previous', h.nav)
		expect(out).toEqual({ started: null, skipped: [], gaveUp: false, restarted: false, cancelled: false })
		expect(h.restarts).toBe(0)
	})
})

describe('navigateQueue — shuffle', () => {
	function session(rng: RandomSource = first, anchor = 'a'): ShuffleSession<T> {
		const s = createShuffleSession<T>(keyOf, rng)
		s.sync(anchor)
		return s
	}

	/** Plays like the app: each started track is reported to the session (the player's `sync`). */
	async function navigate(direction: 'next' | 'previous', h: Harness): Promise<NavigationOutcome<T>> {
		const out = await navigateQueue(direction, h.nav)
		if (out.started) h.nav.shuffle?.sync(out.started.id)
		if (out.started) h.nav.currentKey = out.started.id
		return out
	}

	it('skips a missing pick, never counts it as played, and plays another one', async () => {
		const s = session()
		const h = queue(tr('a', 'b', 'c', 'd'), 'a', { missing: ['b'], shuffle: s })
		const out = await navigate('next', h)
		expect(h.started).toEqual(['b', 'c'])
		expect(out.started?.id).toBe('c')
		expect(s.snapshot()).toEqual({ history: ['a', 'c'], position: 1, played: ['a', 'c'] })
		// "previous" goes back to what was heard, never to the missing track.
		expect((await navigate('previous', h)).started?.id).toBe('a')
	})

	it('stops with every other track missing, the session back where it was', async () => {
		const s = session()
		const h = queue(tr('a', 'b', 'c'), 'a', { missing: ['b', 'c'], shuffle: s })
		const out = await navigate('next', h)
		expect(h.started.sort()).toEqual(['b', 'c'])
		expect(out.started).toBeNull()
		expect(s.snapshot()).toEqual({ history: ['a'], position: 0, played: ['a'] })
	})

	it('gives up after the bound on a long all-missing queue', async () => {
		const items = Array.from({ length: 30 }, (_, i) => ({ id: `t${i}` }))
		const s = session(first, 't0')
		const h = queue(items, 't0', { missing: items.slice(1).map((t) => t.id), shuffle: s })
		const out = await navigate('next', h)
		expect(out.gaveUp).toBe(true)
		expect(new Set(h.started).size).toBe(MAX_CONSECUTIVE_LOAD_FAILURES)
		expect(s.snapshot().played).toEqual(['t0'])
	})

	it('never repeats a track before every track was played (album or library)', async () => {
		const items = tr('a', 'b', 'c', 'd', 'e', 'f')
		const s = session(scripted(0.7, 0.2, 0.9, 0.4, 0.1))
		const h = queue(items, 'a', { shuffle: s })
		const heard = ['a']
		for (let i = 0; i < items.length - 1; i++) heard.push((await navigate('next', h)).started!.id)
		expect([...heard].sort()).toEqual(['a', 'b', 'c', 'd', 'e', 'f'])
	})

	it('"previous" with no history restarts the current track', async () => {
		const s = session()
		const h = queue(tr('a', 'b', 'c'), 'a', { shuffle: s })
		const out = await navigate('previous', h)
		expect(out.restarted).toBe(true)
		expect(h.restarts).toBe(1)
		expect(h.started).toEqual([])
	})

	it('"previous" skips a history entry whose file went missing', async () => {
		const s = session()
		const h = queue(tr('a', 'b', 'c', 'd'), 'a', { shuffle: s })
		await navigate('next', h) // b
		await navigate('next', h) // c
		const missing = queue(tr('a', 'b', 'c', 'd'), 'c', { missing: ['b'], shuffle: s })
		const out = await navigate('previous', missing)
		expect(missing.started).toEqual(['b', 'a'])
		expect(out.started?.id).toBe('a')
	})

	it('without a current track, "next" falls back to sequential order', async () => {
		const s = session()
		const h = queue(tr('a', 'b'), null, { shuffle: s })
		expect((await navigateQueue('next', h.nav)).started?.id).toBe('a')
	})

	it('without a current track (after Stop), "previous" still walks back the history', async () => {
		const s = session()
		const h = queue(tr('a', 'b', 'c'), 'a', { shuffle: s })
		await navigate('next', h) // b
		const stopped = queue(tr('a', 'b', 'c'), null, { shuffle: s })
		const out = await navigateQueue('previous', stopped.nav)
		expect(out.started?.id).toBe('a')
		expect(stopped.restarts).toBe(0)
	})
})

describe('navigateQueue — one-track list (same intent in both modes)', () => {
	for (const mode of ['sequential', 'shuffle'] as const) {
		for (const direction of ['next', 'previous'] as const) {
			it(`${mode} ${direction} replays the only track from its start`, async () => {
				const shuffle = mode === 'shuffle' ? createShuffleSession<T>(keyOf, first) : null
				shuffle?.sync('a')
				const h = queue(tr('a'), 'a', { shuffle })
				const out = await navigateQueue(direction, h.nav)
				expect(h.started).toEqual(['a'])
				expect(out.started?.id).toBe('a')
				expect(h.restarts).toBe(0)
				// The session is untouched: nothing new was heard.
				if (shuffle) expect(shuffle.snapshot()).toEqual({ history: ['a'], position: 0, played: ['a'] })
			})
		}
	}

	it('a one-track list whose track fails stops with it skipped', async () => {
		const h = queue(tr('a'), 'a', { missing: ['a'] })
		const out = await navigateQueue('next', h.nav)
		expect(ids(out.skipped)).toEqual(['a'])
		expect(out.started).toBeNull()
	})
})

describe('skipNotice', () => {
	const base: NavigationOutcome<T> = { started: null, skipped: [], gaveUp: false, restarted: false, cancelled: false }

	it('says nothing when nothing was skipped', () => {
		expect(skipNotice({ ...base, started: { id: 'b' } }, true)).toBeNull()
		expect(skipNotice(base, false)).toBeNull()
	})

	it('names the first skipped track and counts the others when playback moved on', () => {
		expect(skipNotice({ ...base, started: { id: 'd' }, skipped: tr('b') }, true)).toEqual({
			kind: 'skipped',
			first: { id: 'b' },
			others: 0,
		})
		expect(skipNotice({ ...base, started: { id: 'd' }, skipped: tr('b', 'c') }, false)).toEqual({
			kind: 'skipped',
			first: { id: 'b' },
			others: 1,
		})
	})

	it('says the current track keeps playing when nothing else could start during playback (manual next)', () => {
		expect(skipNotice({ ...base, skipped: tr('b', 'c'), gaveUp: true }, true)).toEqual({
			kind: 'kept',
			first: { id: 'b' },
			count: 2,
		})
	})

	it('reports a stop only when nothing plays any more (end of a track), at the bound or not', () => {
		expect(skipNotice({ ...base, skipped: tr('b', 'c'), gaveUp: true }, false)).toEqual({
			kind: 'stopped',
			first: { id: 'b' },
			count: 2,
		})
		expect(skipNotice({ ...base, skipped: tr('b') }, false)?.kind).toBe('stopped')
	})

	it('says nothing for a navigation a newer one took over (that one reports)', () => {
		expect(skipNotice({ ...base, skipped: tr('b'), cancelled: true }, true)).toBeNull()
		expect(skipNotice({ ...base, skipped: tr('b'), cancelled: true }, false)).toBeNull()
	})
})

describe('createNoticeGate', () => {
	it('lets a notice through once while the same one keeps coming, then again after a quiet window', () => {
		let now = 0
		const gate = createNoticeGate(5_000, () => now)
		expect(gate('kept:b')).toBe(true)
		now = 100
		expect(gate('kept:b')).toBe(false) // key repeat
		now = 4_000
		expect(gate('kept:b')).toBe(false) // still repeating: the window restarts
		now = 9_100
		expect(gate('kept:b')).toBe(true) // quiet for 5 s
	})

	it('always lets a different notice through', () => {
		let now = 0
		const gate = createNoticeGate(5_000, () => now)
		expect(gate('skipped:b')).toBe(true)
		now = 10
		expect(gate('skipped:c')).toBe(true)
		expect(gate('kept:c')).toBe(true)
	})
})

// ---------------------------------------------------------------------------
// Queue context (CRA-181, point 6: the queue is the list as displayed)
// ---------------------------------------------------------------------------

describe('resolveQueue / refreshQueueSnapshot', () => {
	const library = { view: 'library', playlistId: null }
	const playlist = { view: 'library', playlistId: 'pl-1' }
	const discovery = { view: 'discovery', playlistId: null }

	it('uses the live list without a context (playback not started from a list)', () => {
		expect(resolveQueue(null, library, tr('a'))).toEqual(tr('a'))
	})

	it('follows the displayed list while the user stays where playback started: a filter change mid-play changes what comes next', () => {
		const started = tr('a', 'b', 'c', 'd')
		let context: QueueContext<T> | null = { location: library, snapshot: started }
		const filtered = tr('a', 'c')
		context = refreshQueueSnapshot(context, library, filtered)
		expect(resolveQueue(context, library, filtered)).toEqual(filtered)
		expect(context?.snapshot).toEqual(filtered)
	})

	it('freezes the list as it was when the user navigates away, and follows it again on return', () => {
		const started = tr('a', 'b', 'c')
		let context: QueueContext<T> | null = { location: library, snapshot: started }
		// In a playlist, the displayed list is another one: the snapshot is kept.
		const playlistTracks = tr('x', 'y')
		context = refreshQueueSnapshot(context, playlist, playlistTracks)
		expect(resolveQueue(context, playlist, playlistTracks)).toEqual(started)
		expect(resolveQueue(context, discovery, [])).toEqual(started)
		// Back in the library: live again.
		const live = tr('a', 'b', 'c', 'z')
		expect(resolveQueue(context, library, live)).toEqual(live)
	})

	it('keeps no context when there is none', () => {
		expect(refreshQueueSnapshot(null, library, tr('a'))).toBeNull()
	})
})

describe('navigateQueue — restartCurrent may be async', () => {
	it('awaits it', async () => {
		const s = createShuffleSession<T>(keyOf, first)
		s.sync('a')
		const restart = vi.fn().mockResolvedValue(undefined)
		const out = await navigateQueue('previous', {
			items: tr('a', 'b'),
			currentKey: 'a',
			keyOf,
			shuffle: s,
			start: async () => true,
			restartCurrent: restart,
		})
		expect(restart).toHaveBeenCalledOnce()
		expect(out.restarted).toBe(true)
	})
})

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

interface Rel extends PreviewRelease {
	playable: boolean[]
}

const rel = (id: string, playable: boolean[]): Rel => ({ id, tracks: playable, playable })
const canPlay = (r: Rel, i: number) => r.playable[i] === true

describe('discoveryTrackKey / previewTargetKey', () => {
	it('builds a stable key from release id and track index', () => {
		expect(discoveryTrackKey('r1', 2)).toBe('r1:2')
		expect(previewTargetKey({ release: rel('r1', [true]), trackIndex: 2 })).toBe('r1:2')
	})
})

describe('findPlayableTrackIndex', () => {
	const r = rel('r', [false, true, true, false])

	it('finds the first playable track', () => {
		expect(findPlayableTrackIndex(r, 'first', canPlay)).toBe(1)
	})

	it('finds the last playable track', () => {
		expect(findPlayableTrackIndex(r, 'last', canPlay)).toBe(2)
	})

	it('returns -1 when nothing is playable', () => {
		const none = rel('n', [false, false])
		expect(findPlayableTrackIndex(none, 'first', canPlay)).toBe(-1)
		expect(findPlayableTrackIndex(none, 'last', canPlay)).toBe(-1)
		expect(findPlayableTrackIndex(rel('e', []), 'first', canPlay)).toBe(-1)
	})
})

describe('buildDiscoveryCandidates', () => {
	it('lists playable tracks in release order, then track order', () => {
		const releases = [rel('r1', [true, false, true]), rel('r2', [false]), rel('r3', [true])]
		const keys = buildDiscoveryCandidates(releases, canPlay).map(previewTargetKey)
		expect(keys).toEqual(['r1:0', 'r1:2', 'r3:0'])
	})

	it('is empty for an empty queue', () => {
		expect(buildDiscoveryCandidates([], canPlay)).toEqual([])
	})
})

describe('discovery shuffle through createShuffleSession', () => {
	it('shuffles at track level across releases without repeating', () => {
		const releases = [rel('r1', [true, true]), rel('r2', [true])]
		const s = createShuffleSession<PreviewTarget<Rel>>(previewTargetKey, first)
		s.reset(discoveryTrackKey('r1', 0))
		const a = s.next(buildDiscoveryCandidates(releases, canPlay), 'r1:0')!
		expect(previewTargetKey(a)).toBe('r1:1')
		const b = s.next(buildDiscoveryCandidates(releases, canPlay), 'r1:1')!
		expect(previewTargetKey(b)).toBe('r2:0')
		const prev = s.previous(buildDiscoveryCandidates(releases, canPlay))!
		expect(previewTargetKey(prev)).toBe('r1:1')
	})

	it('skips a history entry whose track became unplayable', () => {
		const s = createShuffleSession<PreviewTarget<Rel>>(previewTargetKey, first)
		const before = [rel('r1', [true, true])]
		s.reset('r1:0')
		s.next(buildDiscoveryCandidates(before, canPlay), 'r1:0') // r1:1
		const after = [rel('r1', [true, false])]
		expect(s.previous(buildDiscoveryCandidates(after, canPlay))?.trackIndex).toBe(0)
		expect(s.previous(buildDiscoveryCandidates(after, canPlay))).toBeNull()
	})
})

describe('nextPreviewTarget', () => {
	const r1 = rel('r1', [true, false, true])
	const r2 = rel('r2', [false, true])
	const r3 = rel('r3', [false, false])
	const queue = [r1, r2, r3]

	it('moves to the next playable track of the same release, skipping unplayable ones', () => {
		const t = nextPreviewTarget({ release: r1, trackIndex: 0 }, queue, canPlay)
		expect(t).toEqual({ release: r1, trackIndex: 2 })
	})

	it('moves to the first playable track of the next release at the end of a release', () => {
		const t = nextPreviewTarget({ release: r1, trackIndex: 2 }, queue, canPlay)
		expect(t).toEqual({ release: r2, trackIndex: 1 })
	})

	it('skips releases with nothing playable and wraps around the queue', () => {
		const t = nextPreviewTarget({ release: r2, trackIndex: 1 }, queue, canPlay)
		// r3 has nothing playable, wrap to r1
		expect(t).toEqual({ release: r1, trackIndex: 0 })
	})

	it('loops back on the current release when it is the only playable one', () => {
		const t = nextPreviewTarget({ release: r1, trackIndex: 2 }, [r1, r3], canPlay)
		expect(t).toEqual({ release: r1, trackIndex: 0 })
	})

	it('returns null when the current release is not in the queue', () => {
		expect(nextPreviewTarget({ release: r1, trackIndex: 2 }, [r2, r3], canPlay)).toBeNull()
	})

	it('returns null for an empty queue', () => {
		expect(nextPreviewTarget({ release: r1, trackIndex: 2 }, [], canPlay)).toBeNull()
	})

	it('returns null when nothing in the queue is playable', () => {
		const dead = rel('d', [false])
		expect(nextPreviewTarget({ release: dead, trackIndex: 0 }, [dead, r3], canPlay)).toBeNull()
	})
})

describe('previousPreviewTarget', () => {
	const r1 = rel('r1', [true, false, true])
	const r2 = rel('r2', [false, true, false])
	const r3 = rel('r3', [false, false])
	const queue = [r1, r2, r3]

	it('moves to the previous playable track of the same release, skipping unplayable ones', () => {
		const t = previousPreviewTarget({ release: r1, trackIndex: 2 }, queue, canPlay)
		expect(t).toEqual({ release: r1, trackIndex: 0 })
	})

	it('moves to the last playable track of the previous release at the start of a release', () => {
		const t = previousPreviewTarget({ release: r2, trackIndex: 1 }, queue, canPlay)
		expect(t).toEqual({ release: r1, trackIndex: 2 })
	})

	it('wraps from the first release to the last playable one', () => {
		const t = previousPreviewTarget({ release: r1, trackIndex: 0 }, queue, canPlay)
		// r3 has nothing playable, so the search continues to r2
		expect(t).toEqual({ release: r2, trackIndex: 1 })
	})

	it('returns null when the current release is not in the queue or the queue is empty', () => {
		expect(previousPreviewTarget({ release: r1, trackIndex: 0 }, [r2], canPlay)).toBeNull()
		expect(previousPreviewTarget({ release: r1, trackIndex: 0 }, [], canPlay)).toBeNull()
	})
})
