/**
 * `applyCanvasPatch` — the single canvas mutation door (ADR 0019) — driven
 * in-process. This is the exact function POST /canvas/apply calls, and the
 * read side uses the same `serializeToJsonCanvas(workspaceSnapshot())` GET
 * /canvas serves, so the tested path IS the shipping path.
 *
 * Guards: create/update/delete dispatch with kind resolved from the doc
 * (never an id prefix), edge creation with label survival, kind:"note"
 * aliasing, every-kind create coverage (page/shape/drawing/group), atomic
 * validation (throws before mutating), and the one-patch-one-undo-step
 * contract of `commitAsOneTransaction`.
 *
 * Mutation-verified by (all in src/main/canvas-apply.ts):
 *   - moving `commitAsOneTransaction` inside the entities loop (one commit
 *     per item) — "a multi-item patch collapses to one undo step" fails.
 *     Note: merely deleting the wrapper is NOT caught, because the deferred
 *     microtask diff-sync still coalesces the patch into one transaction;
 *     the wrapper guards against per-item commits, which is what this
 *     mutation simulates.
 *   - making `resolveKind` return the caller's `item.kind` for updates
 *     instead of `entityKindById` — "updates an entity passing only its id"
 *     fails (CanvasPatchError).
 *   - making the delete loop iterate nothing — "creates edges and deletes
 *     entities by id" and "applies create + delete as one patch" fail.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { applyCanvasPatch, CanvasPatchError } from '../../src/main/canvas-apply'
import { serializeToJsonCanvas } from '../../src/main/runtime/json-canvas-serializer'
import { workspaceSnapshot } from '../../src/main/runtime/workspace-tabs'
import { getTextEntities } from '../../src/main/runtime/document-commands'
import { workspaceEdges } from '../../src/main/runtime/workspace-model'
import { undo } from '../../src/main/runtime/workspace-undo'
import { DOC_MAP_ENTITIES } from '../../src/main/runtime/workspace-doc'

/** The same read shape GET /canvas serves. */
function getCanvas() {
  return serializeToJsonCanvas(workspaceSnapshot())
}

let harness: WorkspaceHarness

