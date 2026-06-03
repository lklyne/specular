/**
 * Wireframe agent CLI route smoke tests (plan 3.4 — agent CLI parity).
 *
 * `specular wireframe <fileId|path> <verb> …` POSTs a WireframeOp to the
 * production `/wireframe/op` route. These tests drive that route directly (the
 * surface the CLI uses) and assert: each verb projects to `.wireframe.json`, the
 * edit is one undoable Y.Doc transaction (the shared 3.0b apply path, not a raw
 * disk write), a bad target / node id / illegal prop returns a legible 4xx, and
 * a create → duplicate round-trip yields the expected tree.
 *
 * Mutation-verified by:
 *   - replacing `commitWireframeOp(entityId, op)` in routes/wireframe.ts with a
 *     direct `writeNoteFile` and confirming the one-transaction + undo cases fail
 *     (the edit no longer enters the Y.Doc / undo stack).
 *   - deleting the `findWireframeOpError` check and confirming the unknown-node-id
 *     case stops returning 400 (the op silently no-ops to a 200 instead).
 */

import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import {
  createWireframeEntity,
  deleteFileEntities,
  flushWorkspaceAutosave,
  getUndoState,
  getWireframeContent,
  listFileEntities,
  resetSmokeState,
  undoWorkspace,
  wireframeOp,
} from './app-client'
import { observeYDocTransactions } from './test-utils'

function sampleContent(): string {
  return JSON.stringify(
    {
      version: '1.0',
      root: {
        id: 'root',
        type: 'frame',
        direction: 'vertical',
        children: [
          { id: 'title', type: 'text', text: 'Hello', level: 'h1' },
          { id: 'agree', type: 'checkbox', label: 'Agree', checked: false },
        ],
      },
    },
    null,
    2,
  )
}

async function cleanupFileEntities(): Promise<void> {
  const { fileEntities } = await listFileEntities()
  if (fileEntities.length) {
    await deleteFileEntities(fileEntities.map((e) => e.id))
  }
}

async function drainUndoStack(): Promise<void> {
  for (let i = 0; i < 200; i++) {
    const state = await getUndoState()
    if (!state.canUndo) return
    await undoWorkspace()
  }
}

describe('wireframe agent CLI route', () => {
  beforeEach(async () => {
    await resetSmokeState()
    await cleanupFileEntities()
    await drainUndoStack()
  })

  afterEach(async () => {
    await cleanupFileEntities()
  })

  it('insert: applies via the route and projects to disk', async () => {
    const { id } = await createWireframeEntity({ content: sampleContent() })
    const result = await wireframeOp(id, {
      kind: 'insert',
      parentId: 'root',
      index: 1,
      node: { id: 'mid', type: 'divider' },
    })
    expect(result.ok).toBe(true)
    expect(result.id).toBe(id)

    await flushWorkspaceAutosave()
    const after = await getWireframeContent(id)
    expect(after.disk).toBe(result.content)
    expect(after.disk).toContain('"id": "mid"')
  })

  it('set: changes a prop and projects to disk', async () => {
    const { id } = await createWireframeEntity({ content: sampleContent() })
    const result = await wireframeOp(id, {
      kind: 'setProps',
      nodeId: 'root',
      patch: { direction: 'horizontal' },
    })
    expect(result.ok).toBe(true)

    await flushWorkspaceAutosave()
    const after = await getWireframeContent(id)
    expect(after.disk).toContain('"direction": "horizontal"')
  })

  it('delete and reorder drive the route', async () => {
    const { id } = await createWireframeEntity({ content: sampleContent() })

    // reorder agree before title
    expect(
      (await wireframeOp(id, { kind: 'reorder', nodeId: 'agree', targetParentId: 'root', targetIndex: 0 })).ok,
    ).toBe(true)
    // delete title
    const del = await wireframeOp(id, { kind: 'delete', nodeId: 'title' })
    expect(del.ok).toBe(true)

    await flushWorkspaceAutosave()
    const after = await getWireframeContent(id)
    expect(after.disk).not.toContain('"id": "title"')
    expect(after.disk).toContain('"id": "agree"')
  })

  it('resolves the target by file path, not just id', async () => {
    const { id, file } = await createWireframeEntity({ content: sampleContent() })
    const result = await wireframeOp(file, { kind: 'setText', nodeId: 'title', value: 'ByPath' })
    expect(result.ok).toBe(true)
    expect(result.id).toBe(id)
    expect(result.content).toContain('"text": "ByPath"')
  })

  it('applies a route op as exactly one Y.Doc transaction (undoable like a canvas edit)', async () => {
    const original = sampleContent()
    const { id } = await createWireframeEntity({ content: original })
    const count = await observeYDocTransactions(async () => {
      const result = await wireframeOp(id, { kind: 'setText', nodeId: 'title', value: 'Once' })
      expect(result.ok).toBe(true)
    })
    expect(count).toBe(1)

    await undoWorkspace()
    const after = await flushWorkspaceAutosave().then(() => getWireframeContent(id))
    expect(after.runtime).toBe(original)
    expect(after.disk).toBe(original)
  })

  it('round-trips: create → duplicate → tree gains a clone of the node', async () => {
    const { id } = await createWireframeEntity({ content: sampleContent() })
    const result = await wireframeOp(id, { kind: 'duplicate', nodeId: 'title' })
    expect(result.ok).toBe(true)
    // The clone carries the same text but a fresh id — two "Hello" text nodes now.
    expect(result.content!.match(/"text": "Hello"/g)?.length).toBe(2)
    expect(result.content!.match(/"id": "title"/g)?.length).toBe(1)
  })

  it('rejects an unknown target with a 404', async () => {
    const result = await wireframeOp('file_does_not_exist', { kind: 'delete', nodeId: 'x' })
    expect(result.ok).toBe(false)
    expect(result.status).toBe(404)
    expect(result.error).toMatch(/No wireframe entity/)
  })

  it('rejects an unknown node id with a legible 400', async () => {
    const { id } = await createWireframeEntity({ content: sampleContent() })
    const result = await wireframeOp(id, { kind: 'delete', nodeId: 'nope' })
    expect(result.ok).toBe(false)
    expect(result.status).toBe(400)
    expect(result.error).toMatch(/Unknown node/)
  })

  it('rejects an illegal prop with a legible 400', async () => {
    const { id } = await createWireframeEntity({ content: sampleContent() })
    const result = await wireframeOp(id, {
      kind: 'setProps',
      nodeId: 'agree',
      patch: { variant: 'primary' },
    })
    expect(result.ok).toBe(false)
    expect(result.status).toBe(400)
    expect(result.error).toMatch(/not valid for node type/)
  })
})
