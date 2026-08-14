/**
 * Edge routing fields (`routing`, `elbowSplit`, `elbowSplitAxis`) surviving the
 * runtime → Y.Doc → `.canvas` path, and undo restoring the prior values.
 *
 * Mutation-verified by dropping `routing`/`elbowSplit`/`elbowSplitAxis` from
 * serializeEdge in json-canvas-serializer.ts (the disk cases fail) and from the
 * patch list in updateEdge (document-commands.ts) — the update and undo cases
 * fail.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { createTextEntity, updateEdge } from '../../src/main/runtime/document-commands'
import { createEdges } from '../../src/main/workspace-edges'
import { workspaceEdges } from '../../src/main/runtime/space-model'
import { undo } from '../../src/main/runtime/space-undo'
import { DOC_MAP_EDGES } from '../../src/main/runtime/space-doc'

let harness: WorkspaceHarness

function makeEdge(extra: Record<string, unknown> = {}): string {
  const a = createTextEntity({ canvasX: 0, canvasY: 0, text: 'A' })
  const b = createTextEntity({ canvasX: 400, canvasY: 0, text: 'B' })
  const { edgeIds } = createEdges({
    edges: [{ fromEntityId: a.id, toEntityId: b.id, kind: 'connection', ...extra }],
  })
  return edgeIds[0]
}

describe('edge routing fields', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('persists through the Y.Doc and the .canvas file', async () => {
    const edgeId = makeEdge({ routing: 'elbow', elbowSplit: 0.85, elbowSplitAxis: 'x' })
    await settleSync()

    const yEdge = harness.doc.getMap(DOC_MAP_EDGES).get(edgeId) as
      | { get(key: string): unknown }
      | undefined
    expect(yEdge?.get('routing')).toBe('elbow')
    expect(yEdge?.get('elbowSplit')).toBe(0.85)
    expect(yEdge?.get('elbowSplitAxis')).toBe('x')

    const onDisk = harness.diskDoc()?.edges.find((e) => e.id === edgeId)
    expect(onDisk?.routing).toBe('elbow')
    expect(onDisk?.elbowSplit).toBe(0.85)
    expect(onDisk?.elbowSplitAxis).toBe('x')
  })

  it('round-trips an update through undo', async () => {
    const edgeId = makeEdge({ routing: 'elbow', elbowSplit: 0.5, elbowSplitAxis: 'x' })
    await settleSync()

    expect(updateEdge(edgeId, { elbowSplit: 0.2, elbowSplitAxis: 'y', routing: 'straight' })).toBe(
      true,
    )
    await settleSync()
    expect(harness.diskDoc()?.edges.find((e) => e.id === edgeId)?.elbowSplit).toBe(0.2)

    undo()
    await settleSync()
    const edge = workspaceEdges.find((e) => e.id === edgeId)
    expect(edge?.routing).toBe('elbow')
    expect(edge?.elbowSplit).toBe(0.5)
    expect(edge?.elbowSplitAxis).toBe('x')
    expect(harness.diskDoc()?.edges.find((e) => e.id === edgeId)?.elbowSplit).toBe(0.5)
  })
})
