/**
 * The unified duplicate/clone path against the real runtime, in-process.
 *
 * Three clone mechanisms used to exist (duplicateEntity's own per-kind
 * switch, pasteEntitiesFromClipboard, duplicateGroup). cmd-D now always
 * clones through copy/paste (`duplicateSelection` -> `copyableSelectionPayload`
 * + `pasteEntitiesFromClipboard`, or `duplicateEntity` -> `copyableEntityPayload`
 * + `pasteEntitiesFromClipboard` for the per-kind context-menu channels), so
 * a placement bug fixed once (shapes/drawings missing from collision
 * detection) can't silently regress in only one of the three call sites.
 *
 * Mutation-verified by:
 * - "cmd-D duplicates a shape without stacking it on an existing shape":
 *   removing the `shapeEntities`/`drawingEntities` spreads from
 *   `occupiedRegions()` (src/main/workspace-placement.ts) makes the clone
 *   land exactly on the second shape instead of below it.
 * - "duplicating a shape via cmd-D round-trips through undo as one step":
 *   removing `duplicateSelection`'s `pasteEntitiesFromClipboard(...)` call
 *   (src/main/runtime/duplicate-selection.ts) makes cmd-D a no-op, failing
 *   the creation assertion.
 * - "pasteEntitiesFromClipboard keeps the caller's exact placement, no
 *   re-snap": restoring `snapToGrid(...)` around `input.canvasX + entity.dx`
 *   in `pasteEntitiesInternal` (src/main/workspace-clipboard.ts) rounds
 *   137/213 to 140/220, failing the exact-position assertion.
 * - "duplicateGroup keeps an explicit drag-copy placement exactly, no
 *   re-snap": restoring `snapToGrid(...)` around `input.placement` in
 *   `duplicateGroupInternal` (src/main/workspace-groups.ts) rounds the
 *   group's position, failing the exact-position assertion.
 * - "per-kind context-menu duplicate (duplicateEntity) clones a file entity
 *   via the paste path": throwing from `duplicateEntity` before it delegates
 *   to `copyableEntityPayload`/`pasteEntitiesFromClipboard`
 *   (src/main/workspace-pages.ts) fails the creation assertion.
 * - "duplicating a page carries its anchored sticky, re-attached to the
 *   cloned page": deleting the `anchorables` attachment loop at the end of
 *   `pasteEntitiesInternal` (src/main/workspace-clipboard.ts) leaves the
 *   cloned sticky with no `pageAnchor`; deleting the
 *   `withPageAnchoredEntityIds` expansion in `copyableEntityPayload` leaves
 *   the sticky uncloned entirely.
 * - "pasting an anchored sticky without its page re-resolves the anchor by
 *   placement": replacing the attachment loop's `reanchorEntityById`
 *   fallback with a no-op leaves the pasted-onto-page clone unanchored.
 * - "copying a scrolled page keeps the anchored sticky's apparent offset":
 *   reverting `apparentPosition(...)` to raw `canvasX/canvasY` in
 *   `copyableEntityPayload` (src/main/workspace-clipboard.ts) pastes the
 *   clone at the stale pre-scroll offset (100 instead of 70).
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import {
  createFileEntity,
  createShapeEntity,
  createTextEntity,
  getFileEntities,
  getShapeEntities,
  getTextEntities,
} from '../../src/main/runtime/document-commands'
import { duplicateSelection } from '../../src/main/runtime/duplicate-selection'
import { duplicateEntity } from '../../src/main/workspace-pages'
import { applyCanvasPatch } from '../../src/main/canvas-apply'
import { pages } from '../../src/main/runtime/runtime-context'
import { copyableEntityPayload } from '../../src/main/workspace-clipboard'
import { selectEntity, selectNone } from '../../src/main/runtime/selection-controller'
import { createUserGroup, duplicateGroup } from '../../src/main/workspace-groups'
import { copyableSelectionPayload, pasteEntitiesFromClipboard } from '../../src/main/workspace-clipboard'
import { undo, redo } from '../../src/main/runtime/space-undo'
import { workspaceGroups } from '../../src/main/runtime/space-model'

let harness: WorkspaceHarness

describe('unified duplicate path', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
  })

  afterAll(() => harness?.dispose())

  it('cmd-D duplicates a shape without stacking it on an existing shape', async () => {
    // B sits exactly where findDuplicatePlacement's first ("right of A")
    // candidate would land — the collision this bug hides.
    const a = createShapeEntity({ canvasX: 0, canvasY: 0, width: 100, height: 100, shapeKind: 'rectangle' })
    const b = createShapeEntity({ canvasX: 180, canvasY: 0, width: 100, height: 100, shapeKind: 'rectangle' })
    await settleSync()

    selectEntity(a.id, 'shape')
    duplicateSelection()
    await settleSync()

    const shapes = getShapeEntities()
    expect(shapes).toHaveLength(3)
    const clone = shapes.find((s) => s.id !== a.id && s.id !== b.id)
    expect(clone).toBeDefined()
    expect({ x: clone!.canvasX, y: clone!.canvasY }).not.toEqual({ x: b.canvasX, y: b.canvasY })
  })

  it('duplicating a shape via cmd-D round-trips through undo as one step', async () => {
    const shape = createShapeEntity({ canvasX: 0, canvasY: 0, width: 100, height: 100, shapeKind: 'rectangle' })
    await settleSync()

    selectEntity(shape.id, 'shape')
    duplicateSelection()
    await settleSync()
    expect(getShapeEntities()).toHaveLength(2)

    undo()
    expect(getShapeEntities()).toHaveLength(1)
    expect(getShapeEntities()[0].id).toBe(shape.id)

    redo()
    expect(getShapeEntities()).toHaveLength(2)
  })

  it('per-kind context-menu duplicate (duplicateEntity) clones a file entity via the paste path', async () => {
    const file = createFileEntity({ canvasX: 0, canvasY: 0, file: '/tmp/does-not-exist.png', width: 100, height: 100 })
    await settleSync()

    const result = duplicateEntity({ entityId: file.id, focus: true })
    await settleSync()

    const files = getFileEntities()
    expect(files).toHaveLength(2)
    const clone = files.find((f) => f.id === result.entityId)
    expect(clone).toBeDefined()
    expect(clone?.file).toBe('/tmp/does-not-exist.png')
    expect({ x: clone!.canvasX, y: clone!.canvasY }).not.toEqual({ x: file.canvasX, y: file.canvasY })
  })

  it('pasteEntitiesFromClipboard keeps the caller\'s exact placement, no re-snap', async () => {
    const note = createTextEntity({ canvasX: 0, canvasY: 0, text: 'hi' })
    await settleSync()
    selectEntity(note.id, 'text')

    const payload = copyableSelectionPayload()
    expect(payload).toBeTruthy()

    // 137/213 are not multiples of the 20px grid — a re-introduced
    // commit-time snap would round these to 140/220.
    const result = pasteEntitiesFromClipboard({ payload: payload!, canvasX: 137, canvasY: 213 })
    await settleSync()

    const pasted = getTextEntities().find((t) => t.id === result.entityIds[0])
    expect(pasted?.canvasX).toBe(137)
    expect(pasted?.canvasY).toBe(213)
  })

  it('duplicateGroup keeps an explicit drag-copy placement exactly, no re-snap', async () => {
    const note = createTextEntity({ canvasX: 0, canvasY: 0, text: 'hi' })
    await settleSync()
    const group = createUserGroup([note.id], 'Group')
    await settleSync()

    const { groupId } = duplicateGroup({
      groupId: group.id,
      placement: { canvasX: 137, canvasY: 213 },
    })
    await settleSync()

    const clone = workspaceGroups.find((g) => g.id === groupId)
    expect(clone?.canvasX).toBe(137)
    expect(clone?.canvasY).toBe(213)
  })

  it('duplicating a page carries its anchored sticky, re-attached to the cloned page', async () => {
    const { created } = applyCanvasPatch({
      entities: [
        { kind: 'page', url: 'https://example.com', canvasX: 0, canvasY: 0, presetIndex: 0 },
      ],
    })
    const pageId = created[0]
    await settleSync()

    // createTextEntity resolves the anchor by placement — center on the page body.
    const sticky = createTextEntity({ canvasX: 40, canvasY: 100, text: 'on page' })
    await settleSync()
    expect(getTextEntities().find((t) => t.id === sticky.id)?.pageAnchor?.pageId).toBe(pageId)

    selectEntity(pageId, 'page')
    duplicateSelection()
    await settleSync()

    expect(pages).toHaveLength(2)
    const clonePage = pages.find((p) => p.id !== pageId)
    const cloneSticky = getTextEntities().find((t) => t.id !== sticky.id)
    expect(clonePage).toBeDefined()
    expect(cloneSticky).toBeDefined()
    expect(cloneSticky?.pageAnchor?.pageId).toBe(clonePage!.id)

    undo()
    expect(pages).toHaveLength(1)
    expect(getTextEntities()).toHaveLength(1)

    redo()
    await settleSync()
    expect(pages).toHaveLength(2)
    const redonePage = pages.find((p) => p.id !== pageId)
    const redoneSticky = getTextEntities().find((t) => t.id !== sticky.id)
    expect(redoneSticky?.pageAnchor?.pageId).toBe(redonePage!.id)
  })

  it('copying a scrolled page keeps the anchored sticky\'s apparent offset', async () => {
    const { created } = applyCanvasPatch({
      entities: [
        { kind: 'page', url: 'https://example.com', canvasX: 0, canvasY: 0, presetIndex: 0 },
      ],
    })
    const pageId = created[0]
    await settleSync()

    // Anchored at scroll 0; the page then scrolls down 30px, so the sticky
    // appears 30px higher than its stored coords (scroll-follow projection).
    const sticky = createTextEntity({ canvasX: 40, canvasY: 100, text: 'on page' })
    await settleSync()
    expect(getTextEntities().find((t) => t.id === sticky.id)?.pageAnchor?.pageId).toBe(pageId)
    const page = pages.find((p) => p.id === pageId)!
    page.scrollY = 30

    const payload = copyableEntityPayload([pageId])
    expect(payload).toBeTruthy()
    const result = pasteEntitiesFromClipboard({ payload: payload!, canvasX: 1000, canvasY: 1000 })
    await settleSync()

    const clonePage = pages.find((p) => result.entityIds.includes(p.id))
    const cloneSticky = getTextEntities().find(
      (t) => t.id !== sticky.id && result.entityIds.includes(t.id),
    )
    expect(clonePage).toBeDefined()
    expect(cloneSticky).toBeDefined()
    // The clone reproduces what the user SAW: 70px below the page top, not
    // the stale stored offset of 100px.
    expect(cloneSticky!.canvasX - clonePage!.canvasX).toBe(40)
    expect(cloneSticky!.canvasY - clonePage!.canvasY).toBe(70)
  })

  it('pasting an anchored sticky without its page re-resolves the anchor by placement', async () => {
    const { created } = applyCanvasPatch({
      entities: [
        { kind: 'page', url: 'https://example.com', canvasX: 0, canvasY: 0, presetIndex: 0 },
      ],
    })
    const pageId = created[0]
    await settleSync()
    const sticky = createTextEntity({ canvasX: 40, canvasY: 100, text: 'on page' })
    await settleSync()
    expect(getTextEntities().find((t) => t.id === sticky.id)?.pageAnchor?.pageId).toBe(pageId)

    const payload = copyableEntityPayload([sticky.id])
    expect(payload).toBeTruthy()

    // Far from any page: the clone detaches.
    const far = pasteEntitiesFromClipboard({ payload: payload!, canvasX: 4000, canvasY: 4000 })
    await settleSync()
    expect(getTextEntities().find((t) => t.id === far.entityIds[0])?.pageAnchor).toBeUndefined()

    // Back onto the original page body: the clone anchors to the original page.
    const onPage = pasteEntitiesFromClipboard({ payload: payload!, canvasX: 60, canvasY: 120 })
    await settleSync()
    expect(
      getTextEntities().find((t) => t.id === onPage.entityIds[0])?.pageAnchor?.pageId,
    ).toBe(pageId)
  })
})