describe('canvas apply', () => {
  beforeEach(() => {
    // Same boot step src/main/index.ts runs after app.whenReady (idempotent).
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('creates entities and exposes them via the workspace snapshot', () => {
    const { created } = applyCanvasPatch({
      entities: [
        { kind: 'text', text: 'alpha', canvasX: 0, canvasY: 0 },
        { kind: 'text', text: 'beta', canvasX: 200, canvasY: 0 },
      ],
    })
    expect(created).toHaveLength(2)

    const byId = new Map(getCanvas().nodes.map((n) => [n.id, n]))
    const alpha = byId.get(created[0]) as { type: string; text: string } | undefined
    const beta = byId.get(created[1]) as { type: string; text: string } | undefined
    expect(alpha?.type).toBe('text')
    expect(alpha?.text).toBe('alpha')
    expect(beta?.text).toBe('beta')
  })

  it('updates an entity in place', () => {
    const { created } = applyCanvasPatch({
      entities: [{ kind: 'text', text: 'before', canvasX: 0, canvasY: 0 }],
    })
    const id = created[0]

    const { updated } = applyCanvasPatch({ entities: [{ id, kind: 'text', text: 'after' }] })
    expect(updated).toEqual([id])
    expect(getTextEntities().find((e) => e.id === id)?.text).toBe('after')
  })

  it('updates an entity passing only its id — kind resolves from the doc', () => {
    const { created } = applyCanvasPatch({
      entities: [{ kind: 'text', text: 'before', canvasX: 0, canvasY: 0 }],
    })
    const id = created[0]

    const { updated } = applyCanvasPatch({ entities: [{ id, text: 'after' }] })
    expect(updated).toEqual([id])
    expect(getTextEntities().find((e) => e.id === id)?.text).toBe('after')
  })

  it('reports nothing deleted for an id the doc does not know (no lie, no crash)', () => {
    const removed = applyCanvasPatch({ delete: ['text_does_not_exist'] })
    expect(removed.deleted).toEqual([])
  })

  it('creates edges and deletes entities by id, resolving kind from the doc', () => {
    const { created } = applyCanvasPatch({
      entities: [
        { kind: 'text', text: 'from', canvasX: 0, canvasY: 0 },
        { kind: 'text', text: 'to', canvasX: 200, canvasY: 0 },
      ],
    })
    const [from, to] = created

    const linked = applyCanvasPatch({
      edges: [{ fromEntityId: from, toEntityId: to, kind: 'connection', label: 'digest of' }],
    })
    expect(linked.edges).toHaveLength(1)
    expect(workspaceEdges.map((e) => e.id)).toContain(linked.edges[0])
    // label must survive createEdges.
    const canvasEdge = getCanvas().edges.find((e) => e.id === linked.edges[0])
    expect(canvasEdge?.label).toBe('digest of')

    const removed = applyCanvasPatch({ delete: [from] })
    expect(removed.deleted).toEqual([from])

    const ids = getTextEntities().map((e) => e.id)
    expect(ids).toContain(to)
    expect(ids).not.toContain(from)
  })

  it('patches an edge in place when the apply patch reuses its id, instead of duplicating it', async () => {
    // Bug #295: edge items were unconditionally routed to creation, so an
    // apply patch carrying an existing edge id appended a second record
    // instead of updating the first.
    const { created } = applyCanvasPatch({
      entities: [
        { kind: 'text', text: 'from', canvasX: 0, canvasY: 0 },
        { kind: 'text', text: 'to', canvasX: 200, canvasY: 0 },
      ],
    })
    const [from, to] = created

    const linked = applyCanvasPatch({
      edges: [{ fromEntityId: from, toEntityId: to, kind: 'connection', label: 'v1' }],
    })
    const edgeId = linked.edges[0]
    await settleSync()

    const relabeled = applyCanvasPatch({ edges: [{ id: edgeId, label: 'v2' }] })
    expect(relabeled.edges).toEqual([edgeId])
    await settleSync()

    const matching = getCanvas().edges.filter((e) => e.id === edgeId)
    expect(matching).toHaveLength(1)
    expect(matching[0].label).toBe('v2')

    undo()
    const revertedMatching = getCanvas().edges.filter((e) => e.id === edgeId)
    expect(revertedMatching).toHaveLength(1)
    expect(revertedMatching[0].label).toBe('v1')
  })

  it('creates every entity kind via apply — incl. drawing/shape', () => {
    const seed = applyCanvasPatch({
      entities: [
        { kind: 'text', forceKind: true, text: 'a', canvasX: 0, canvasY: 0 },
        { kind: 'text', forceKind: true, text: 'b', canvasX: 200, canvasY: 0 },
      ],
    })
    const [a, b] = seed.created

    const { created } = applyCanvasPatch({
      entities: [
        { kind: 'page', url: 'https://example.com', presetIndex: 9, canvasX: 0, canvasY: 400 },
        { kind: 'shape', shapeKind: 'rectangle', text: 'box', canvasX: 400, canvasY: 400 },
        {
          kind: 'drawing',
          canvasX: 800,
          canvasY: 400,
          width: 100,
          height: 100,
          strokes: [{ id: 's1', color: '#000', width: 2, points: [{ x: 0, y: 0 }, { x: 50, y: 50 }] }],
        },
        { kind: 'group', entityIds: [a, b], label: 'pair' },
      ],
    })
    expect(created).toHaveLength(4)
    const [pageId, shapeId, drawingId, groupId] = created

    const byId = new Map(getCanvas().nodes.map((n) => [n.id, n.type]))
    expect(byId.get(pageId)).toBe('link')
    expect(byId.get(shapeId)).toBe('shape')
    expect(byId.get(drawingId)).toBe('drawing')
    expect(byId.get(groupId)).toBe('group')
  })

  it('round-trips shape border style/color/width and undo restores the prior border', async () => {
    const { created } = applyCanvasPatch({
      entities: [
        {
          kind: 'shape',
          shapeKind: 'rectangle',
          canvasX: 0,
          canvasY: 0,
          borderStyle: 'solid',
          borderColor: '4',
          strokeWidth: 3,
        },
      ],
    })
    const id = created[0]
    await settleSync()

    // Create carries the border fields through persist → serialize (the GET
    // /canvas shape node).
    const seeded = getCanvas().nodes.find((n) => n.id === id) as
      | { borderStyle?: string; borderColor?: string; strokeWidth?: number }
      | undefined
    expect(seeded?.borderStyle).toBe('solid')
    expect(seeded?.borderColor).toBe('4')
    expect(seeded?.strokeWidth).toBe(3)

    applyCanvasPatch({
      entities: [{ id, kind: 'shape', borderStyle: 'none', borderColor: '1', strokeWidth: 1 }],
    })
    await settleSync()
    const updated = getCanvas().nodes.find((n) => n.id === id) as
      | { borderStyle?: string; borderColor?: string; strokeWidth?: number }
      | undefined
    expect(updated?.borderStyle).toBe('none')
    expect(updated?.borderColor).toBe('1')
    expect(updated?.strokeWidth).toBe(1)

    undo()
    const reverted = getCanvas().nodes.find((n) => n.id === id) as
      | { borderStyle?: string; borderColor?: string; strokeWidth?: number }
      | undefined
    expect(reverted?.borderStyle).toBe('solid')
    expect(reverted?.borderColor).toBe('4')
    expect(reverted?.strokeWidth).toBe(3)
  })

  it('renames a group via text, aliased to label; explicit label wins; undo restores the prior name', async () => {
    const seed = applyCanvasPatch({
      entities: [
        { kind: 'text', forceKind: true, text: 'a', canvasX: 0, canvasY: 0 },
        { kind: 'text', forceKind: true, text: 'b', canvasX: 200, canvasY: 0 },
      ],
    })
    const [a, b] = seed.created
    const { created } = applyCanvasPatch({
      entities: [{ kind: 'group', entityIds: [a, b], label: 'original' }],
    })
    const groupId = created[0]
    await settleSync()

    const { updated } = applyCanvasPatch({ entities: [{ id: groupId, kind: 'group', text: 'renamed' }] })
    expect(updated).toEqual([groupId])
    await settleSync()
    const renamed = getCanvas().nodes.find((n) => n.id === groupId) as { label?: string } | undefined
    expect(renamed?.label).toBe('renamed')

    undo()
    const reverted = getCanvas().nodes.find((n) => n.id === groupId) as { label?: string } | undefined
    expect(reverted?.label).toBe('original')

    applyCanvasPatch({ entities: [{ id: groupId, kind: 'group', text: 'ignored', label: 'explicit' }] })
    const explicit = getCanvas().nodes.find((n) => n.id === groupId) as { label?: string } | undefined
    expect(explicit?.label).toBe('explicit')
  })

  it('accepts kind:"note" as an alias for text', () => {
    const { created } = applyCanvasPatch({
      entities: [{ kind: 'note', text: 'short note', canvasX: 0, canvasY: 0 }],
    })
    expect(created).toHaveLength(1)
    const node = getCanvas().nodes.find((n) => n.id === created[0])
    expect(node?.type).toBe('text')
  })

  it('applies create + delete as one patch', () => {
    const seed = applyCanvasPatch({
      entities: [{ kind: 'text', text: 'seed', canvasX: 0, canvasY: 0 }],
    })
    const seedId = seed.created[0]

    const result = applyCanvasPatch({
      entities: [
        { kind: 'text', text: 'one', canvasX: 0, canvasY: 200 },
        { kind: 'text', text: 'two', canvasX: 200, canvasY: 200 },
      ],
      delete: [seedId],
    })
    expect(result.created).toHaveLength(2)
    expect(result.deleted).toEqual([seedId])

    const ids = getTextEntities().map((e) => e.id)
    expect(ids).toEqual(expect.arrayContaining(result.created))
    expect(ids).not.toContain(seedId)
  })

  it('throws CanvasPatchError before mutating anything on a bad item', () => {
    expect(() =>
      applyCanvasPatch({
        entities: [
          { kind: 'text', text: 'good', canvasX: 0, canvasY: 0 },
          { kind: 'not-a-kind', canvasX: 200, canvasY: 0 },
        ],
      }),
    ).toThrow(CanvasPatchError)

    // The good item must not have leaked through — validation runs first.
    expect(getTextEntities()).toHaveLength(0)
    expect(getCanvas().nodes).toHaveLength(0)
  })

  it('a multi-item patch collapses to one undo step', async () => {
    const { created } = applyCanvasPatch({
      entities: [
        { kind: 'text', text: 'one', canvasX: 0, canvasY: 0 },
        { kind: 'text', text: 'two', canvasX: 200, canvasY: 0 },
        { kind: 'text', text: 'three', canvasX: 400, canvasY: 0 },
      ],
    })
    expect(created).toHaveLength(3)
    await settleSync()

    undo()

    expect(getTextEntities()).toHaveLength(0)
    const entitiesMap = harness.doc.getMap(DOC_MAP_ENTITIES)
    for (const id of created) expect(entitiesMap.has(id)).toBe(false)
  })
})
