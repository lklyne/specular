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

  it('creates edges and deletes entities by id, resolving kind from the doc', async () => {
    const { created } = await applyCanvas({
      entities: [
        { kind: 'text', text: 'from', canvasX: 0, canvasY: 0 },
        { kind: 'text', text: 'to', canvasX: 200, canvasY: 0 },
      ],
    })
    const [from, to] = created

    const linked = await applyCanvas({
      edges: [{ fromEntityId: from, toEntityId: to, kind: 'connection' }],
    })
    expect(linked.edges).toHaveLength(1)
    const edges = (await getWorkspace()).edges as Array<{ id: string }>
    expect(edges.map((e) => e.id)).toContain(linked.edges[0])

    // Delete passes only the id — apply resolves the kind from the doc.
    const removed = await applyCanvas({ delete: [from] })
    expect(removed.deleted).toEqual([from])

    const ids = (await getTextEntities()).textEntities.map((e) => e.id)
    expect(ids).toContain(to)
    expect(ids).not.toContain(from)
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
