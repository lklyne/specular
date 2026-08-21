/**
 * Background-tab writes (issue #360, phase 3) against the real runtime.
 *
 * `withTabContext` swaps a non-active tab into the runtime arrays, runs the
 * normal `applyCanvasPatch` spine, folds the result back into that tab's
 * record and `.canvas` file, and puts the user's canvas back — without the
 * user noticing. These tests guard each half of that promise: the write lands
 * on the target (snapshot AND disk), and nothing about the active tab moves
 * (runtime arrays, its `.canvas` bytes, its undo stack). Pages are refused
 * because the swap carries them as data and never mints a WebContentsView.
 *
 * Mutation-verified by: (a) dropping the `writeTabAsCanvasFile` call in
 * `withTabContext` — four cases fail on their disk assertions; (b) dropping
 * `setActiveDoc(detached)` from the swap so the write lands in the user's
 * doc — "does not touch the user's undo stack" fails with an extra stack
 * item; (c) deleting the `rejectPageWorkOnBackgroundTab` call in
 * `applyCanvasPatch` — both page cases fail (no throw, and a live page is
 * minted onto a canvas nobody is looking at); (d) making `readCanvasDocument`
 * ignore its `targetTab` argument — "reads a background tab without swapping
 * or activating" fails, it answers with the active canvas.
 */

import { readFileSync } from 'fs'
import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { applyCanvasPatch } from '../../src/main/canvas-apply'
import { readCanvasDocument } from '../../src/main/routes/canvas'
import { withTabContext } from '../../src/main/runtime/space-tab-context'
import {
  createBackgroundSpaceTab,
  setActiveSpaceTab,
} from '../../src/main/runtime/space-tab-operations'
import { activeSpaceTabId, spaceTabs } from '../../src/main/runtime/space-model'
import { createTextEntity } from '../../src/main/runtime/document-commands'
import { textEntities } from '../../src/main/runtime/text-entity-state'
import { pages } from '../../src/main/runtime/runtime-context'
import { getActiveDoc } from '../../src/main/runtime/space-doc'
import { getActiveUndoManager, undo } from '../../src/main/runtime/space-undo'
import { readCanvasFile } from '../../src/main/runtime/space-persistence'
import type { PersistedWorkspaceTab } from '../../src/shared/types'
import type { JsonCanvasDocument } from '../../src/shared/json-canvas-types'

let harness: WorkspaceHarness

function newBackgroundTab(name: string): PersistedWorkspaceTab {
  const created = createBackgroundSpaceTab(name)
  if (!created.ok) throw new Error(created.error)
  return spaceTabs.find((tab) => tab.id === created.id)!
}

function tabFilePath(tab: string | PersistedWorkspaceTab): string {
  return harness.diskPath(tab)
}

function textsOnDisk(tab: string | PersistedWorkspaceTab): string[] {
  const doc = readCanvasFile(tabFilePath(tab))
  return nodeTexts(doc)
}

function nodeTexts(doc: JsonCanvasDocument | null): string[] {
  return (doc?.nodes ?? [])
    .filter((node) => node.type === 'text')
    .map((node) => (node as { text?: string }).text ?? '')
}

function snapshotTexts(tab: PersistedWorkspaceTab): string[] {
  return Object.values(tab.snapshot.entities ?? {})
    .filter((entity) => entity?.kind === 'text')
    .map((entity) => (entity as { text?: string }).text ?? '')
}

function addNote(text: string): void {
  applyCanvasPatch({ entities: [{ kind: 'text', text, canvasX: 0, canvasY: 0 }] })
}

