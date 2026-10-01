import type { IncomingMessage, ServerResponse } from 'http'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { advancePendingIntent, sessionRoutes } from '../../src/main/routes/session'
import { pendingIntents, PENDING_INTENT_TTL_MS } from '../../src/main/presence-manager'
import { presenceCursors } from '../../src/main/presence-cursor'
import { mcpSessions } from '../../src/main/presence-session'

// A chained browse command (`click @e1 && fill @e2 "x"`) runs as a single
// `batch` spawn, so main can't fire a fresh presence intent between steps —
// the whole ordered list is registered up front (`/session/presence/intent`'s
// `queue` field) and advanced one step at a time as each step's own CDP
// event (mousePressed — see app-control-server.ts) or TTL consumes the one
// before it. These tests drive `advancePendingIntent` directly since that's
// exactly what those two triggers call.

const intentRoute = sessionRoutes.find(
  (route) => route.method === 'POST' && route.pattern === '/session/presence/intent',
)
if (!intentRoute) throw new Error('POST /session/presence/intent route not found')

function fakeRequest(sessionId: string, clientName = 'agent'): IncomingMessage {
  return {
    headers: {
      'x-specular-session-id': sessionId,
      'x-specular-client-name': clientName,
    },
  } as unknown as IncomingMessage
}

function fakeResponse(): ServerResponse {
  return { statusCode: 0, setHeader: () => {}, end: () => {} } as unknown as ServerResponse
}

function postIntent(sessionId: string, body: Record<string, unknown>): Promise<void> {
  return intentRoute!.handler({
    request: fakeRequest(sessionId),
    response: fakeResponse(),
    url: '/session/presence/intent',
    body,
    params: {},
  })
}

beforeEach(() => {
  vi.useFakeTimers()
  presenceCursors.clear()
  pendingIntents.clear()
  mcpSessions.clear()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('chained browse presence — queue advance', () => {
  it('advances to the next queued step when the current one is consumed', async () => {
    await postIntent('session-a', {
      command: 'click',
      labelKey: 'click_target',
      queue: [
        {
          labelKey: 'type_text',
          command: 'fill',
          targetRef: '@e2',
          targetRefSource: 'agent-browser',
          targetQuery: null,
          labelHint: 'editing control',
        },
      ],
    })
    expect(pendingIntents.get('session-a')?.command).toBe('click')
    expect(pendingIntents.get('session-a')?.queue).toHaveLength(1)

    advancePendingIntent(fakeRequest('session-a'), 'session-a', {})

    const current = pendingIntents.get('session-a')
    expect(current?.command).toBe('fill')
    expect(current?.labelKey).toBe('type_text')
    expect(current?.queue).toHaveLength(0)
  })

  it('deletes the pending intent outright once the queue is empty', async () => {
    await postIntent('session-b', { command: 'click', labelKey: 'click_target' })
    expect(pendingIntents.has('session-b')).toBe(true)

    advancePendingIntent(fakeRequest('session-b'), 'session-b', {})
    expect(pendingIntents.has('session-b')).toBe(false)
  })

  it('advances via its own TTL when no CDP event consumes the step (e.g. wait/get)', async () => {
    await postIntent('session-c', {
      command: 'wait',
      labelKey: 'wait_page',
      queue: [
        {
          labelKey: 'click_target',
          command: 'click',
          targetRef: '@e2',
          targetRefSource: 'agent-browser',
          targetQuery: null,
          labelHint: null,
        },
      ],
    })
    expect(pendingIntents.get('session-c')?.command).toBe('wait')

    vi.advanceTimersByTime(PENDING_INTENT_TTL_MS + 1)

    expect(pendingIntents.get('session-c')?.command).toBe('click')
  })

  it('a fresh intent from the same session replaces the whole queue, not just the current step', async () => {
    await postIntent('session-d', {
      command: 'click',
      labelKey: 'click_target',
      queue: [
        {
          labelKey: 'type_text',
          command: 'fill',
          targetRef: '@e2',
          targetRefSource: 'agent-browser',
          targetQuery: null,
          labelHint: null,
        },
      ],
    })
    expect(pendingIntents.get('session-d')?.queue).toHaveLength(1)

    // A new, unrelated single-command intent — the queue it carries (none)
    // must win outright, not merge with the stale one.
    await postIntent('session-d', { command: 'scroll', labelKey: 'scroll_page' })

    const current = pendingIntents.get('session-d')
    expect(current?.command).toBe('scroll')
    expect(current?.queue).toHaveLength(0)
  })

  it('drops an out-of-allowlist labelKey from the incoming queue rather than storing a dead step', async () => {
    await postIntent('session-e', {
      command: 'click',
      labelKey: 'click_target',
      queue: [
        { labelKey: 'not-a-real-label', command: 'screenshot' },
        { labelKey: 'wait_page', command: 'wait' },
      ],
    })

    const queue = pendingIntents.get('session-e')?.queue
    expect(queue).toHaveLength(1)
    expect(queue?.[0].command).toBe('wait')
  })
})
