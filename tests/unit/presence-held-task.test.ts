import type { IncomingMessage } from 'http'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  activePresenceTasks,
  clearActivePresenceTask,
  getPresenceCursors,
  presenceCursors,
  PRESENCE_HELD_BACKSTOP_MS,
  upsertActivePresenceTask,
  upsertPresenceCursor,
} from '../../src/main/presence-cursor'
import { mcpSessions, MCP_SESSION_TIMEOUT_MS } from '../../src/main/presence-session'

// `specular presence start "<task>"` (a held task) brackets a high-level
// task so the agent's cursor idles in place on the canvas for its whole
// duration instead of departing whenever the driving LLM thinks for more
// than the ordinary 10s idle-retire between calls, or the 15s MCP session
// timeout lapses between short-lived CLI processes. A 5-minute backstop
// still retires it if the agent crashes without ever calling
// `specular presence done`.

function fakeRequest(sessionId: string, clientName = 'agent'): IncomingMessage {
  return {
    headers: {
      'x-specular-session-id': sessionId,
      'x-specular-client-name': clientName,
    },
  } as unknown as IncomingMessage
}

// Comfortably past `beginPresenceDeparture`'s removal grace period without
// depending on its exact (unexported) value.
const PAST_DEPARTURE_GRACE_MS = 3_000

beforeEach(() => {
  vi.useFakeTimers()
  presenceCursors.clear()
  activePresenceTasks.clear()
  mcpSessions.clear()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('held presence task — expiry exemption', () => {
  it('survives past the 10s idle retire and the MCP session timeout while the hold is fresh', () => {
    const request = fakeRequest('session-held')
    upsertPresenceCursor(request, { canvasX: 0, canvasY: 0, activity: 'thinking' })
    upsertActivePresenceTask(request, { taskLabel: 'refactor sidebar', hold: true })

    vi.advanceTimersByTime(MCP_SESSION_TIMEOUT_MS + 1)

    const cursors = getPresenceCursors()
    expect(cursors.some((c) => c.sessionId === 'session-held')).toBe(true)
    expect(cursors.find((c) => c.sessionId === 'session-held')?.activity).not.toBe('departing')
  })

  it('retires once the 5-minute backstop elapses with no further requests', () => {
    const request = fakeRequest('session-held')
    upsertPresenceCursor(request, { canvasX: 0, canvasY: 0, activity: 'thinking' })
    upsertActivePresenceTask(request, { taskLabel: 'refactor sidebar', hold: true })

    vi.advanceTimersByTime(PRESENCE_HELD_BACKSTOP_MS + 1)
    // getPresenceCursors() runs the expiry sweep at the current virtual
    // time — the periodic 2s-interval sweep alone might not yet have landed
    // exactly on this instant. Backstop trips the same departure grace
    // period an ordinary cursor gets.
    expect(getPresenceCursors().find((c) => c.sessionId === 'session-held')?.activity).toBe(
      'departing',
    )

    vi.advanceTimersByTime(PAST_DEPARTURE_GRACE_MS)
    expect(getPresenceCursors().some((c) => c.sessionId === 'session-held')).toBe(false)
  })

  it('a fresh request just before the backstop keeps the hold alive', () => {
    const request = fakeRequest('session-held')
    upsertPresenceCursor(request, { canvasX: 0, canvasY: 0, activity: 'thinking' })
    upsertActivePresenceTask(request, { taskLabel: 'refactor sidebar', hold: true })

    vi.advanceTimersByTime(PRESENCE_HELD_BACKSTOP_MS - 1_000)
    // A real request (e.g. another act/think event) refreshes the hold.
    upsertActivePresenceTask(request, {})
    vi.advanceTimersByTime(2_000)

    const cursors = getPresenceCursors()
    expect(cursors.some((c) => c.sessionId === 'session-held')).toBe(true)
    expect(cursors.find((c) => c.sessionId === 'session-held')?.activity).not.toBe('departing')
  })

  it('`done` departs a held cursor immediately, ignoring the backstop', () => {
    const request = fakeRequest('session-held')
    upsertPresenceCursor(request, { canvasX: 0, canvasY: 0, activity: 'thinking' })
    upsertActivePresenceTask(request, { taskLabel: 'refactor sidebar', hold: true })

    clearActivePresenceTask(request)
    expect(activePresenceTasks.has('session-held')).toBe(false)

    vi.advanceTimersByTime(PAST_DEPARTURE_GRACE_MS)
    expect(getPresenceCursors().some((c) => c.sessionId === 'session-held')).toBe(false)
  })

  it('a non-held cursor still retires at the ordinary 10s idle timeout', () => {
    const request = fakeRequest('session-plain')
    upsertPresenceCursor(request, { canvasX: 0, canvasY: 0, activity: 'acting' })

    vi.advanceTimersByTime(10_001)
    expect(getPresenceCursors().find((c) => c.sessionId === 'session-plain')?.activity).toBe(
      'departing',
    )

    vi.advanceTimersByTime(PAST_DEPARTURE_GRACE_MS)
    expect(getPresenceCursors().some((c) => c.sessionId === 'session-plain')).toBe(false)
  })
})
