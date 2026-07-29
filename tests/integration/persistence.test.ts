/**
 * Persistence integration tests.
 *
 * Covers the autosave + .canvas-file path in src/main/runtime/workspace-*.ts,
 * driven in-process: mutations go through the same document-commands the IPC
 * handlers call, assertions read the file the renderer round-trips from on
 * next launch. Also guards the load-time migrations (group stack-order
 * normalization, legacy Browser-mode appState restore).
 *
 * Mutation-verified by: commenting out `scheduleWorkspaceAutosave()` in
 * `createTextEntity` (src/main/runtime/document-commands.ts) — "autosave
 * writes a mutation to the .canvas file on disk" fails because the disk
 * snapshot stays empty after the debounce window.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import {
  createTextEntity,
  deleteTextEntity,
  getTextEntities,
} from '../../src/main/runtime/document-commands'
import { currentEntityOrder } from '../../src/main/runtime/entity-order-state'
import { getSelectionState } from '../../src/main/workspace-entities'
import {
  DEFAULT_WORKSPACE_ID,
  readCanvasFile,
} from '../../src/main/runtime/workspace-persistence'
import type { JsonCanvasDocument } from '../../src/shared/json-canvas-types'

let harness: WorkspaceHarness

function diskTextIds(doc: JsonCanvasDocument | null): Set<string> {
  return new Set((doc?.nodes ?? []).filter((n) => n.type === 'text').map((n) => n.id))
}

describe('persistence', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('autosave writes a mutation to the .canvas file on disk', async () => {
    // The one intentionally-sleeping test: it verifies the real 350ms
    // debounce fires on its own, so it must NOT flush.
    const path = harness.diskPath('Blank')
    const beforeIds = diskTextIds(readCanvasFile(path))

    const entity = createTextEntity({ canvasX: 120, canvasY: 240, text: 'persisted text' })
    await settleSync()

    // Wait past the 350ms debounce — no explicit flush.
    await new Promise((resolve) => setTimeout(resolve, 500))
    const afterIds = diskTextIds(readCanvasFile(path))
    expect(afterIds.has(entity.id)).toBe(true)
    expect(beforeIds.has(entity.id)).toBe(false)
  })

  it('flush triggers an immediate write (no debounce wait needed)', async () => {
    const entity = createTextEntity({ canvasX: 5, canvasY: 5, text: 'flush me' })
    await settleSync()

    expect(diskTextIds(harness.diskDoc()).has(entity.id)).toBe(true)
  })

  it('in-memory state matches on-disk snapshot after flush', async () => {
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'one' })
    createTextEntity({ canvasX: 200, canvasY: 0, text: 'two' })
    await settleSync()

    const disk = diskTextIds(harness.diskDoc())
    const runtimeIds = new Set(getTextEntities().map((t) => t.id))

    expect(disk.size).toBe(runtimeIds.size)
    for (const id of runtimeIds) {
      expect(disk.has(id)).toBe(true)
    }
  })

  it('deletion removes the entity from the .canvas file after flush', async () => {
    const entity = createTextEntity({ canvasX: 10, canvasY: 10, text: 'temporary' })
    await settleSync()
    expect(diskTextIds(harness.diskDoc()).has(entity.id)).toBe(true)

    deleteTextEntity(entity.id)
    await settleSync()
    expect(diskTextIds(harness.diskDoc()).has(entity.id)).toBe(false)
  })

  it('normalizes scattered group stack order on load and autosaves the migration', async () => {
    const doc: JsonCanvasDocument = {
      nodes: [
        { id: 'a', type: 'shape', shapeKind: 'rectangle', x: 0, y: 0, width: 100, height: 100, parentGroupId: 'group' },
        { id: 'x', type: 'shape', shapeKind: 'rectangle', x: 150, y: 0, width: 100, height: 100 },
        { id: 'group', type: 'group', x: 0, y: 0, width: 300, height: 160, label: 'Group' },
        { id: 'b', type: 'shape', shapeKind: 'rectangle', x: 20, y: 20, width: 100, height: 100, parentGroupId: 'group' },
        { id: 'y', type: 'shape', shapeKind: 'rectangle', x: 300, y: 0, width: 100, height: 100 },
      ],
      edges: [],
      specular: { entityOrder: ['a', 'x', 'group', 'b', 'y'] },
      appState: { zoom: 1, pan: { x: 0, y: 0 } },
    }

    harness.loadFixture({ name: 'Stack Migration', doc })
    expect(currentEntityOrder()).toEqual(['x', 'a', 'b', 'group', 'y'])

    const persisted = harness.diskDoc('Stack Migration')
    const order = (persisted?.specular as { entityOrder?: string[] } | undefined)?.entityOrder
    expect(order).toEqual(['x', 'a', 'b', 'group', 'y'])
  })

  it('opens legacy Browser-mode appState as canvas with the saved page selected', () => {
    harness.loadFixture({
      name: 'Legacy Browser Selection',
      doc: {
        nodes: [
          {
            id: 'legacy-page',
            type: 'link',
            x: 120,
            y: 80,
            width: 800,
            height: 600,
            url: 'data:text/html,legacy',
          },
        ],
        edges: [],
        appState: {
          zoom: 1,
          pan: { x: 0, y: 0 },
          selectedEntityIds: ['legacy-page'],
          browserTabMode: 'page',
        },
      },
    })

    const selection = getSelectionState()
    expect(selection.selectedEntityId).toBe('legacy-page')
    expect(selection.selectedEntityIds).toEqual(['legacy-page'])
  })
})
