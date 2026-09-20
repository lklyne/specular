import type { IncomingMessage, ServerResponse } from 'http'
import { describe, it, expect } from 'vitest'
import {
  setAgentBrowserRefs,
  invalidateAgentBrowserRefsForPage,
  synthesizeAgentBrowserTargetQuery,
} from '../../src/main/presence-manager'
import { sessionRoutes } from '../../src/main/routes/session'

// Ref -> {role, name} for the same fixture snapshot as
// agent-browser-snapshot-refs.test.ts's INTERACTIVE_SNAPSHOT.
const LOGIN_PAGE_REFS = [
  { ref: 'e1', role: 'heading', name: 'Log in' },
  { ref: 'e9', role: 'textbox', name: 'Email' },
  { ref: 'e10', role: 'textbox', name: 'Password' },
  { ref: 'e5', role: 'button', name: 'Sign in' },
  { ref: 'e6', role: 'link', name: 'Docs' },
  { ref: 'e7', role: 'link', name: 'Docs' },
  { ref: 'e8', role: 'link', name: 'Pricing' },
  { ref: 'e2', role: 'button', name: 'Close dialog' },
  { ref: 'e3', role: 'combobox', name: null },
  { ref: 'e11', role: 'option', name: 'Free' },
  { ref: 'e12', role: 'option', name: 'Pro' },
  { ref: 'e4', role: 'checkbox', name: 'Accept "terms" [v2]' },
]

describe('synthesizeAgentBrowserTargetQuery', () => {
  it('synthesizes a name-only query for a ref with a unique, non-empty name', () => {
    const sessionId = 'session-unique'
    const pageId = 'page-login'
    setAgentBrowserRefs(sessionId, pageId, LOGIN_PAGE_REFS)

    expect(synthesizeAgentBrowserTargetQuery(sessionId, pageId, 'e5')).toEqual({
      selector: null,
      text: null,
      name: 'Sign in',
    })
    // Escaped quotes and literal brackets in the name survive round-trip.
    expect(synthesizeAgentBrowserTargetQuery(sessionId, pageId, 'e4')).toEqual({
      selector: null,
      text: null,
      name: 'Accept "terms" [v2]',
    })
  })

  it('refuses two refs sharing the same name and role', () => {
    const sessionId = 'session-dup-role'
    const pageId = 'page-login'
    setAgentBrowserRefs(sessionId, pageId, LOGIN_PAGE_REFS)

    expect(synthesizeAgentBrowserTargetQuery(sessionId, pageId, 'e6')).toBeNull()
    expect(synthesizeAgentBrowserTargetQuery(sessionId, pageId, 'e7')).toBeNull()
  })

  it('refuses two refs sharing a name across different roles — findPresenceTarget has no role signal', () => {
    const sessionId = 'session-dup-cross-role'
    const pageId = 'page-mixed'
    setAgentBrowserRefs(sessionId, pageId, [
      { ref: 'e1', role: 'heading', name: 'Docs' },
      { ref: 'e2', role: 'link', name: 'Docs' },
    ])

    expect(synthesizeAgentBrowserTargetQuery(sessionId, pageId, 'e1')).toBeNull()
    expect(synthesizeAgentBrowserTargetQuery(sessionId, pageId, 'e2')).toBeNull()
  })

  it('refuses a ref with no accessible name', () => {
    const sessionId = 'session-no-name'
    const pageId = 'page-login'
    setAgentBrowserRefs(sessionId, pageId, LOGIN_PAGE_REFS)

    expect(synthesizeAgentBrowserTargetQuery(sessionId, pageId, 'e3')).toBeNull()
  })

  it('returns null for an unknown ref, page, or session', () => {
    const sessionId = 'session-unknown'
    const pageId = 'page-login'
    setAgentBrowserRefs(sessionId, pageId, LOGIN_PAGE_REFS)

    expect(synthesizeAgentBrowserTargetQuery(sessionId, pageId, 'e999')).toBeNull()
    expect(synthesizeAgentBrowserTargetQuery(sessionId, 'other-page', 'e5')).toBeNull()
    expect(synthesizeAgentBrowserTargetQuery('other-session', pageId, 'e5')).toBeNull()
  })

  it('replaces the map wholesale on a fresh snapshot rather than merging', () => {
    const sessionId = 'session-replace'
    const pageId = 'page-login'
    setAgentBrowserRefs(sessionId, pageId, [{ ref: 'e1', role: 'button', name: 'Old' }])
    expect(synthesizeAgentBrowserTargetQuery(sessionId, pageId, 'e1')).toEqual({
      selector: null,
      text: null,
      name: 'Old',
    })

    setAgentBrowserRefs(sessionId, pageId, [{ ref: 'e2', role: 'button', name: 'New' }])
    // The stale e1 entry is gone, not merged alongside e2.
    expect(synthesizeAgentBrowserTargetQuery(sessionId, pageId, 'e1')).toBeNull()
    expect(synthesizeAgentBrowserTargetQuery(sessionId, pageId, 'e2')).toEqual({
      selector: null,
      text: null,
      name: 'New',
    })
  })

  it('invalidateAgentBrowserRefsForPage drops every session tracking that page', () => {
    const pageId = 'page-to-invalidate'
    setAgentBrowserRefs('session-x', pageId, LOGIN_PAGE_REFS)
    setAgentBrowserRefs('session-y', pageId, LOGIN_PAGE_REFS)
    setAgentBrowserRefs('session-x', 'other-page', LOGIN_PAGE_REFS)

    invalidateAgentBrowserRefsForPage(pageId)

    expect(synthesizeAgentBrowserTargetQuery('session-x', pageId, 'e5')).toBeNull()
    expect(synthesizeAgentBrowserTargetQuery('session-y', pageId, 'e5')).toBeNull()
    // A different page for the same session is untouched.
    expect(synthesizeAgentBrowserTargetQuery('session-x', 'other-page', 'e5')).toEqual({
      selector: null,
      text: null,
      name: 'Sign in',
    })
  })
})

