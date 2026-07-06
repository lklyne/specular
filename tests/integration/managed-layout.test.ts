/**
 * Managed layout (auto-layout groups) against the real runtime, in-process.
 *
 * Drives `makeAutoLayoutGroup` / `setGroupLayoutGap` / `reorderManagedChild` —
 * the same mutators the reorder gesture and headless routes call — and asserts
 * on runtime positions, `.canvas` bytes, and undo behavior: the persisted
 * `layoutGap` round-trips disk and undo, a gap change reflows in one undo step,
 * and a vertically-stacked selection converts to a working `column` group.
 *
 * Mutation-verified by:
 *   - dropping `layoutGap: entity.layoutGap` from `serializeGroupEntityToGroupNode`
 *     — the disk round-trip case fails.
 *   - hardcoding `CLUSTER_HORIZONTAL_GUTTER` back into `reflowManagedGroup` —
 *     the gap-reflow case fails.
 *   - forcing `axis = 'x'` in `reflowManagedGroup` — both column cases fail.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import type { JsonCanvasGroupNode } from '../../src/shared/json-canvas-types'
import { createTextEntity, getTextEntities } from '../../src/main/runtime/document-commands'
import {
  makeAutoLayoutGroup,
  reorderManagedChild,
  setGroupLayoutGap,
} from '../../src/main/managed-layout'
import { workspaceGroups } from '../../src/main/runtime/workspace-model'
import { undo } from '../../src/main/runtime/workspace-undo'

let harness: WorkspaceHarness

const SIZE = { width: 100, height: 100 }

function textAt(canvasX: number, canvasY: number, text: string) {
  return createTextEntity({ canvasX, canvasY, text, ...SIZE })
}

function positionsOf(ids: string[]): Array<{ x: number; y: number }> {
  return ids.map((id) => {
    const entity = getTextEntities().find((t) => t.id === id)
    if (!entity) throw new Error(`missing text entity ${id}`)
    return { x: entity.canvasX, y: entity.canvasY }
  })
}

describe('managed layout', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('layoutGap persists to .canvas, survives a reload, and round-trips undo', async () => {
    const a = textAt(0, 0, 'a')
    const b = textAt(200, 0, 'b')
    const group = makeAutoLayoutGroup({ entityIds: [a.id, b.id] })!
    await settleSync()

    expect(setGroupLayoutGap(group.id, 40)).toBe(true)
    await settleSync()

    const node = harness
      .diskDoc()!
      .nodes.find((n): n is JsonCanvasGroupNode => n.id === group.id)
    expect(node?.layoutGap).toBe(40)

    // Reload the serialized workspace: the deserialize + restore path keeps it.
    harness.loadFixture({ doc: harness.diskDoc()! })
    expect(workspaceGroups.find((g) => g.id === group.id)?.layoutGap).toBe(40)
  })

  it('setGroupLayoutGap reflows children at the new gap and undoes in one step', async () => {
    const a = textAt(0, 0, 'a')
    const b = textAt(200, 0, 'b')
    const c = textAt(400, 0, 'c')
    const group = makeAutoLayoutGroup({ entityIds: [a.id, b.id, c.id] })!
    expect(group.layoutMode).toBe('row')
    await settleSync()

    // Default gutter (80): 0, 180, 360.
    expect(positionsOf([a.id, b.id, c.id]).map((p) => p.x)).toEqual([0, 180, 360])

    setGroupLayoutGap(group.id, 40)
    await settleSync()
    expect(positionsOf([a.id, b.id, c.id]).map((p) => p.x)).toEqual([0, 140, 280])

    // One undo step reverts the gap field and the reflowed positions together.
    undo()
    expect(workspaceGroups.find((g) => g.id === group.id)?.layoutGap).toBeUndefined()
    expect(positionsOf([a.id, b.id, c.id]).map((p) => p.x)).toEqual([0, 180, 360])
  })

  it('a vertical selection converts to a column group and reflows vertically', async () => {
    const a = textAt(0, 0, 'a')
    const b = textAt(0, 240, 'b')
    const c = textAt(0, 480, 'c')
    const group = makeAutoLayoutGroup({ entityIds: [a.id, b.id, c.id] })!
    await settleSync()

    expect(group.layoutMode).toBe('column')
    // Packed top-to-bottom at the default gutter, sharing the column's x.
    expect(positionsOf([a.id, b.id, c.id])).toEqual([
      { x: 0, y: 0 },
      { x: 0, y: 180 },
      { x: 0, y: 360 },
    ])
  })

  it('reorderManagedChild reorders a column group along y', async () => {
    const a = textAt(0, 0, 'a')
    const b = textAt(0, 240, 'b')
    const c = textAt(0, 480, 'c')
    const group = makeAutoLayoutGroup({ entityIds: [a.id, b.id, c.id] })!
    await settleSync()

    expect(reorderManagedChild(group.id, a.id, 2)).toBe(true)
    await settleSync()

    // New order b, c, a — packed vertically.
    expect(positionsOf([b.id, c.id, a.id]).map((p) => p.y)).toEqual([0, 180, 360])
  })
})
