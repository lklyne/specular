import type { IncomingMessage, ServerResponse } from 'http'
import { beforeEach, describe, expect, it } from 'vitest'
import { sessionRoutes } from '../../src/main/routes/session'
import { activePresenceTasks, presenceCursors } from '../../src/main/presence-cursor'
import { mcpSessions } from '../../src/main/presence-session'

// `taskLabel` is free text an agent controls end to end (`specular presence
// start "<label>"` / the MCP `start_task` tool), and it's now rendered on
// the canvas next to the cursor (AgentCursorLabel.tsx) — so, like
// `labelHint` already is, it must be bounded at the route boundary rather
// than trusted verbatim.

const presenceRoute = sessionRoutes.find(
  (route) => route.method === 'POST' && route.pattern === '/session/presence',
)
if (!presenceRoute) throw new Error('POST /session/presence route not found')

function fakeRequest(sessionId: string): IncomingMessage {
  return { headers: { 'x-specular-session-id': sessionId } } as unknown as IncomingMessage
}

function fakeResponse(): ServerResponse {
  return { statusCode: 0, setHeader: () => {}, end: () => {} } as unknown as ServerResponse
}

function postPresenceStart(sessionId: string, taskLabel: string): Promise<void> {
  return presenceRoute!.handler({
    request: fakeRequest(sessionId),
    response: fakeResponse(),
    url: '/session/presence',
    body: { eventType: 'start', hold: true, surface: 'canvas', phase: 'thinking', taskLabel },
    params: {},
  })
}

beforeEach(() => {
  presenceCursors.clear()
  activePresenceTasks.clear()
  mcpSessions.clear()
})

describe('taskLabel bounding at the /session/presence route boundary', () => {
  it('trims, collapses internal whitespace/newlines, and caps at 80 chars', async () => {
    await postPresenceStart('session-a', '  fix   the\n\nheader   layout  ')
    expect(activePresenceTasks.get('session-a')?.taskLabel).toBe('fix the header layout')
  })

  it('caps a long label at 80 characters', async () => {
    const long = 'x'.repeat(200)
    await postPresenceStart('session-b', long)
    const stored = activePresenceTasks.get('session-b')?.taskLabel
    expect(stored).toHaveLength(80)
    expect(stored).toBe('x'.repeat(80))
  })

  it('treats a whitespace-only label as absent', async () => {
    await postPresenceStart('session-c', '   \n\t  ')
    expect(activePresenceTasks.get('session-c')?.taskLabel).toBeNull()
  })
})
