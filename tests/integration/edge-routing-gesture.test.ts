/**
 * The `routing-edge` gesture: dragging an elbow edge's crossbar. Move ticks
 * are broadcast-only, the commit is one Y.Doc transaction, and undo round-trips
 * cleanly back to the pre-drag route.
 *
 * Mutation-verified: writing the split per move tick (calling `updateEdge`
 * inside `moveEdgeRoutingGesture`) fails the transaction-count and cancel
 * cases; committing by mutating the runtime edge directly instead of through
 * `updateEdge` fails the transaction-count and undo cases.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { createTextEntity } from '../../src/main/runtime/document-commands'
import { createEdges } from '../../src/main/workspace-edges'
import { workspaceEdges } from '../../src/main/runtime/space-model'
import { undo } from '../../src/main/runtime/space-undo'
import { currentInteractionState } from '../../src/main/runtime/interaction-state'
import {
  cancelEdgeRoutingGesture,
  commitEdgeRoutingGesture,
  moveEdgeRoutingGesture,
  startEdgeRoutingGesture,
} from '../../src/main/edge-routing-gesture'

let harness: WorkspaceHarness

function seedElbowEdge(): string {
  const a = createTextEntity({ canvasX: 0, canvasY: 0, text: 'A' })
  const b = createTextEntity({ canvasX: 600, canvasY: 0, text: 'B' })
  const { edgeIds } = createEdges({
    edges: [
      { fromEntityId: a.id, toEntityId: b.id, routing: 'elbow', kind: 'connection' },
    ],
  })
  return edgeIds[0]
}

function edge(edgeId: string) {
  return workspaceEdges.find((e) => e.id === edgeId)
}

describe('routing-edge gesture', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('ticks the split on the broadcast only, then writes once at commit', async () => {
    const edgeId = seedElbowEdge()
    await settleSync()

    expect(startEdgeRoutingGesture(edgeId, 0.5, 'x')).toBe(true)
    moveEdgeRoutingGesture(0.7)
    moveEdgeRoutingGesture(0.85)
    const live = currentInteractionState()
    expect(live).toMatchObject({ kind: 'routing-edge', edgeId, split: 0.85, axis: 'x' })
    // Nothing written yet — the renderer previews from the broadcast.
    expect(edge(edgeId)?.elbowSplit).toBeUndefined()

    let transactions = 0
    const handler = () => (transactions += 1)
    harness.doc.on('afterTransaction', handler)
    try {
      commitEdgeRoutingGesture()
      await settleSync()
    } finally {
      harness.doc.off('afterTransaction', handler)
    }

    expect(transactions).toBe(1)
    expect(edge(edgeId)?.elbowSplit).toBe(0.85)
    expect(edge(edgeId)?.elbowSplitAxis).toBe('x')
    expect(currentInteractionState().kind).toBe('idle')
  })

  it('undo restores the pre-drag route', async () => {
    const edgeId = seedElbowEdge()
    await settleSync()

    startEdgeRoutingGesture(edgeId, 0.5, 'x')
    moveEdgeRoutingGesture(0.2)
    commitEdgeRoutingGesture()
    await settleSync()
    expect(edge(edgeId)?.elbowSplit).toBe(0.2)

    undo()
    await settleSync()
    expect(edge(edgeId)?.elbowSplit).toBeUndefined()
    expect(edge(edgeId)?.routing).toBe('elbow')
  })

  it('cancel leaves the edge untouched', async () => {
    const edgeId = seedElbowEdge()
    await settleSync()

    startEdgeRoutingGesture(edgeId, 0.5, 'x')
    moveEdgeRoutingGesture(0.9)
    cancelEdgeRoutingGesture('escape')
    await settleSync()

    expect(edge(edgeId)?.elbowSplit).toBeUndefined()
    expect(currentInteractionState().kind).toBe('idle')
  })

  it('persists the committed split through the .canvas round-trip', async () => {
    const edgeId = seedElbowEdge()
    startEdgeRoutingGesture(edgeId, 0.5, 'y')
    moveEdgeRoutingGesture(0.33)
    commitEdgeRoutingGesture()
    await settleSync()

    const stored = harness.diskDoc()?.edges.find((e) => e.id === edgeId)
    expect(stored?.elbowSplit).toBe(0.33)
    expect(stored?.elbowSplitAxis).toBe('y')
  })
})
