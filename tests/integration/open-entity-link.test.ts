/**
 * Opening a link from an entity's text (click on a markdown link in a
 * sticky) against the real runtime.
 *
 * Guards `openLinkFromEntity` end to end: a first open creates a page for
 * the URL placed beside the source entity plus a connection edge back to it
 * — landing in the runtime arrays and the Y.Doc, and surviving an undo
 * round-trip — while a second open of an equivalent URL (trailing-slash
 * variant included) reveals the existing page instead of duplicating it.
 *
 * Mutation-verified by: deleting the `createEdges` call from
 * `openLinkFromEntity` (src/main/workspace-pages.ts) — "connects the new
 * page back to the source entity with an edge" fails; deleting the
 * `existing` early-return — "reveals the existing page instead of opening a
 * duplicate" fails with a second page.
 */

import { beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { openLinkFromEntity } from '../../src/main/workspace-pages'
import { createTextEntity } from '../../src/main/runtime/document-commands'
import { pages } from '../../src/main/runtime/runtime-context'
import { workspaceEdges } from '../../src/main/runtime/space-model'
import { undo } from '../../src/main/runtime/space-undo'
import { DOC_MAP_EDGES, DOC_MAP_PAGES } from '../../src/main/runtime/space-doc'

let harness: WorkspaceHarness

function createSourceNote(): string {
  const note = createTextEntity({
    canvasX: 100,
    canvasY: 100,
    width: 200,
    height: 200,
    text: 'see https://example.com',
  })
  return note.id
}

describe('open entity link', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  it('opens a page for the URL beside the source entity', async () => {
    const noteId = createSourceNote()
    const result = openLinkFromEntity({ entityId: noteId, url: 'example.com' })
    await settleSync()

    expect(result).not.toBeNull()
    expect(pages).toHaveLength(1)
    expect(pages[0].url).toBe('https://example.com/')
    // Beside the note, not on top of it: strictly right of its right edge.
    expect(pages[0].canvasX).toBeGreaterThan(300)
    expect(harness.doc.getMap(DOC_MAP_PAGES).has(pages[0].id)).toBe(true)
  })

  it('connects the new page back to the source entity with an edge', async () => {
    const noteId = createSourceNote()
    const result = openLinkFromEntity({ entityId: noteId, url: 'https://example.com' })
    await settleSync()

    expect(workspaceEdges).toHaveLength(1)
    expect(workspaceEdges[0].fromEntityId).toBe(noteId)
    expect(workspaceEdges[0].toEntityId).toBe(result?.pageId)
    expect(workspaceEdges[0].kind).toBe('connection')
    expect(harness.doc.getMap(DOC_MAP_EDGES).has(workspaceEdges[0].id)).toBe(true)
  })

  it('reveals the existing page instead of opening a duplicate', async () => {
    const noteId = createSourceNote()
    const first = openLinkFromEntity({ entityId: noteId, url: 'https://example.com' })
    await settleSync()

    // Equivalent forms of the same URL: bare host, trailing slash.
    const again = openLinkFromEntity({ entityId: noteId, url: 'example.com/' })
    await settleSync()

    expect(again?.pageId).toBe(first?.pageId)
    expect(pages).toHaveLength(1)
    expect(workspaceEdges).toHaveLength(1)
  })

  it('rejects text that does not normalize to a URL', async () => {
    const noteId = createSourceNote()
    expect(openLinkFromEntity({ entityId: noteId, url: '   ' })).toBeNull()
    await settleSync()
    expect(pages).toHaveLength(0)
  })

  it('undo removes the page and its edge together', async () => {
    const noteId = createSourceNote()
    await settleSync()
    openLinkFromEntity({ entityId: noteId, url: 'https://example.com' })
    await settleSync()
    expect(pages).toHaveLength(1)
    expect(workspaceEdges).toHaveLength(1)

    undo()
    await settleSync()

    expect(pages).toHaveLength(0)
    expect(workspaceEdges).toHaveLength(0)
    expect(harness.doc.getMap(DOC_MAP_EDGES).size).toBe(0)
  })
})
