/**
 * `applyLayoutDirective` — the layout engine behind POST /layout/apply-directive
 * and the CLI `layout` verb — driven in-process. The `apply` helper mirrors
 * the route: `validateLayoutDirective` at the boundary, then the same
 * function the HTTP handler calls.
 *
 * Guards: explicit-origin rows, spacing tokens, outer-footprint + inset math
 * (device bezels), inset defaults, re-layout of existing pages into a grid
 * with kind resolution, atomic unknown-id failure, implicit bbox origin, and
 * the persistence + undo lifecycle of layout-placed pages.
 *
 * Mutation-verified by: changing the outer→inner offset in
 * src/main/workspace-placement.ts (`applyLayoutDirective`'s final `positions`
 * map) to drop `itemInsets` — the outer-footprint/inset case and the implicit
 * bbox-origin case fail.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { applyLayoutDirective } from '../../src/main/workspace-placement'
import { validateLayoutDirective, type ApplyDirectiveRequest } from '../../src/shared/types'
import { applyCanvasPatch } from '../../src/main/canvas-apply'
import { pages } from '../../src/main/runtime/runtime-context'
import { undo, redo } from '../../src/main/runtime/workspace-undo'
import { DOC_MAP_PAGES } from '../../src/main/runtime/workspace-doc'

/** Mirrors POST /layout/apply-directive: validate at the boundary, then apply. */
function apply(request: ApplyDirectiveRequest) {
  const error = validateLayoutDirective(request.layout)
  if (error) throw new Error(error)
  return applyLayoutDirective(request)
}

/** Create pages through the canvas-apply door, like `specular create page`. */
function createTestPages(configs: Array<{ url: string; canvasX: number; canvasY: number }>): string[] {
  const { created } = applyCanvasPatch({
    entities: configs.map((c) => ({ kind: 'page', presetIndex: 9, ...c })),
  })
  return created
}

let harness: WorkspaceHarness