describe('POST /session/presence/agent-browser-refs', () => {
  const route = sessionRoutes.find(
    (r) => r.method === 'POST' && r.pattern === '/session/presence/agent-browser-refs',
  )
  if (!route) throw new Error('POST /session/presence/agent-browser-refs route not found')

  function fakeRequest(sessionId: string): IncomingMessage {
    return { headers: { 'x-specular-session-id': sessionId } } as unknown as IncomingMessage
  }
  function fakeResponse(): ServerResponse {
    return { statusCode: 0, setHeader: () => {}, end: () => {} } as unknown as ServerResponse
  }
  function post(sessionId: string, body: Record<string, unknown>): Promise<void> {
    return route!.handler({
      request: fakeRequest(sessionId),
      response: fakeResponse(),
      url: '/session/presence/agent-browser-refs',
      body,
      params: {},
    })
  }

  it('stores a valid ref map, reachable via synthesizeAgentBrowserTargetQuery', async () => {
    await post('session-route-a', { pageId: 'page-route', refs: LOGIN_PAGE_REFS })
    expect(synthesizeAgentBrowserTargetQuery('session-route-a', 'page-route', 'e5')).toEqual({
      selector: null,
      text: null,
      name: 'Sign in',
    })
  })

  it('drops malformed entries instead of throwing', async () => {
    await post('session-route-b', {
      pageId: 'page-route',
      refs: [
        null,
        'not an object',
        { ref: 'e1' }, // missing role
        { role: 'button' }, // missing ref
        { ref: 'e2', role: 'button', name: 'Go' },
      ],
    })
    expect(synthesizeAgentBrowserTargetQuery('session-route-b', 'page-route', 'e2')).toEqual({
      selector: null,
      text: null,
      name: 'Go',
    })
    expect(synthesizeAgentBrowserTargetQuery('session-route-b', 'page-route', 'e1')).toBeNull()
  })

  it('never throws on a missing pageId or refs — always answers ok', async () => {
    await expect(post('session-route-c', {})).resolves.toBeUndefined()
    await expect(post('session-route-c', { pageId: 'page-route', refs: 'not-an-array' })).resolves.toBeUndefined()
  })
})
