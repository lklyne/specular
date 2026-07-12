import type { IncomingMessage } from 'http'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  bumpActiveScanId,
  presenceCursors,
  scheduleThinkingState,
  staggerOperation,
  upsertPresenceCursor,
} from '../../src/main/presence-cursor'
import { mcpSessions, MCP_SESSION_TIMEOUT_MS } from '../../src/main/presence-session'
import { PRESENCE_STEP_DELAY_MS, PRESENCE_THINKING_DELAY_MS } from '../../src/shared/presence-timing'

// Issue #319 Phase 4: two concurrent agent sessions must not be able to
// interrupt each other's presence choreography. Before this phase,
// `thinkingTimer` and `activeScanId` were module singletons, so any
// session's activity canceled every other session's pending "thinking"
// transition or in-flight scan animation.

function fakeRequest(sessionId: string, clientName = 'agent'): IncomingMessage {
  return {
    headers: {
      'x-specular-session-id': sessionId,
      'x-specular-client-name': clientName,
    },
  } as unknown as IncomingMessage
}

beforeEach(() => {
  vi.useFakeTimers()
  presenceCursors.clear()
  mcpSessions.clear()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('scheduleThinkingState — per-session timers', () => {
  it("session B scheduling its own thinking transition does not cancel session A's pending one", () => {
    const requestA = fakeRequest('session-a', 'agent-a')
    const requestB = fakeRequest('session-b', 'agent-b')
    upsertPresenceCursor(requestA, { canvasX: 0, canvasY: 0 })
    upsertPresenceCursor(requestB, { canvasX: 0, canvasY: 0 })

    scheduleThinkingState(requestA)
    vi.advanceTimersByTime(PRESENCE_THINKING_DELAY_MS / 2)
    // B acts in the gap. Under the old singleton timer this call would
    // clearTimeout() A's pending transition via the shared module variable.
    scheduleThinkingState(requestB)

    vi.advanceTimersByTime(PRESENCE_THINKING_DELAY_MS / 2)
    expect(presenceCursors.get('session-a')?.activity).toBe('thinking')
    // B's own timer hasn't reached its full delay yet.
    expect(presenceCursors.get('session-b')?.activity).not.toBe('thinking')

    vi.advanceTimersByTime(PRESENCE_THINKING_DELAY_MS / 2)
    expect(presenceCursors.get('session-b')?.activity).toBe('thinking')
  })
})

describe('staggerOperation — per-session scan cancellation', () => {
  it("bumping session B's scan id does not cancel session A's in-flight scan", async () => {
    const requestA = fakeRequest('session-a', 'agent-a')
    const requestB = fakeRequest('session-b', 'agent-b')
    upsertPresenceCursor(requestA, { canvasX: 0, canvasY: 0 })
    upsertPresenceCursor(requestB, { canvasX: 0, canvasY: 0 })

    const itemsA = [{ x: 10, y: 10 }, { x: 20, y: 20 }]
    const itemsB = [{ x: 30, y: 30 }, { x: 40, y: 40 }]
    const performedA: number[] = []
    const performedB: number[] = []

    staggerOperation(requestA, itemsA, null, (i) => performedA.push(i))
    staggerOperation(requestB, itemsB, null, (i) => performedB.push(i))

    // Simulate session B kicking off new activity mid-scan (e.g. another
    // command arriving) — this should cancel only B's scan.
    bumpActiveScanId('session-b')

    await vi.advanceTimersByTimeAsync(PRESENCE_STEP_DELAY_MS * itemsA.length + 10)

    expect(performedA).toEqual([0, 1])
    expect(performedB).toEqual([])
  })
})

describe('upsertPresenceCursor — clientName eviction scoping', () => {
  it('evicts a same-clientName cursor only once its session has gone stale', () => {
    const requestA = fakeRequest('session-a', 'shared-client')
    upsertPresenceCursor(requestA, { canvasX: 0, canvasY: 0 })
    expect(presenceCursors.has('session-a')).toBe(true)

    vi.advanceTimersByTime(MCP_SESSION_TIMEOUT_MS + 1)

    const requestB = fakeRequest('session-b', 'shared-client')
    upsertPresenceCursor(requestB, { canvasX: 0, canvasY: 0 })

    expect(presenceCursors.has('session-a')).toBe(false)
    expect(presenceCursors.has('session-b')).toBe(true)
  })

  it('does not evict a live same-clientName cursor belonging to a concurrent session', () => {
    const requestA = fakeRequest('session-a', 'shared-client')
    upsertPresenceCursor(requestA, { canvasX: 0, canvasY: 0 })
    expect(presenceCursors.has('session-a')).toBe(true)

    // Well inside the MCP session timeout — session A is still live.
    vi.advanceTimersByTime(MCP_SESSION_TIMEOUT_MS / 3)

    const requestB = fakeRequest('session-b', 'shared-client')
    upsertPresenceCursor(requestB, { canvasX: 0, canvasY: 0 })

    expect(presenceCursors.has('session-a')).toBe(true)
    expect(presenceCursors.has('session-b')).toBe(true)
  })
})
