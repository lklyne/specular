/**
 * Undo/redo smoke tests.
 *
 * Drives the global undo stack defined in src/main/runtime/workspace-undo.ts
 * through HTTP test routes. Each test mutates the workspace, calls undo, and
 * checks both the runtime snapshot and (where relevant) on-disk state.
 *
 * Mutation-verified by:
 *   - removing the `setActiveUndoManager(manager)` call inside
 *     `createCanvasUndoManager()` and confirming undo becomes a no-op (every
 *     case below then fails).
 *   - bypassing `requestDocSync()` inside `syncRuntimeToDoc` and confirming
 *     "undo of entity creation removes it from Y.Doc" fails (the entity
 *     never made it into Y.Doc so undo has nothing to revert).
 */

import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import {
  applyMultiResize,
  beginMultiResize,
  createDrawingEntity,
  createGroup,
  createTextEntities,
  deleteDrawingEntities,
  deleteGroups,
  deleteTextEntities,
  endMultiResize,
  getDrawingEntities,
  getTextEntities,
  getUndoState,
  getWorkspace,
  redoWorkspace,
  resetSmokeState,
  undoWorkspace,
  updateDrawingEntity,
} from './app-client'
import { wait } from './test-utils'

async function cleanupTextEntities(): Promise<void> {
  const { textEntities } = await getTextEntities()
  if (textEntities.length) {
    await deleteTextEntities(textEntities.map((t) => t.id))
  }
}

async function cleanupDrawingEntities(): Promise<void> {
  const { drawingEntities } = await getDrawingEntities()
  if (drawingEntities.length) {
    await deleteDrawingEntities(drawingEntities.map((d) => d.id))
  }
}

async function cleanupGroups(): Promise<void> {
  const graph = await getWorkspace()
  const groupIds = (graph.entities as Array<{ id: string; kind: string }>)
    .filter((e) => e.kind === 'group')
    .map((e) => e.id)
  if (groupIds.length) {
    await deleteGroups(groupIds)
  }
}

async function drainUndoStack(): Promise<void> {
  // Pop up to 200 entries so a polluted stack from prior tests can't
  // leak into the next test's expectations.
  for (let i = 0; i < 200; i++) {
    const state = await getUndoState()
    if (!state.canUndo) return
    await undoWorkspace()
  }
}

