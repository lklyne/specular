import { describe, it, expect, vi, beforeEach } from 'vitest'

// findPresenceTarget's page-side fallback (issue #319 — deeply nested
// interactive leaves never reach main's depth-capped agent snapshot) goes
// through queryElementsByName, a real page-IPC round trip. Mock the wrapper
// at the module boundary (page-runtime.ts) so this stays a plain-Node unit
// test — mirrors how presence-target-truncated-name.test.ts and
// presence-targeting.test.ts avoid touching Electron.
const { queryElementsByNameMock, queryPageElementsMock } = vi.hoisted(() => ({
  queryElementsByNameMock: vi.fn(),
  queryPageElementsMock: vi.fn(),
}))

vi.mock('../../src/main/runtime/page-runtime', () => ({
  takePageAgentSnapshot: vi.fn(),
  queryPageElements: queryPageElementsMock,
  queryElementsByName: queryElementsByNameMock,
}))

import { findPresenceTarget } from '../../src/main/presence-manager'
import { cacheAgentSnapshot, invalidateAgentSnapshot } from '../../src/main/runtime/agent-snapshot-cache'

const PAGE_ID = 'page-deep-app'

beforeEach(() => {
  invalidateAgentSnapshot(PAGE_ID)
  queryElementsByNameMock.mockReset()
  queryPageElementsMock.mockReset()
  // An empty structural snapshot — the depth-capped tree found nothing, the
  // condition the fallback exists for.
  cacheAgentSnapshot({ pageId: PAGE_ID, url: 'https://example.com/results', title: 'Results', nodes: [] })
})

describe('findPresenceTarget — page-side accessible-name fallback', () => {
  it('falls back to the live lookup when snapshot scoring found nothing, and accepts a unique match', async () => {
    queryElementsByNameMock.mockResolvedValue([{ rect: { x: 100, y: 200, width: 50, height: 20 } }])

    const result = await findPresenceTarget(PAGE_ID, { name: 'Select flight', interactiveOnly: true })

    expect(queryElementsByNameMock).toHaveBeenCalledWith(PAGE_ID, { name: 'Select flight', text: null })
    expect(result).toEqual({
      targetRef: null,
      targetRefSource: 'specular',
      targetName: 'Select flight',
      targetRect: { x: 100, y: 200, width: 50, height: 20 },
      pageX: 125,
      pageY: 210,
    })
  })

  it('refuses when the live lookup finds more than one rendered match', async () => {
    queryElementsByNameMock.mockResolvedValue([
      { rect: { x: 0, y: 0, width: 10, height: 10 } },
      { rect: { x: 0, y: 100, width: 10, height: 10 } },
    ])

    const result = await findPresenceTarget(PAGE_ID, { name: 'Duplicate label', interactiveOnly: true })

    expect(result).toBeNull()
  })

  it('returns null when the live lookup finds nothing', async () => {
    queryElementsByNameMock.mockResolvedValue([])

    const result = await findPresenceTarget(PAGE_ID, { name: 'Nothing here' })

    expect(result).toBeNull()
  })

  it('never reaches the live lookup once a CSS-selector query already ran', async () => {
    queryPageElementsMock.mockResolvedValue([])

    const result = await findPresenceTarget(PAGE_ID, { selector: '.does-not-exist', name: 'Anything' })

    expect(queryElementsByNameMock).not.toHaveBeenCalled()
    expect(result).toBeNull()
  })

  it('falls back for a text= query too, passing text through (not name)', async () => {
    queryElementsByNameMock.mockResolvedValue([{ rect: { x: 5, y: 5, width: 5, height: 5 } }])

    const result = await findPresenceTarget(PAGE_ID, { text: 'Some visible text' })

    expect(queryElementsByNameMock).toHaveBeenCalledWith(PAGE_ID, { name: null, text: 'Some visible text' })
    expect(result?.targetRect).toEqual({ x: 5, y: 5, width: 5, height: 5 })
  })
})
