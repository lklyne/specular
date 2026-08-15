/**
 * Free-ended edges (nullable `fromEntityId`/`toEntityId` + `fromPoint`/
 * `toPoint`) round-tripping through `specular.freeEdges`, the delete cascade
 * detaching rather than deleting when the other end survives, and re-binding
 * a free end moving the edge back into the spec's `edges[]`.
 *
 * Mutation-verified: dropping the `specular.freeEdges` split in
 * `serializeToJsonCanvas` (json-canvas-serializer.ts) fails the persistence
 * case; reverting `removeEdgesTouchingEntities` to unconditional delete
 * (workspace-edges.ts) fails the detach case; dropping the `fromPoint`/
 * `toPoint` clear in `updateEdge` fails the re-bind case.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { createTextEntity, deleteTextEntity, updateEdge } from '../../src/main/runtime/document-commands'
import { createEdges } from '../../src/main/workspace-edges'
import { workspaceEdges } from '../../src/main/runtime/space-model'
import { undo } from '../../src/main/runtime/space-undo'

let harness: WorkspaceHarness

describe('free-ended edges', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('persists a free-ended edge through specular.freeEdges, not edges[]', async () => {
    const a = createTextEntity({ canvasX: 0, canvasY: 0, text: 'A' })
    const { edgeIds } = createEdges({
      edges: [
        {
          fromEntityId: a.id,
          toEntityId: null,
          toPoint: { x: 500, y: 200 },
          kind: 'connection',
        },
      ],
    })
    const edgeId = edgeIds[0]
    await settleSync()

    const doc = harness.diskDoc()
    expect(doc?.edges.some((e) => e.id === edgeId)).toBe(false)
    const freeEdge = doc?.specular?.freeEdges?.find((e) => e.id === edgeId)
    expect(freeEdge).toBeDefined()
    expect(freeEdge?.fromNode).toBe(a.id)
    expect(freeEdge?.toNode).toBeUndefined()
    expect(freeEdge?.toPoint).toEqual({ x: 500, y: 200 })

    // A strict JSON Canvas reader still sees a fully valid file: no dangling
    // ids in edges[], and the free edge simply doesn't appear there.
    for (const e of doc?.edges ?? []) {
      expect(typeof e.fromNode).toBe('string')
      expect(typeof e.toNode).toBe('string')
    }
  })

  it('survives deletion of the entity at its bound end by detaching, not deleting', async () => {
    const a = createTextEntity({ canvasX: 0, canvasY: 0, text: 'A' })
    const b = createTextEntity({ canvasX: 400, canvasY: 0, text: 'B' })
    const { edgeIds } = createEdges({
      edges: [{ fromEntityId: a.id, toEntityId: b.id, kind: 'connection' }],
    })
    const edgeId = edgeIds[0]
    await settleSync()

    expect(deleteTextEntity(b.id)).toBe(true)
    await settleSync()

    const edge = workspaceEdges.find((e) => e.id === edgeId)
    expect(edge).toBeDefined()
    expect(edge?.fromEntityId).toBe(a.id)
    expect(edge?.toEntityId).toBeNull()
    expect(edge?.toPoint).toBeDefined()

    const doc = harness.diskDoc()
    expect(doc?.edges.some((e) => e.id === edgeId)).toBe(false)
    expect(doc?.specular?.freeEdges?.some((e) => e.id === edgeId)).toBe(true)
  })

  it('moves a re-bound free end back into edges[]', async () => {
    const a = createTextEntity({ canvasX: 0, canvasY: 0, text: 'A' })
    const b = createTextEntity({ canvasX: 400, canvasY: 0, text: 'B' })
    const { edgeIds } = createEdges({
      edges: [
        {
          fromEntityId: a.id,
          toEntityId: null,
          toPoint: { x: 500, y: 200 },
          kind: 'connection',
        },
      ],
    })
    const edgeId = edgeIds[0]
    await settleSync()
    expect(harness.diskDoc()?.specular?.freeEdges?.some((e) => e.id === edgeId)).toBe(true)

    expect(updateEdge(edgeId, { toEntityId: b.id })).toBe(true)
    await settleSync()

    const edge = workspaceEdges.find((e) => e.id === edgeId)
    expect(edge?.toEntityId).toBe(b.id)
    expect(edge?.toPoint).toBeUndefined()

    const doc = harness.diskDoc()
    expect(doc?.specular?.freeEdges?.some((e) => e.id === edgeId) ?? false).toBe(false)
    expect(doc?.edges.some((e) => e.id === edgeId)).toBe(true)
  })

  it('undoing an entity delete restores the bound end, not just the free point', async () => {
    const a = createTextEntity({ canvasX: 0, canvasY: 0, text: 'A' })
    const b = createTextEntity({ canvasX: 400, canvasY: 0, text: 'B' })
    const { edgeIds } = createEdges({
      edges: [{ fromEntityId: a.id, toEntityId: b.id, kind: 'connection' }],
    })
    const edgeId = edgeIds[0]
    await settleSync()

    expect(deleteTextEntity(b.id)).toBe(true)
    await settleSync()
    expect(workspaceEdges.find((e) => e.id === edgeId)?.toEntityId).toBeNull()

    undo()
    await settleSync()
    const edge = workspaceEdges.find((e) => e.id === edgeId)
    expect(edge?.toEntityId).toBe(b.id)
  })
})