describe('undo', () => {
  beforeEach(async () => {
    await resetSmokeState()
    await cleanupTextEntities()
    await cleanupGroups()
    await drainUndoStack()
  })

  afterEach(async () => {
    await cleanupTextEntities()
    await cleanupGroups()
  })

  it('undo of entity creation removes the entity from runtime', async () => {
    const before = await getTextEntities()
    const beforeIds = new Set(before.textEntities.map((t) => t.id))

    const { ids } = await createTextEntities([
      { canvasX: 50, canvasY: 50, text: 'will be undone' },
    ])
    const newId = ids[0]
    // Let the forward sync microtask deliver to Y.Doc + UndoManager.
    await wait(50)

    const created = await getTextEntities()
    expect(created.textEntities.some((t) => t.id === newId)).toBe(true)

    await undoWorkspace()

    const undone = await getTextEntities()
    const undoneIds = new Set(undone.textEntities.map((t) => t.id))
    expect(undoneIds.has(newId)).toBe(false)
    // No new entities sneaked in.
    expect(undoneIds.size).toBe(beforeIds.size)
  })

  it('redo replays the creation', async () => {
    const { ids } = await createTextEntities([
      { canvasX: 70, canvasY: 70, text: 'roundtrip' },
    ])
    const newId = ids[0]
    await wait(50)

    await undoWorkspace()
    const undone = await getTextEntities()
    expect(undone.textEntities.some((t) => t.id === newId)).toBe(false)

    await redoWorkspace()
    const redone = await getTextEntities()
    expect(redone.textEntities.some((t) => t.id === newId)).toBe(true)
  })

  it('undo of grouping restores prior membership', async () => {
    // Use single-item creates: the batch path runs entity creation through
    // staggerOperation, which spaces work out across multiple ticks and
    // leaves the undo stack non-deterministic for our test.
    const { ids: idsA } = await createTextEntities([
      { canvasX: 0, canvasY: 0, text: 'a' },
    ])
    await wait(50)
    const { ids: idsB } = await createTextEntities([
      { canvasX: 100, canvasY: 0, text: 'b' },
    ])
    await wait(50)
    const ids = [...idsA, ...idsB]

    const group = await createGroup(ids, 'test group')
    await wait(50)

    const beforeUndoGraph = await getWorkspace()
    expect(
      (beforeUndoGraph.entities as Array<{ id: string; kind: string }>).some(
        (e) => e.kind === 'group' && e.id === group.id,
      ),
    ).toBe(true)

    await undoWorkspace()

    const afterUndoGraph = await getWorkspace()
    expect(
      (afterUndoGraph.entities as Array<{ id: string; kind: string }>).some(
        (e) => e.kind === 'group' && e.id === group.id,
      ),
    ).toBe(false)
    // Members survive the ungrouping.
    const { textEntities } = await getTextEntities()
    const surviving = new Set(textEntities.map((t) => t.id))
    for (const id of ids) {
      expect(surviving.has(id)).toBe(true)
    }
  })

  it('canUndo flips correctly with stack lifecycle', async () => {
    const stateBefore = await getUndoState()
    // The redo stack may carry over from prior tests in the same Electron
    // process — we only assert the undo half is drained.
    expect(stateBefore.canUndo).toBe(false)

    await createTextEntities([{ canvasX: 0, canvasY: 0, text: 'x' }])
    await wait(50)

    const stateAfterCreate = await getUndoState()
    expect(stateAfterCreate.canUndo).toBe(true)

    await undoWorkspace()
    const stateAfterUndo = await getUndoState()
    expect(stateAfterUndo.canRedo).toBe(true)
  })

  it('redo stack clears when a new mutation lands after undo', async () => {
    const { ids: firstIds } = await createTextEntities([
      { canvasX: 0, canvasY: 0, text: 'first' },
    ])
    await wait(50)
    await undoWorkspace()

    expect((await getUndoState()).canRedo).toBe(true)

    // A new mutation after undo should drop the redo stack.
    await createTextEntities([{ canvasX: 50, canvasY: 50, text: 'fork' }])
    await wait(50)

    const state = await getUndoState()
    expect(state.canRedo).toBe(false)
    // First creation should not come back via redo.
    if (state.canRedo) {
      await redoWorkspace()
      const after = await getTextEntities()
      expect(after.textEntities.some((t) => t.id === firstIds[0])).toBe(false)
    }
  })

  it('multi-selection resize gesture collapses to one undo step', async () => {
    // Create two text entities to resize together.
    const { ids: idsA } = await createTextEntities([
      { canvasX: 0, canvasY: 0, text: 'a', width: 100, height: 50 },
    ])
    await wait(50)
    const { ids: idsB } = await createTextEntities([
      { canvasX: 200, canvasY: 0, text: 'b', width: 100, height: 50 },
    ])
    await wait(50)

    const before = await getTextEntities()
    const aInit = before.textEntities.find((t) => t.id === idsA[0])!
    const bInit = before.textEntities.find((t) => t.id === idsB[0])!

    // Simulate a multi-resize gesture: begin → multiple apply ticks → end.
    await beginMultiResize()
    await applyMultiResize([
      { id: idsA[0], kind: 'text', width: 120, height: 60, canvasX: aInit.canvasX, canvasY: aInit.canvasY },
      { id: idsB[0], kind: 'text', width: 120, height: 60, canvasX: bInit.canvasX, canvasY: bInit.canvasY },
    ])
    await applyMultiResize([
      { id: idsA[0], kind: 'text', width: 140, height: 70, canvasX: aInit.canvasX, canvasY: aInit.canvasY },
      { id: idsB[0], kind: 'text', width: 140, height: 70, canvasX: bInit.canvasX, canvasY: bInit.canvasY },
    ])
    await endMultiResize()
    await wait(50)

    // Both entities should be resized.
    const afterResize = await getTextEntities()
    const aResized = afterResize.textEntities.find((t) => t.id === idsA[0])!
    const bResized = afterResize.textEntities.find((t) => t.id === idsB[0])!
    expect(aResized.width).toBe(140)
    expect(bResized.width).toBe(140)

    // One undo should restore both entities to their pre-resize bounds.
    await undoWorkspace()
    await wait(50)

    const afterUndo = await getTextEntities()
    const aUndone = afterUndo.textEntities.find((t) => t.id === idsA[0])!
    const bUndone = afterUndo.textEntities.find((t) => t.id === idsB[0])!
    expect(aUndone.width).toBe(aInit.width)
    expect(bUndone.width).toBe(bInit.width)
  })
})

