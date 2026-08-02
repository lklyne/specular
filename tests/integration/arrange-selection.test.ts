/**
 * `arrangeEntities` — the popup toolbar's row/column/grid button, driven
 * in-process against the real runtime. The default path tidies in place (keeps
 * the cluster's footprint, evens the gaps via the `arrangeInSpan` kernel,
 * covered by span-arrange.test.ts); an explicit `gap` packs tight instead. This
 * suite adds the commit path: drawing-stroke travel, group-descendant carry,
 * single-undo-step batching, and the span-vs-pack branch.
 *
 * Mutation-verified by:
 *   - dropping the `shiftDrawingStrokes(id, dx, dy)` call in `moveEntityTo`
 *     (src/main/runtime/document-commands.ts) — the drawing-stroke case fails.
 *   - returning early instead of walking `descendantEntityIdsForGroup` — the
 *     group case fails (the group moves in y, so the child must follow).
 *   - removing the gesture session (bare mutation) — the single-undo case fails.
 *   - routing the no-gap call through `packEntities` — the span row's
 *     redistributed middle position (300, not a packed 120) fails.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import {
  arrangeEntities,
  createDrawingEntity,
  createShapeEntity,
  createTextEntity,
  getDrawingEntities,
  getShapeEntities,
  getTextEntities,
} from '../../src/main/runtime/document-commands'
import { createUserGroup } from '../../src/main/workspace-groups'
import { workspaceGroups } from '../../src/main/runtime/space-model'
import { selectEntities, selectNone } from '../../src/main/runtime/selection-controller'
import { undo } from '../../src/main/runtime/space-undo'
import { workspaceRoutes } from '../../src/main/routes/workspace'
import type { ServerResponse } from 'node:http'

let harness: WorkspaceHarness

function shapePos(id: string) {
  const s = getShapeEntities().find((e) => e.id === id)!
  return { x: s.canvasX, y: s.canvasY }
}

describe('arrange selection', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
  })

  afterAll(() => harness?.dispose())

  it('tidies a scattered row: footprint pinned, gaps evened, one undo restores it', async () => {
    // Scattered x (0, 150, 600) → the outer two are pinned and the middle one
    // slides to even the gaps; span-preserving, NOT packed to the top-left.
    const a = createShapeEntity({ canvasX: 0, canvasY: 0, width: 100, height: 100 })
    const b = createShapeEntity({ canvasX: 150, canvasY: 40, width: 100, height: 100 })
    const c = createShapeEntity({ canvasX: 600, canvasY: 10, width: 100, height: 100 })
    await settleSync()

    expect(arrangeEntities([a.id, b.id, c.id], 'row')).toBe(true)
    await settleSync()

    const pa = shapePos(a.id)
    const pb = shapePos(b.id)
    const pc = shapePos(c.id)
    // Endpoints pinned to the original extent; middle redistributed to 300.
    expect(pa).toEqual({ x: 0, y: 0 })
    expect(pc.x).toBe(600)
    expect(pb.x).toBe(300)
    // One shared baseline (min y = 0).
    expect(pb.y).toBe(0)
    expect(pc.y).toBe(0)

    // One gesture session ⇒ one undo step reverts the whole arrange.
    undo()
    expect(shapePos(a.id)).toEqual({ x: 0, y: 0 })
    expect(shapePos(b.id)).toEqual({ x: 150, y: 40 })
    expect(shapePos(c.id)).toEqual({ x: 600, y: 10 })
  })

  it('packs tight from the top-left when an explicit gap is given', async () => {
    const a = createShapeEntity({ canvasX: 0, canvasY: 0, width: 100, height: 100 })
    const b = createShapeEntity({ canvasX: 150, canvasY: 40, width: 100, height: 100 })
    const c = createShapeEntity({ canvasX: 600, canvasY: 10, width: 100, height: 100 })
    await settleSync()

    expect(arrangeEntities([a.id, b.id, c.id], 'row', { gap: 20 })).toBe(true)
    await settleSync()

    // gap 20 ⇒ tight pack: 0, 120, 240 — the footprint collapses.
    expect(shapePos(a.id)).toEqual({ x: 0, y: 0 })
    expect(shapePos(b.id)).toEqual({ x: 120, y: 0 })
    expect(shapePos(c.id)).toEqual({ x: 240, y: 0 })
  })

  it('carries drawing strokes with the moved entity', async () => {
    const anchor = createShapeEntity({ canvasX: 0, canvasY: 0, width: 100, height: 100 })
    // Far below so a row arrange must lift it to the baseline; stroke in absolute coords.
    const drawing = createDrawingEntity({
      canvasX: 0,
      canvasY: 500,
      width: 100,
      height: 100,
      strokes: [{ id: 'k', color: '#000', width: 2, points: [{ x: 40, y: 540 }] }],
    })
    await settleSync()

    expect(arrangeEntities([anchor.id, drawing.id], 'row')).toBe(true)

    const moved = getDrawingEntities().find((d) => d.id === drawing.id)!
    // The origin moved; the ink must move by the same delta or it drifts away.
    const dx = moved.canvasX - 0
    const dy = moved.canvasY - 500
    expect(dy).not.toBe(0)
    expect(moved.strokes[0].points[0]).toEqual({ x: 40 + dx, y: 540 + dy })
  })

  it('moves group descendants by the group delta', async () => {
    // Group sits below-right of `other`; a row arrange lifts it to the baseline
    // (min y = 0), so the group moves in y and its child must follow.
    const childA = createTextEntity({ canvasX: 300, canvasY: 200, text: 'a', width: 100, height: 100 })
    const childB = createTextEntity({ canvasX: 340, canvasY: 220, text: 'b', width: 100, height: 100 })
    const group = createUserGroup([childA.id, childB.id], 'g')
    const other = createShapeEntity({ canvasX: 0, canvasY: 0, width: 100, height: 100 })
    await settleSync()

    const g0 = workspaceGroups.find((g) => g.id === group.id)!
    const before = { gx: g0.canvasX, gy: g0.canvasY, ax: childA.canvasX, ay: childA.canvasY }

    expect(arrangeEntities([group.id, other.id], 'row')).toBe(true)

    const g1 = workspaceGroups.find((g) => g.id === group.id)!
    const dx = g1.canvasX - before.gx
    const dy = g1.canvasY - before.gy
    expect(dy).not.toBe(0) // the group actually moved, so the carry is exercised
    const a1 = getTextEntities().find((t) => t.id === childA.id)!
    // Child shifts by exactly the group's delta — independent absolute coords.
    expect(a1.canvasX).toBe(before.ax + dx)
    expect(a1.canvasY).toBe(before.ay + dy)
  })
})

// The CLI's `arrange` verb POSTs here. Guarding the route (not just the mutator)
// keeps the CLI path in sync with the toolbar.
describe('POST /selection/arrange', () => {
  const route = workspaceRoutes.find((r) => r.pattern === '/selection/arrange')!

  function invoke(body: unknown) {
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
      .handler({ response, body } as never)
      .then(() => ({ status, json: json as { changed?: boolean; error?: string } }))
  }

  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
  })

  it('arranges the ids in the body', async () => {
    const a = createShapeEntity({ canvasX: 0, canvasY: 0, width: 100, height: 100 })
    const b = createShapeEntity({ canvasX: 400, canvasY: 300, width: 100, height: 100 })
    await settleSync()

    const { status, json } = await invoke({ mode: 'row', entityIds: [a.id, b.id] })
    expect(status).toBe(200)
    expect(json.changed).toBe(true)
    // Row ⇒ shared baseline.
    expect(shapePos(a.id).y).toBe(shapePos(b.id).y)
  })

  it('falls back to the current selection when no ids are given', async () => {
    const a = createShapeEntity({ canvasX: 0, canvasY: 0, width: 100, height: 100 })
    const b = createShapeEntity({ canvasX: 400, canvasY: 300, width: 100, height: 100 })
    await settleSync()
    selectEntities([a.id, b.id])

    const { json } = await invoke({ mode: 'column' })
    expect(json.changed).toBe(true)
    // Column ⇒ shared left edge.
    expect(shapePos(a.id).x).toBe(shapePos(b.id).x)
  })

  it('rejects an invalid mode with 400', async () => {
    const { status, json } = await invoke({ mode: 'diagonal', entityIds: ['x', 'y'] })
    expect(status).toBe(400)
    expect(json.error).toBeTruthy()
  })
})
