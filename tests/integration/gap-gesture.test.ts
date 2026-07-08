/**
 * Gap-resize gesture (ADR 0015 Milestone 2) against the real runtime,
 * in-process.
 *
 * Drives the same start → move* → commit | cancel sequence the
 * `canvas-gap-resize-*` IPC handlers run and asserts the §6 I5 contract:
 * move ticks update only the broadcast `resizing-gap` interaction state
 * (runtime positions and the group's `layoutGap` stay untouched), commit
 * reflows once via `setGroupLayoutGap` as a single undo step, and cancel
 * leaves no mutation behind.
 *
 * Mutation-verified by:
 *   - making `moveGapGesture` call `setGroupLayoutGap` per tick — the
 *     "moves leave the doc untouched" case fails.
 *   - dropping the axis projection in `moveGapGesture` (always using x) —
 *     the column case fails.
 *   - skipping `setGroupLayoutGap` in `commitGapGesture` — the commit case
 *     fails.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { createTextEntity, getTextEntities } from '../../src/main/runtime/document-commands'
import { makeAutoLayoutGroup } from '../../src/main/managed-layout'
import {
  cancelGapGesture,
  commitGapGesture,
  moveGapGesture,
  startGapGesture,
} from '../../src/main/gap-gesture'
import { currentInteractionState } from '../../src/main/runtime/interaction-state'
import { selectEntities } from '../../src/main/runtime/selection-controller'
import { __resetForTests as resetInteraction } from '../../src/main/runtime/interaction-controller'
import { workspaceGroups } from '../../src/main/runtime/workspace-model'
import { undo } from '../../src/main/runtime/workspace-undo'

let harness: WorkspaceHarness

const SIZE = { width: 100, height: 100 }

function textAt(canvasX: number, canvasY: number, text: string) {
  return createTextEntity({ canvasX, canvasY, text, ...SIZE })
}

function xs(ids: string[]): number[] {
  return ids.map((id) => {
    const entity = getTextEntities().find((t) => t.id === id)
    if (!entity) throw new Error(`missing text entity ${id}`)
    return entity.canvasX
  })
}

describe('gap-resize gesture', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    resetInteraction()
  })

  afterAll(() => harness?.dispose())

  it('start enters resizing-gap; moves preview only; commit reflows in one undo step', async () => {
    const a = textAt(0, 0, 'a')
    const b = textAt(200, 0, 'b')
    const c = textAt(400, 0, 'c')
    const group = makeAutoLayoutGroup({ entityIds: [a.id, b.id, c.id] })!
    await settleSync()
    // Reflowed at the default gutter (80): 0, 180, 360.
    expect(xs([a.id, b.id, c.id])).toEqual([0, 180, 360])

    // Grab the first gap strip and drag 60px left along the row axis.
    expect(startGapGesture(group.id, 140, 20)).toBe(true)
    expect(currentInteractionState()).toEqual({
      kind: 'resizing-gap',
      groupId: group.id,
      entityIds: [a.id, b.id, c.id],
      gap: 80,
      axis: 'x',
    })

    moveGapGesture(100, 20)
    moveGapGesture(80, 20)
    await settleSync()

    // §6 I5: ticks update only the broadcast gap — no doc/runtime writes.
    const state = currentInteractionState()
    expect(state).toMatchObject({ kind: 'resizing-gap', groupId: group.id, gap: 20, axis: 'x' })
    expect(xs([a.id, b.id, c.id])).toEqual([0, 180, 360])
    expect(workspaceGroups.find((g) => g.id === group.id)?.layoutGap).toBeUndefined()

    // Commit writes once and reflows at the new gap.
    expect(commitGapGesture()).toBe(true)
    await settleSync()
    expect(currentInteractionState().kind).toBe('idle')
    expect(workspaceGroups.find((g) => g.id === group.id)?.layoutGap).toBe(20)
    expect(xs([a.id, b.id, c.id])).toEqual([0, 120, 240])

    // One undo step reverts the gap field and the reflowed positions together.
    undo()
    expect(workspaceGroups.find((g) => g.id === group.id)?.layoutGap).toBeUndefined()
    expect(xs([a.id, b.id, c.id])).toEqual([0, 180, 360])
  })

  it('projects the drag onto the column axis for column groups', async () => {
    const a = textAt(0, 0, 'a')
    const b = textAt(0, 240, 'b')
    const group = makeAutoLayoutGroup({ entityIds: [a.id, b.id] })!
    expect(group.layoutMode).toBe('column')
    await settleSync()

    expect(startGapGesture(group.id, 20, 140)).toBe(true)
    // Horizontal movement is ignored; vertical movement widens the gap.
    moveGapGesture(500, 190)
    expect(currentInteractionState()).toMatchObject({ kind: 'resizing-gap', gap: 130, axis: 'y' })

    expect(commitGapGesture()).toBe(true)
    await settleSync()
    expect(workspaceGroups.find((g) => g.id === group.id)?.layoutGap).toBe(130)
    const ys = [a.id, b.id].map(
      (id) => getTextEntities().find((t) => t.id === id)!.canvasY,
    )
    expect(ys).toEqual([0, 230])
  })

  it('clamps the previewed gap at zero', async () => {
    const a = textAt(0, 0, 'a')
    const b = textAt(200, 0, 'b')
    const group = makeAutoLayoutGroup({ entityIds: [a.id, b.id] })!
    await settleSync()

    startGapGesture(group.id, 140, 20)
    moveGapGesture(-500, 20)
    expect(currentInteractionState()).toMatchObject({ kind: 'resizing-gap', gap: 0 })
    commitGapGesture()
    await settleSync()
    expect(workspaceGroups.find((g) => g.id === group.id)?.layoutGap).toBe(0)
  })

  it('cancel restores pre-drag state with no mutation', async () => {
    const a = textAt(0, 0, 'a')
    const b = textAt(200, 0, 'b')
    const group = makeAutoLayoutGroup({ entityIds: [a.id, b.id] })!
    await settleSync()

    startGapGesture(group.id, 140, 20)
    moveGapGesture(300, 20)
    cancelGapGesture('escape')
    await settleSync()

    expect(currentInteractionState().kind).toBe('idle')
    expect(workspaceGroups.find((g) => g.id === group.id)?.layoutGap).toBeUndefined()
    expect(xs([a.id, b.id])).toEqual([0, 180])

    // A later move/commit from the dead gesture is a clean no-op.
    moveGapGesture(999, 20)
    expect(commitGapGesture()).toBe(false)
  })

  it('refuses to start on a non-managed group', async () => {
    expect(startGapGesture('nope', 0, 0)).toBe(false)
    expect(currentInteractionState().kind).toBe('idle')
  })

  it('selection door: commits by moving the entities, one undo step, cross axis kept', async () => {
    // Equal 100px gaps along x, deliberately unaligned on y.
    const a = textAt(0, 0, 'a')
    const b = textAt(200, 10, 'b')
    const c = textAt(400, 20, 'c')
    selectEntities([a.id, b.id, c.id])
    await settleSync()

    expect(startGapGesture(null, 150, 20)).toBe(true)
    expect(currentInteractionState()).toEqual({
      kind: 'resizing-gap',
      groupId: null,
      entityIds: [a.id, b.id, c.id],
      gap: 100,
      axis: 'x',
    })

    // §6 I5: ticks update only the broadcast gap — positions stay untouched.
    moveGapGesture(90, 20)
    await settleSync()
    expect(currentInteractionState()).toMatchObject({ kind: 'resizing-gap', gap: 40 })
    expect(xs([a.id, b.id, c.id])).toEqual([0, 200, 400])

    // Commit repacks at the new gap; each entity keeps its own y.
    expect(commitGapGesture()).toBe(true)
    await settleSync()
    expect(currentInteractionState().kind).toBe('idle')
    expect(xs([a.id, b.id, c.id])).toEqual([0, 140, 280])
    const ys = [a.id, b.id, c.id].map(
      (id) => getTextEntities().find((t) => t.id === id)!.canvasY,
    )
    expect(ys).toEqual([0, 10, 20])

    // One undo step restores all positions together.
    undo()
    expect(xs([a.id, b.id, c.id])).toEqual([0, 200, 400])
  })

  it('selection door: refuses an unequal-gap selection; cancel leaves no mutation', async () => {
    const a = textAt(0, 0, 'a')
    const b = textAt(200, 0, 'b')
    const c = textAt(500, 0, 'c') // gaps 100 / 200 — not a row
    selectEntities([a.id, b.id, c.id])
    await settleSync()
    expect(startGapGesture(null, 150, 20)).toBe(false)
    expect(currentInteractionState().kind).toBe('idle')

    // An eligible selection cancels cleanly: no positions written.
    selectEntities([a.id, b.id])
    expect(startGapGesture(null, 150, 20)).toBe(true)
    moveGapGesture(300, 20)
    cancelGapGesture('escape')
    await settleSync()
    expect(currentInteractionState().kind).toBe('idle')
    expect(xs([a.id, b.id])).toEqual([0, 200])
  })
})