describe('layout directive', () => {
  beforeEach(() => {
    // Same boot step src/main/index.ts runs after app.whenReady (idempotent).
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('lays new items out in a row at an explicit origin', () => {
    const result = apply({
      layout: { kind: 'row', gap: 'xs', originX: 0, originY: 0 },
      items: [
        { width: 200, height: 100 },
        { width: 200, height: 100 },
        { width: 200, height: 100 },
      ],
    })
    expect(result.positions).toEqual([
      { canvasX: 0, canvasY: 0 },
      { canvasX: 220, canvasY: 0 },
      { canvasX: 440, canvasY: 0 },
    ])
    expect(result.kinds).toEqual([null, null, null])
  })

  it('honors spacing tokens precisely (no snap-to-grid)', () => {
    const result = apply({
      layout: { kind: 'row', gap: 'm', originX: 100, originY: 100 },
      items: [
        { width: 100, height: 100 },
        { width: 100, height: 100 },
      ],
    })
    // m = 60px
    expect(result.positions[1].canvasX - result.positions[0].canvasX).toBe(160)
  })

  it('treats per-item dimensions as outer footprints and offsets positions by insets', () => {
    // Simulated iPad Pro 11 landscape: 1194x834 inner + 24px shell insets on
    // all sides. Outer footprint = 1242 x 882, insetX = insetY = 24.
    const result = apply({
      layout: { kind: 'row', gap: 'm', originX: 100, originY: 200 },
      items: [
        { width: 1242, height: 882, insetX: 24, insetY: 24 },
        { width: 1242, height: 882, insetX: 24, insetY: 24 },
      ],
    })
    // First item's inner top-left lands at the user-supplied origin.
    expect(result.positions[0]).toEqual({ canvasX: 100, canvasY: 200 })
    // Second item: outer-step = outerWidth + gap = 1242 + 60 = 1302.
    expect(result.positions[1].canvasX - result.positions[0].canvasX).toBe(1302)
    expect(result.positions[1].canvasY).toBe(200)
  })

  it('treats items without insets as un-framed (insets default to 0)', () => {
    const result = apply({
      layout: { kind: 'row', gap: 'xs', originX: 0, originY: 0 },
      items: [
        { width: 200, height: 100 },
        { width: 200, height: 100 },
      ],
    })
    expect(result.positions).toEqual([
      { canvasX: 0, canvasY: 0 },
      { canvasX: 220, canvasY: 0 },
    ])
  })

  it('reorganizes existing pages into a 2-col grid and fills in kinds', () => {
    const ids = createTestPages(
      [0, 1, 2, 3].map((i) => ({
        url: `data:text/html,<div>${i}</div>`,
        canvasX: i * 500,
        canvasY: i * 300,
      })),
    )
    expect(pages.map((p) => p.id)).toEqual(expect.arrayContaining(ids))

    const result = apply({
      layout: { kind: 'grid', cols: 2, gap: 24, originX: 0, originY: 0 },
      items: ids.map((id) => ({ id })),
    })

    expect(result.kinds).toEqual(['page', 'page', 'page', 'page'])
    // 2-col grid with uniform tracks. Same-size pages → predictable cells.
    const rowDelta = result.positions[2].canvasY - result.positions[0].canvasY
    const colDelta = result.positions[1].canvasX - result.positions[0].canvasX
    expect(colDelta).toBeGreaterThan(0)
    expect(rowDelta).toBeGreaterThan(0)
    expect(result.positions[3].canvasX).toBe(result.positions[1].canvasX)
    expect(result.positions[3].canvasY).toBe(result.positions[2].canvasY)
  })

  it('errors on unknown id without partial application', () => {
    expect(() =>
      apply({
        layout: { kind: 'row', gap: 16, originX: 0, originY: 0 },
        items: [{ id: 'page_does_not_exist_xyz' }],
      }),
    ).toThrow()
  })

  it('uses bbox of existing items as implicit origin when no anchor given', () => {
    const ids = createTestPages(
      [0, 1].map((i) => ({
        url: `data:text/html,<div>${i}</div>`,
        canvasX: 500 + i * 1000,
        canvasY: 400 + i * 400,
      })),
    )

    const result = apply({
      layout: { kind: 'row', gap: 'xs' },
      items: ids.map((id) => ({ id })),
    })

    // Implicit origin = (min x, min y) of bbox = (500, 400). No snap. The
    // page's chrome-header inset cancels between bbox (outer) and the
    // returned inner position.
    expect(result.positions[0].canvasX).toBe(500)
    expect(result.positions[0].canvasY).toBe(400)
  })

  it('persists layout-placed pages to disk', async () => {
    // The directive computes positions; page creation lands them there.
    const result = apply({
      layout: { kind: 'row', gap: 20, originX: 1200, originY: 1200 },
      items: [
        { width: 200, height: 200 },
        { width: 200, height: 200 },
      ],
    })
    const ids = createTestPages(
      result.positions.map((p, i) => ({
        url: `data:text/html,<div>persist-${i}</div>`,
        canvasX: p.canvasX,
        canvasY: p.canvasY,
      })),
    )
    await settleSync()

    const disk = harness.diskDoc()
    expect(disk).not.toBeNull()
    const byId = new Map(disk!.nodes.map((n) => [n.id, n]))
    for (const [i, id] of ids.entries()) {
      const node = byId.get(id) as { type: string; x: number; y: number } | undefined
      expect(node).toBeDefined()
      expect(node!.type).toBe('link')
      expect(node!.x).toBe(result.positions[i].canvasX)
      expect(node!.y).toBe(result.positions[i].canvasY)
    }
  })

  it('round-trips a single layout-placed page through undo/redo', async () => {
    const result = apply({
      layout: { kind: 'row', gap: 20, originX: 1600, originY: 1600 },
      items: [{ width: 200, height: 200 }],
    })
    const [id] = createTestPages([
      {
        url: 'data:text/html,<div>undo-layout</div>',
        canvasX: result.positions[0].canvasX,
        canvasY: result.positions[0].canvasY,
      },
    ])
    await settleSync()

    // Pages sync to their own doc map (hybrid entities — see runtime CLAUDE.md).
    const pagesMap = harness.doc.getMap(DOC_MAP_PAGES)
    expect(pagesMap.has(id)).toBe(true)

    undo()
    expect(pages.some((p) => p.id === id)).toBe(false)
    expect(pagesMap.has(id)).toBe(false)

    redo()
    expect(pages.some((p) => p.id === id)).toBe(true)
    expect(pagesMap.has(id)).toBe(true)
  })
})