/**
 * Drawing resize + undo round-trip.
 *
 * Verifies that updateDrawingEntity persists strokes to Y.Doc (forward sync)
 * and that undo restores both bounds and the original stroke geometry (reverse
 * sync). Required by CLAUDE.md test contract for new runtime mutators.
 *
 * Mutation-verified by:
 *   - Removing `if (patch.strokes !== undefined) drawingPatch.strokes = entry.strokes`
 *     in resizeMultiSelection and confirming the stroke assertion after update fails.
 *   - Removing `scheduleWorkspaceAutosave()` from createDrawingEntity and
 *     confirming the "strokes after resize" assertion is never reached (entity
 *     not found on undo).
 */
describe('drawing resize undo', () => {
  beforeEach(async () => {
    await resetSmokeState()
    await cleanupDrawingEntities()
    await drainUndoStack()
  })

  afterEach(async () => {
    await cleanupDrawingEntities()
  })

  it('resize + undo restores original bounds and stroke geometry', async () => {
    const initialStrokes = [
      {
        id: 'stroke-1',
        color: '#ff0000',
        width: 3,
        points: [
          { x: 10, y: 10 },
          { x: 50, y: 50 },
          { x: 90, y: 10 },
        ],
      },
    ]

    const drawing = await createDrawingEntity({
      canvasX: 0,
      canvasY: 0,
      width: 100,
      height: 100,
      strokes: initialStrokes,
    })
    await wait(50)

    // Resize to 200×200 (2× in both axes) with proportionally scaled strokes.
    await updateDrawingEntity(drawing.id, {
      width: 200,
      height: 200,
      strokes: [
        {
          id: 'stroke-1',
          color: '#ff0000',
          width: 3,
          points: [
            { x: 20, y: 20 },
            { x: 100, y: 100 },
            { x: 180, y: 20 },
          ],
        },
      ],
    })
    await wait(50)

    const { drawingEntities: afterResize } = await getDrawingEntities()
    const resized = afterResize.find((d) => d.id === drawing.id)
    expect(resized).toBeDefined()
    expect(resized!.width).toBe(200)
    expect(resized!.height).toBe(200)
    expect(resized!.strokes[0].points[0]).toMatchObject({ x: 20, y: 20 })

    await undoWorkspace()
    await wait(50)

    const { drawingEntities: afterUndo } = await getDrawingEntities()
    const restored = afterUndo.find((d) => d.id === drawing.id)
    expect(restored).toBeDefined()
    expect(restored!.width).toBe(100)
    expect(restored!.height).toBe(100)
    expect(restored!.strokes[0].points[0]).toMatchObject({ x: 10, y: 10 })
    expect(restored!.strokes[0].points[1]).toMatchObject({ x: 50, y: 50 })
  })
})