describe('background-tab writes', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('lands in the target tab\'s snapshot and .canvas file', async () => {
    const scratch = newBackgroundTab('scratch')
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'user note' })
    await settleSync()
    harness.flush()

    const result = withTabContext(scratch.id, () =>
      applyCanvasPatch({
        entities: [
          { kind: 'text', text: 'agent one', canvasX: 10, canvasY: 20 },
          { kind: 'text', text: 'agent two', canvasX: 30, canvasY: 40 },
        ],
      }),
    )

    expect(result.created).toHaveLength(2)
    expect(snapshotTexts(scratch).sort()).toEqual(['agent one', 'agent two'])
    expect(textsOnDisk('scratch').sort()).toEqual(['agent one', 'agent two'])
  })

  it('leaves the active tab\'s runtime arrays and .canvas file untouched', async () => {
    const scratch = newBackgroundTab('scratch')
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'user note' })
    await settleSync()
    harness.flush()
    const activeBytes = readFileSync(tabFilePath('Blank'), 'utf8')

    withTabContext(scratch.id, () => addNote('agent note'))
    await settleSync()

    // The other file changed; the one the user is looking at did not.
    expect(textsOnDisk('scratch')).toEqual(['agent note'])
    expect(readFileSync(tabFilePath('Blank'), 'utf8')).toBe(activeBytes)
    expect(textEntities.map((entity) => entity.text)).toEqual(['user note'])
    expect(activeSpaceTabId).toBe(spaceTabs[0].id)
  })

  it('does not touch the user\'s undo stack, and undo is a no-op on the write', async () => {
    const scratch = newBackgroundTab('scratch')
    const userEntity = createTextEntity({ canvasX: 0, canvasY: 0, text: 'user note' })
    await settleSync()
    const stackBefore = getActiveUndoManager()!.undoStack.length

    withTabContext(scratch.id, () => addNote('agent note'))
    await settleSync()

    expect(getActiveUndoManager()!.undoStack.length).toBe(stackBefore)

    undo()
    await settleSync()

    // The user's own last action undid normally...
    expect(textEntities.some((entity) => entity.id === userEntity.id)).toBe(false)
    // ...and the agent's write is untouched by it.
    expect(snapshotTexts(scratch)).toEqual(['agent note'])
    expect(textsOnDisk(scratch)).toEqual(['agent note'])
  })

  it('writes the whole batch as one transaction in the detached doc', () => {
    const scratch = newBackgroundTab('scratch')

    let transactions = 0
    withTabContext(scratch.id, () => {
      getActiveDoc().on('afterTransaction', () => {
        transactions += 1
      })
      applyCanvasPatch({
        entities: [
          { kind: 'text', text: 'a', canvasX: 0, canvasY: 0 },
          { kind: 'text', text: 'b', canvasX: 0, canvasY: 0 },
          { kind: 'text', text: 'c', canvasX: 0, canvasY: 0 },
        ],
      })
    })

    expect(transactions).toBe(1)
    expect(snapshotTexts(scratch)).toHaveLength(3)
  })

  it('reads a background tab without swapping or activating', async () => {
    const scratch = newBackgroundTab('scratch')
    withTabContext(scratch.id, () => addNote('agent note'))
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'user note' })
    await settleSync()

    const doc = readCanvasDocument(scratch)

    expect(nodeTexts(doc)).toEqual(['agent note'])
    // The read names the canvas the user is still on, and nothing swapped.
    expect(doc.appState?.activeTab?.name).toBe('Blank')
    expect(activeSpaceTabId).toBe(spaceTabs[0].id)
    expect(textEntities.map((entity) => entity.text)).toEqual(['user note'])
  })

  it('refuses to create a page on a background tab and applies nothing', () => {
    const scratch = newBackgroundTab('scratch')

    expect(() =>
      withTabContext(scratch.id, () =>
        applyCanvasPatch({
          entities: [
            { kind: 'text', text: 'agent note', canvasX: 0, canvasY: 0 },
            { kind: 'page', url: 'http://localhost:4321/', canvasX: 0, canvasY: 0 },
          ],
        }),
      ),
    ).toThrow(/pages cannot be created, edited, or deleted on a background tab/)

    expect(snapshotTexts(scratch)).toEqual([])
    expect(pages).toHaveLength(0)
  })

  it('carries a background tab\'s existing pages through as data only', async () => {
    // Build the page on the tab that starts active, then move focus away so
    // the page-bearing tab is the background target.
    const created = applyCanvasPatch({
      entities: [{ kind: 'page', url: 'http://localhost:4321/', canvasX: 0, canvasY: 0 }],
    })
    const pageId = created.created[0]
    const scratch = newBackgroundTab('scratch')
    const withPage = spaceTabs[0]
    await settleSync()
    setActiveSpaceTab(scratch.id)
    await settleSync()
    expect(pages).toHaveLength(0)

    withTabContext(withPage.id, () => addNote('agent note'))

    // The page survived the swap in the snapshot and on disk, and no live view
    // was minted for it while the user was elsewhere.
    expect(withPage.snapshot.pages.map((page) => page.id)).toEqual([pageId])
    expect(pages).toHaveLength(0)
    const doc = readCanvasFile(tabFilePath('Blank'))
    expect((doc?.nodes ?? []).filter((node) => node.type === 'link')).toHaveLength(1)
    expect(nodeTexts(doc)).toEqual(['agent note'])

    // Editing that carried-through page is refused rather than silently lost.
    expect(() =>
      withTabContext(withPage.id, () =>
        applyCanvasPatch({ entities: [{ id: pageId, canvasX: 999 }] }),
      ),
    ).toThrow(/pages cannot be created, edited, or deleted on a background tab/)
  })
})
