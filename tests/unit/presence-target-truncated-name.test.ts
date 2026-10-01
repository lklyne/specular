import { describe, it, expect, beforeEach } from 'vitest'
import { findPresenceTarget } from '../../src/main/presence-manager'
import { cacheAgentSnapshot, invalidateAgentSnapshot } from '../../src/main/runtime/agent-snapshot-cache'

// Real data from a live run: `specular get text @e816` never moved the
// presence cursor. The ref's accessible name was a long sentence on a link
// — main's own agent-snapshot cache stores node.name as `bestElementName`'s
// `tag "text"` label (src/preload/dom-element-utils.ts), which caps `text`
// at 80 chars via `compactText`. `synthesizeAgentBrowserTargetQuery` builds
// its query from the *untruncated* name agent-browser saw, so the
// query is longer than the truncated candidate — the ordinary substring
// tier in scoreDescriptorMatch checks candidate-includes-query, which can
// never succeed once the candidate is the shorter string. This is the
// pure-logic reproduction, seeding the cache directly rather than a live
// page.
const PAGE_ID = 'page-flight-results'

const FULL_NAME =
  'From 901 US dollars round trip total. Nonstop flight with ZIPAIR Tokyo. ' +
  'Leaves San Francisco International Airport at 3:45 PM on Tuesday, January 12 ' +
  'and arrives at Narita International Airport at 7:55 PM on Wednesday, January 13. ' +
  'Total duration 11 hr 10 min. Select flight'

// What buildStructuredSnapshotNode actually stores: bestElementName wraps the
// text in `tag "..."` and compactText caps it at 80 chars with a trailing "…".
const TRUNCATED_LABEL = `a "${FULL_NAME.slice(0, 79)}…"`

beforeEach(() => {
  invalidateAgentSnapshot(PAGE_ID)
})

describe('findPresenceTarget — long accessible names', () => {
  it('resolves a name query against a node whose cached name was truncated to 80 chars', async () => {
    cacheAgentSnapshot({
      pageId: PAGE_ID,
      url: 'https://example.com/flights',
      title: 'Flights',
      nodes: [
        {
          ref: '@e816',
          depth: 3,
          tagName: 'a',
          role: 'link',
          name: TRUNCATED_LABEL,
          interactive: true,
          bounds: { x: 100, y: 200, width: 600, height: 80 },
          elementPath: 'a.result-row',
          fullPath: 'div > ul > li > a.result-row',
        },
        {
          ref: '@e817',
          depth: 3,
          tagName: 'a',
          role: 'link',
          name: 'a "Unrelated flight offer"',
          interactive: true,
          bounds: { x: 100, y: 300, width: 600, height: 80 },
          elementPath: 'a.result-row',
          fullPath: 'div > ul > li > a.result-row',
        },
      ],
    })

    const result = await findPresenceTarget(PAGE_ID, { name: FULL_NAME, interactiveOnly: true })

    expect(result).not.toBeNull()
    expect(result?.targetRef).toBe('@e816')
    expect(result?.targetRect).toEqual({ x: 100, y: 200, width: 600, height: 80 })
  })

  it('refuses when the truncated prefix matches more than one candidate', async () => {
    cacheAgentSnapshot({
      pageId: PAGE_ID,
      url: 'https://example.com/flights',
      title: 'Flights',
      nodes: [
        {
          ref: '@e1',
          depth: 3,
          tagName: 'a',
          role: 'link',
          name: TRUNCATED_LABEL,
          interactive: true,
          bounds: { x: 100, y: 200, width: 600, height: 80 },
          elementPath: 'a.result-row',
          fullPath: 'div > ul > li > a.result-row',
        },
        {
          // A second row that happens to share the same 80-char prefix (a
          // long shared preamble, different ending) — genuinely ambiguous
          // once both are truncated the same way.
          ref: '@e2',
          depth: 3,
          tagName: 'a',
          role: 'link',
          name: TRUNCATED_LABEL,
          interactive: true,
          bounds: { x: 100, y: 300, width: 600, height: 80 },
          elementPath: 'a.result-row',
          fullPath: 'div > ul > li > a.result-row',
        },
      ],
    })

    const result = await findPresenceTarget(PAGE_ID, { name: FULL_NAME, interactiveOnly: true })
    expect(result).toBeNull()
  })

  it('does not travel to a non-interactive element sharing the truncated prefix', async () => {
    cacheAgentSnapshot({
      pageId: PAGE_ID,
      url: 'https://example.com/flights',
      title: 'Flights',
      nodes: [
        {
          ref: '@e1',
          depth: 2,
          tagName: 'div',
          name: TRUNCATED_LABEL,
          interactive: false,
          bounds: { x: 100, y: 200, width: 600, height: 80 },
          elementPath: 'div.result-row',
          fullPath: 'div > ul > li > div.result-row',
        },
      ],
    })

    const result = await findPresenceTarget(PAGE_ID, { name: FULL_NAME, interactiveOnly: true })
    expect(result).toBeNull()
  })

  it('still resolves an exact, untruncated name through the normal scoring path', async () => {
    cacheAgentSnapshot({
      pageId: PAGE_ID,
      url: 'https://example.com/flights',
      title: 'Flights',
      nodes: [
        {
          ref: '@e1',
          depth: 3,
          tagName: 'button',
          role: 'button',
          name: 'button "Sign in"',
          interactive: true,
          bounds: { x: 10, y: 10, width: 80, height: 30 },
          elementPath: 'button',
          fullPath: 'form > button',
        },
      ],
    })

    const result = await findPresenceTarget(PAGE_ID, { name: 'Sign in', interactiveOnly: true })
    expect(result?.targetRef).toBe('@e1')
    expect(result?.targetName).toBe('Sign in')
  })
})
