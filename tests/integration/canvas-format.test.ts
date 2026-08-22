/**
 * .canvas file format golden test.
 *
 * Builds a rich workspace through the single canvas mutation door
 * (`applyCanvasPatch`, ADR 0019) — one of each patchable entity kind (text,
 * shape, drawing, file) plus a group and an edge — flushes, and snapshots the
 * raw .canvas file text. Any drift in the persisted JSON Canvas format
 * (field renames, ordering changes, new extension keys) shows up as a
 * reviewable diff on __snapshots__/rich-workspace.canvas.
 *
 * The patch door treats an item WITH an `id` as an update, so create ids are
 * always minted by the runtime; they are remapped to stable tokens in
 * insertion order before snapshotting (edge ids ARE injectable and stay
 * fixed).
 *
 * Mutation-verified by: renaming a serializer field in
 * src/main/runtime/json-canvas-serializer.ts (e.g. emitting `shapeType`
 * instead of `shapeKind` in `serializeShapeToShapeNode`) — the file snapshot
 * comparison fails with the drift as the diff.
 */

import { readFileSync } from 'fs'
import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { applyCanvasPatch } from '../../src/main/canvas-apply'

let harness: WorkspaceHarness

/** Remap runtime-minted ids to stable tokens, in insertion order. */
function normalizeGeneratedIds(text: string, generated: string[]): string {
  let out = text
  generated.forEach((id, index) => {
    out = out.split(id).join(`generated-id-${index + 1}`)
  })
  return out
}

describe('.canvas format', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('a rich workspace persists to the golden .canvas snapshot', async () => {
    const creates = applyCanvasPatch({
      entities: [
        { kind: 'text', canvasX: 0, canvasY: 0, width: 200, height: 100, text: 'hello canvas' },
        { kind: 'shape', canvasX: 300, canvasY: 0, width: 120, height: 80, shapeKind: 'rectangle', text: 'box' },
        {
          kind: 'drawing',
          canvasX: 0,
          canvasY: 300,
          width: 150,
          height: 150,
          strokes: [
            { id: 'stroke-1', color: '#ff0000', width: 2, points: [{ x: 0, y: 0 }, { x: 10, y: 10 }, { x: 20, y: 5 }] },
          ],
        },
        // Explicit width/height skips on-disk probing, so no real file needed.
        { kind: 'file', canvasX: 300, canvasY: 300, file: 'notes/fixture.md', width: 220, height: 180 },
      ],
    })
    expect(creates.created).toHaveLength(4)
    const [textId, shapeId, drawingId, fileId] = creates.created

    // Group the text + file entities. Membership persists as parentGroupId
    // on each member node (all kinds), plus the group node's geometry and
    // the normalized entityOrder; the snapshot pins that.
    const linked = applyCanvasPatch({
      entities: [{ kind: 'group', entityIds: [textId, fileId], label: 'Pair' }],
      edges: [
        { id: 'edge-1', fromEntityId: shapeId, toEntityId: drawingId, fromSide: 'right', toSide: 'left', kind: 'connection' },
      ],
    })
    expect(linked.created).toHaveLength(1)
    expect(linked.edges).toEqual(['edge-1'])
    const groupId = linked.created[0]

    await settleSync()
    harness.flush()

    const raw = readFileSync(
      harness.diskPath('Blank'),
      'utf8',
    )

    // The file is valid JSON Canvas with the specular extension block.
    const parsed = JSON.parse(raw) as {
      nodes?: Array<{ id: string; type: string }>
      edges?: unknown[]
      specular?: Record<string, unknown>
    }
    expect(Array.isArray(parsed.nodes)).toBe(true)
    expect(parsed.nodes?.map((n) => n.type).sort()).toEqual([
      'drawing',
      'file',
      'group',
      'shape',
      'text',
    ])
    expect(parsed.edges).toHaveLength(1)
    expect(parsed.specular).toBeTypeOf('object')

    await expect(
      normalizeGeneratedIds(raw, [textId, shapeId, drawingId, fileId, groupId]),
    ).toMatchFileSnapshot('__snapshots__/rich-workspace.canvas')
  })
})
