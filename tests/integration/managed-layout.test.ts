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
 *   - dropping the `setGroupLayoutGap` delegation from the group kind's
 *     `update` handler — the apply-patch case fails.
 *   - routing `layoutGap` through the plain `updateGroupEntity` field patch
 *     instead — the unmanaged-group rejection case fails.
 *   - dropping the `input.gap` write from `makeAutoLayoutGroup` — the
 *     create-with-gap case fails.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import type { JsonCanvasGroupNode } from '../../src/shared/json-canvas-types'
import { applyCanvasPatch } from '../../src/main/canvas-apply'
import { createTextEntity, getTextEntities } from '../../src/main/runtime/document-commands'
import {
  makeAutoLayoutGroup,
  reorderManagedChild,
  setGroupLayoutGap,
} from '../../src/main/managed-layout'
import { createUserGroup } from '../../src/main/workspace-groups'
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

  it('a layoutGap apply patch reflows a managed group through the registry', async () => {
    const a = textAt(0, 0, 'a')
    const b = textAt(200, 0, 'b')
    const group = makeAutoLayoutGroup({ entityIds: [a.id, b.id] })!
    await settleSync()
    expect(positionsOf([a.id, b.id]).map((p) => p.x)).toEqual([0, 180])

    // The same door `specular update <groupId> --gap 20` compiles to.
    applyCanvasPatch({ entities: [{ id: group.id, layoutGap: 20 }] })
    await settleSync()

    expect(workspaceGroups.find((g) => g.id === group.id)?.layoutGap).toBe(20)
    expect(positionsOf([a.id, b.id]).map((p) => p.x)).toEqual([0, 120])
  })

  it('a layoutGap patch no-ops on an unmanaged group and on a non-finite value', async () => {
    const a = textAt(0, 0, 'a')
    const b = textAt(200, 0, 'b')
    const plain = createUserGroup([a.id, b.id])
    await settleSync()

    applyCanvasPatch({ entities: [{ id: plain.id, layoutGap: 40 }] })
    await settleSync()
    expect(workspaceGroups.find((g) => g.id === plain.id)?.layoutGap).toBeUndefined()
    expect(positionsOf([a.id, b.id]).map((p) => p.x)).toEqual([0, 200])

    const managed = makeAutoLayoutGroup({ groupId: plain.id })!
    await settleSync()
    applyCanvasPatch({ entities: [{ id: managed.id, layoutGap: Number.NaN }] })
    await settleSync()
    expect(workspaceGroups.find((g) => g.id === managed.id)?.layoutGap).toBeUndefined()
  })

  it('auto-layout with a gap creates the group already packed at that gap', async () => {
    const a = textAt(0, 0, 'a')
    const b = textAt(200, 0, 'b')
    const group = makeAutoLayoutGroup({ entityIds: [a.id, b.id], gap: 24 })!
    await settleSync()

    expect(workspaceGroups.find((g) => g.id === group.id)?.layoutGap).toBe(24)
    expect(positionsOf([a.id, b.id]).map((p) => p.x)).toEqual([0, 124])
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
