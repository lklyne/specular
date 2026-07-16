/**
 * Phase 1 of scroll tracking (docs/plans/scroll-tracking.md): a page's absolute
 * scroll offset in raw CSS pixels rides the layout broadcast. The page preload
 * reports `{ scrollX, scrollY }` on a dedicated always-on channel, main stores
 * it on the ephemeral runtime page (`page.scrollX` / `page.scrollY`), and the
 * `page` scene entity carries it so downstream phases can scroll-follow.
 *
 * This exercises the runtime→scene-entity leg the same way the IPC handler
 * does: set the offset on the runtime page, then read it back off the layout
 * broadcast's page entity.
 *
 * Mutation-verified by:
 * - dropping `scrollY: page.scrollY ?? 0` from the `backgroundPageOverlays`
 *   scene-entity literal (canvas-layout-data.ts) — the scrollY assertion fails;
 * - dropping `scrollX: page.scrollX ?? 0` from the same literal — the scrollX
 *   assertion fails.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import type { JsonCanvasLinkNode } from '../../src/shared/json-canvas-types'
import { getCanvasLayoutData } from '../../src/main/runtime/canvas-layout-data'
import { pages } from '../../src/main/runtime/runtime-context'

let harness: WorkspaceHarness

const PAGE_ID = 'page-scroll-host'
const PAGE_URL = 'https://example.com/long'

function hostPageNode(): JsonCanvasLinkNode {
  return {
    id: PAGE_ID,
    type: 'link',
    x: 100,
    y: 100,
    width: 375,
    height: 667,
    url: PAGE_URL,
    presetIndex: 0,
  }
}

function loadHostPage(): void {
  harness.loadFixture({
    name: 'Scroll offset host',
    doc: {
      nodes: [hostPageNode()],
      edges: [],
      appState: { zoom: 1, pan: { x: 0, y: 0 } },
    },
  })
}

function pageSceneEntity(id: string): { scrollX: number; scrollY: number } {
  const entity = getCanvasLayoutData().entities.find(
    (candidate) => candidate.kind === 'page' && candidate.id === id,
  )
  if (!entity || entity.kind !== 'page') throw new Error(`no page entity: ${id}`)
  return { scrollX: entity.scrollX, scrollY: entity.scrollY }
}

describe('page scroll offset in the layout broadcast', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('defaults to zero and then carries the runtime page offset', () => {
    loadHostPage()
    expect(pageSceneEntity(PAGE_ID)).toEqual({ scrollX: 0, scrollY: 0 })

    const page = pages.find((candidate) => candidate.id === PAGE_ID)!
    page.scrollX = 42
    page.scrollY = 1337

    expect(pageSceneEntity(PAGE_ID)).toEqual({ scrollX: 42, scrollY: 1337 })
  })

  it('resets the offset when the page navigates to a new document', () => {
    loadHostPage()
    const page = pages.find((candidate) => candidate.id === PAGE_ID)!
    page.scrollX = 42
    page.scrollY = 1337
    page.scrollHeight = 5000

    page.pageView.webContents.emit('did-navigate', {}, 'https://example.com/other')

    expect(pageSceneEntity(PAGE_ID)).toEqual({ scrollX: 0, scrollY: 0 })
    expect(page.scrollHeight).toBe(0)
  })
})
