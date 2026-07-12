/**
 * D8 (issue #318): generation-based staleness detection is warn-only, never
 * a hard block.
 *
 * Main tracks a per-page `navGeneration` counter that browse-handler.ts
 * later compares against the generation a snapshot saw, to warn an agent
 * when it reuses `@eN` refs against a page that navigated in between. This
 * file guards the counter itself: it starts at 0 and bumps on both
 * `did-navigate` and `dom-ready`. Both listeners exist because a full
 * navigation typically fires both, but HMR partial updates only fire
 * `dom-ready` — the downstream comparison is `>`, not `+1`, so
 * double-counting on a full navigation is harmless.
 *
 * The `POST /pages/:id/snapshot-seen` block guards the other half of D8:
 * the snapshot-time baseline must land on the main-process Page object,
 * because every `specular` CLI invocation is a fresh process — a baseline
 * held CLI-side would be gone before the mutation that needs it. The route
 * stamps the page's own CURRENT navGeneration (it ignores any client body):
 * a long-lived client (the MCP server) reads generations through a 60s
 * cache, and a stale-low client-supplied baseline would fire false
 * staleness warnings on fresh snapshots.
 *
 * Mutation-verified by: deleting `page.navGeneration += 1` from the
 * `did-navigate` handler in src/main/runtime/page-factory.ts — "bumps the
 * generation on did-navigate" fails (generation stays at 0); by deleting
 * the dedicated `dom-ready` listener block in the same file — "bumps the
 * generation on dom-ready" fails (generation stays at 0 instead of
 * incrementing before did-navigate ever fires, which is the HMR case this
 * counter exists to approximate); and by changing the snapshot-seen route
 * in src/main/routes/pages.ts to stamp a constant instead of
 * `page.navGeneration` — "stamps the page's current navGeneration" fails.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import type { ServerResponse } from 'node:http'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { applyCanvasPatch } from '../../src/main/canvas-apply'
import { findPageById } from '../../src/main/runtime/runtime-context'
import { pageRoutes } from '../../src/main/routes/pages'

let harness: WorkspaceHarness

function createPage(): string {
  const result = applyCanvasPatch({
    entities: [{ kind: 'page', url: 'https://example.com', canvasX: 0, canvasY: 0, presetIndex: 2 }],
  })
  return result.created[0]
}

describe('page navigation generation (D8)', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('starts at 0 for a newly created page', async () => {
    const pageId = createPage()
    await settleSync()

    expect(findPageById(pageId)?.navGeneration).toBe(0)
  })

  it('bumps on did-navigate', async () => {
    const pageId = createPage()
    await settleSync()
    const page = findPageById(pageId)!

    page.pageView.webContents.emit('did-navigate', {}, 'https://example.com/next')

    expect(page.navGeneration).toBe(1)
  })

  it('bumps on dom-ready — the only signal HMR partial updates fire', async () => {
    const pageId = createPage()
    await settleSync()
    const page = findPageById(pageId)!

    page.pageView.webContents.emit('dom-ready')

    expect(page.navGeneration).toBe(1)
  })

  it('a full navigation firing both events still only needs `>` downstream, not exact tracking', async () => {
    const pageId = createPage()
    await settleSync()
    const page = findPageById(pageId)!
    const seenGeneration = page.navGeneration

    page.pageView.webContents.emit('dom-ready')
    page.pageView.webContents.emit('did-navigate', {}, 'https://example.com/next')

    expect(page.navGeneration).toBeGreaterThan(seenGeneration)
  })

  it('each page tracks its own generation independently', async () => {
    const pageIdA = createPage()
    const pageIdB = createPage()
    await settleSync()
    const pageA = findPageById(pageIdA)!
    const pageB = findPageById(pageIdB)!

    pageA.pageView.webContents.emit('did-navigate', {}, 'https://example.com/a-next')

    expect(pageA.navGeneration).toBe(1)
    expect(pageB.navGeneration).toBe(0)
  })
})

describe('POST /pages/:id/snapshot-seen (D8 baseline)', () => {
  const route = pageRoutes.find(
    (r) => r.method === 'POST' && r.pattern instanceof RegExp && r.pattern.test('/pages/x/snapshot-seen'),
  )!

  function invoke(pageId: string, body: unknown) {
    let status = 200
    let json: unknown
    const response = {
      statusCode: 200,
      setHeader() {},
      end(payload?: string) {
        status = response.statusCode
        json = payload ? JSON.parse(payload) : undefined
      },
    } as unknown as ServerResponse
    return route
      .handler({ response, body, params: { 0: pageId } } as never)
      .then(() => ({ status, json: json as { ok?: boolean; error?: string } }))
  }

  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it("stamps the page's current navGeneration, ignoring any client-supplied number", async () => {
    const pageId = createPage()
    await settleSync()
    const page = findPageById(pageId)!
    expect(page.lastAgentSnapshotGeneration).toBeUndefined()
    page.pageView.webContents.emit('did-navigate', {}, 'https://example.com/next')
    page.pageView.webContents.emit('dom-ready')

    const { status, json } = await invoke(pageId, { generation: 999 })

    expect(status).toBe(200)
    expect(json.ok).toBe(true)
    expect(page.lastAgentSnapshotGeneration).toBe(page.navGeneration)
    expect(page.lastAgentSnapshotGeneration).toBe(2)
  })

  it('404s for an unknown page without recording anything', async () => {
    const { status, json } = await invoke('page_nope', {})

    expect(status).toBe(404)
    expect(json.error).toContain('page_nope')
  })
})
