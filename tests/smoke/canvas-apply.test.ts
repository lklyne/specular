import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import {
  applyCanvas,
  getCanvas,
  getTextEntities,
  getWorkspace,
  resetSmokeState,
} from './app-client'

/**
 * POST /canvas/apply is the single mutation door (ADR 0019): one patch —
 * create (no id) / update (id) / delete (id) / edges — applied in one
 * transaction. GET /canvas is the JSON Canvas read shape `specular workspace`
 * reads. These cover the spine every CLI verb now compiles to.
 *
 * Assertions are id-scoped: reset-state clears selection/presence but not
 * entities, so tests track only the ids they create.
 *
 * Mutation-verified by (a) making the apply route's delete loop a no-op —
 * the delete assertions then fail; (b) skipping the edges branch — the edge
 * assertion fails; (c) having GET /canvas return getWorkspaceGraph() instead
 * of the serialized doc — the `type: 'text'` node-shape lookup returns nothing.
 */
describe('canvas apply + get', () => {
  beforeEach(async () => {
    await resetSmokeState()
  })

  afterAll(async () => {
    await resetSmokeState()
  })

  it('creates entities and exposes them via GET /canvas', async () => {
    const { created } = await applyCanvas({
      entities: [
        { kind: 'text', text: 'alpha', canvasX: 0, canvasY: 0 },
        { kind: 'text', text: 'beta', canvasX: 200, canvasY: 0 },
      ],
    })
    expect(created).toHaveLength(2)

    const doc = await getCanvas()
    const byId = new Map(doc.nodes.map((n) => [n.id, n]))
    const alpha = byId.get(created[0]) as { type: string; text: string } | undefined
    const beta = byId.get(created[1]) as { type: string; text: string } | undefined
    expect(alpha?.type).toBe('text')
    expect(alpha?.text).toBe('alpha')
    expect(beta?.text).toBe('beta')
  })

  it('updates an entity in place', async () => {
    const { created } = await applyCanvas({
      entities: [{ kind: 'text', text: 'before', canvasX: 0, canvasY: 0 }],
    })
    const id = created[0]

    const { updated } = await applyCanvas({ entities: [{ id, kind: 'text', text: 'after' }] })
    expect(updated).toEqual([id])

    const entity = (await getTextEntities()).textEntities.find((e) => e.id === id)
    expect(entity?.text).toBe('after')
  })

  it('updates an entity passing only its id — kind resolves from the doc', async () => {
    // The `update` verb no longer sniffs an id prefix for kind (ADR 0019 §4);
    // it sends just { id, …fields } and apply resolves the kind from the doc.
    const { created } = await applyCanvas({
      entities: [{ kind: 'text', text: 'before', canvasX: 0, canvasY: 0 }],
    })
    const id = created[0]

    const { updated } = await applyCanvas({ entities: [{ id, text: 'after' }] })
    expect(updated).toEqual([id])

    const entity = (await getTextEntities()).textEntities.find((e) => e.id === id)
    expect(entity?.text).toBe('after')
  })

  it('reports nothing deleted for an id the doc does not know (no lie, no crash)', async () => {
    // The former kindFromId bucketed unknown prefixes to `file` and crashed on
    // bare string arrays; apply resolves kind from the doc, so an unknown id is
    // simply not deleted rather than mis-routed.
    const removed = await applyCanvas({ delete: ['text_does_not_exist'] })
    expect(removed.deleted).toEqual([])
  })

  it('creates edges and deletes entities by id, resolving kind from the doc', async () => {
    const { created } = await applyCanvas({
      entities: [
        { kind: 'text', text: 'from', canvasX: 0, canvasY: 0 },
        { kind: 'text', text: 'to', canvasX: 200, canvasY: 0 },
      ],
    })
    const [from, to] = created

    const linked = await applyCanvas({
      edges: [{ fromEntityId: from, toEntityId: to, kind: 'connection', label: 'digest of' }],
    })
    expect(linked.edges).toHaveLength(1)
    const edges = (await getWorkspace()).edges as Array<{ id: string }>
    expect(edges.map((e) => e.id)).toContain(linked.edges[0])
    // label must survive createEdges — it was dropped there before (read full
    // canvas, since getWorkspace() projects edges down to ids).
    const canvasEdge = (await getCanvas()).edges.find((e) => e.id === linked.edges[0])
    expect(canvasEdge?.label).toBe('digest of')

    // Delete passes only the id — apply resolves the kind from the doc.
    const removed = await applyCanvas({ delete: [from] })
    expect(removed.deleted).toEqual([from])

    const ids = (await getTextEntities()).textEntities.map((e) => e.id)
    expect(ids).toContain(to)
    expect(ids).not.toContain(from)
  })

  it('creates every entity kind via apply — incl. drawing/shape, which had no create path before', async () => {
    // The `add` verb (ADR 0019 §1) and direct `apply` both compile to this
    // spine. Drawing and shape never had a create route; the registry handlers
    // give them one. group is created around existing ids.
    const seed = await applyCanvas({
      entities: [
        { kind: 'text', forceKind: true, text: 'a', canvasX: 0, canvasY: 0 },
        { kind: 'text', forceKind: true, text: 'b', canvasX: 200, canvasY: 0 },
      ],
    })
    const [a, b] = seed.created

    const { created } = await applyCanvas({
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

    const byId = new Map((await getCanvas()).nodes.map((n) => [n.id, n.type]))
    expect(byId.get(pageId)).toBe('link')
    expect(byId.get(shapeId)).toBe('shape')
    expect(byId.get(drawingId)).toBe('drawing')
    expect(byId.get(groupId)).toBe('group')
  })

  it('accepts kind:"note" as an alias for text — same vocabulary as `add note`', async () => {
    // `add note` is verb sugar that compiles to kind:text; the patch door now
    // speaks the same word. Short note → text; the file kind's claimsAsNote
    // route still decides text-vs-file by content, not by the alias.
    const { created } = await applyCanvas({
      entities: [{ kind: 'note', text: 'short note', canvasX: 0, canvasY: 0 }],
    })
    expect(created).toHaveLength(1)
    const node = (await getCanvas()).nodes.find((n) => n.id === created[0])
    expect(node?.type).toBe('text')
  })

  it('applies create + delete as one patch', async () => {
    const seed = await applyCanvas({
      entities: [{ kind: 'text', text: 'seed', canvasX: 0, canvasY: 0 }],
    })
    const seedId = seed.created[0]

    const result = await applyCanvas({
      entities: [
        { kind: 'text', text: 'one', canvasX: 0, canvasY: 200 },
        { kind: 'text', text: 'two', canvasX: 200, canvasY: 200 },
      ],
      delete: [seedId],
    })
    expect(result.created).toHaveLength(2)
    expect(result.deleted).toEqual([seedId])

    const ids = (await getTextEntities()).textEntities.map((e) => e.id)
    expect(ids).toEqual(expect.arrayContaining(result.created))
    expect(ids).not.toContain(seedId)
  })
})
