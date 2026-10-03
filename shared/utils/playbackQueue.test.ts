import { describe, it, expect } from 'vitest'
import {
	buildDiscoveryCandidates,
	createShuffleSession,
	discoveryTrackKey,
	findPlayableTrackIndex,
	nextPreviewTarget,
	previewTargetKey,
	previousPreviewTarget,
	sequentialNextIndex,
	sequentialPreviousIndex,
	type PreviewRelease,
	type PreviewTarget,
	type RandomSource,
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
