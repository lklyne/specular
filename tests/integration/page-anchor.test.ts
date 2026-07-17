/**
 * Page-anchored entities (shared/page-anchor.ts) against the real runtime,
 * in-process: placement decides anchoring (create on a page → anchored; drag
 * off → free; drop on → anchored), the anchor persists to disk and
 * round-trips undo, anchored entities travel with their page's drag set,
 * hide while the page is on a different URL, nest under the page in the
 * sidebar, and go free-form when the page is deleted.
 *
 * Mutation-verified by:
 * - removing the `reanchorEntityById` call in `finalizeDrag`
 *   (document-commands.ts) — the drop-on / drag-off cases and the undo
 *   round-trip fail;
 * - replacing `pageAnchor: entity.pageAnchor` with `pageAnchor: undefined`
 *   in `persistTextEntity` (text-entity-state.ts) — the disk-persistence and
 *   undo cases fail (the doc sync derives from the persist projection);
 * - removing the pageIds filter in `withPageAnchoredEntityIds`
 *   (page-anchor-state.ts, return `ids` unconditionally) — the drag-set
 *   expansion case fails.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import type {
  JsonCanvasLinkNode,
  JsonCanvasTextNode,
  JsonCanvasDrawingNode,
  JsonCanvasShapeNode,
} from '../../src/shared/json-canvas-types'
import {
  applyDragDelta,
  createDrawingEntity,
  createShapeEntity,
  createTextEntity,
  finalizeDrag,
  initializeDrag,
} from '../../src/main/runtime/document-commands'
import { hiddenByPageAnchor } from '../../src/main/runtime/document-binding'
import {
  reanchorEntityById,
  withPageAnchoredEntityIds,
} from '../../src/main/runtime/page-anchor-state'
import {
  buildTextEntitySceneEntity,
  textEntities,
} from '../../src/main/runtime/text-entity-state'
import {
  buildDrawingEntitySceneEntity,
  drawingEntities,
} from '../../src/main/runtime/drawing-entity-state'
import {
  buildShapeEntitySceneEntity,
  shapeEntities,
} from '../../src/main/runtime/shape-entity-state'
import { pages } from '../../src/main/runtime/runtime-context'
import { removePageById } from '../../src/main/runtime/page-runtime'
import { getLeftSidebarData } from '../../src/main/runtime/canvas-layout-data'
import { undo, redo } from '../../src/main/runtime/workspace-undo'
import { selectNone } from '../../src/main/runtime/selection-controller'

let harness: WorkspaceHarness

const PAGE_ID = 'page-anchor-host'
const PAGE_URL = 'https://example.com/pricing'

function hostPageNode(): JsonCanvasLinkNode {
  return {
    id: PAGE_ID,
    type: 'link',
    x: 120,
    y: 120,
    width: 375,
    height: 667,
    url: PAGE_URL,
    presetIndex: 0,
  }
}

function loadHostPage(): void {
  harness.loadFixture({
    name: 'Page anchor host',
    doc: {
      nodes: [hostPageNode()],
      edges: [],
      appState: { zoom: 1, pan: { x: 0, y: 0 } },
    },
  })
}

/** A spot whose center lands well inside the host page's body. */
const ON_PAGE = { canvasX: 150, canvasY: 150, width: 100, height: 100 }
/** Empty canvas, far from any page. */
const OFF_PAGE = { canvasX: 2000, canvasY: 2000, width: 100, height: 100 }

describe('page-anchored entities', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
  })

  afterAll(() => harness?.dispose())

  it('anchors a sticky created on a page and persists the anchor to disk', async () => {
    loadHostPage()
    const sticky = createTextEntity({ ...ON_PAGE, text: 'anchored sticky' })
    expect(sticky.pageAnchor).toEqual({ pageId: PAGE_ID, pageUrl: PAGE_URL, scrollX: 0, scrollY: 0 })
    await settleSync()

    const node = harness
      .diskDoc()
      ?.nodes.find((candidate) => candidate.id === sticky.id) as JsonCanvasTextNode | undefined
    expect(node?.specular?.pageAnchor).toEqual({ pageId: PAGE_ID, pageUrl: PAGE_URL, scrollX: 0, scrollY: 0 })
  })

  it('anchors a drawing created on a page and persists the anchor to disk', async () => {
    loadHostPage()
    const drawing = createDrawingEntity({
      ...ON_PAGE,
      strokes: [
        {
          id: 'stroke-1',
          color: '#ff0000',
          width: 4,
          points: [
            { x: 160, y: 160 },
            { x: 220, y: 220 },
          ],
        },
      ],
    })
    expect(drawing.pageAnchor).toEqual({ pageId: PAGE_ID, pageUrl: PAGE_URL, scrollX: 0, scrollY: 0 })
    await settleSync()

    const node = harness
      .diskDoc()
      ?.nodes.find((candidate) => candidate.id === drawing.id) as JsonCanvasDrawingNode | undefined
    expect(node?.pageAnchor).toEqual({ pageId: PAGE_ID, pageUrl: PAGE_URL, scrollX: 0, scrollY: 0 })
  })

  it('creates free-form on empty canvas', () => {
    loadHostPage()
    const sticky = createTextEntity({ ...OFF_PAGE, text: 'free sticky' })
    expect(sticky.pageAnchor).toBeUndefined()
  })

  it('drop on a page anchors; the drag undoes as one step including the anchor', async () => {
    loadHostPage()
    const sticky = createTextEntity({ ...OFF_PAGE, text: 'roaming sticky' })
    await settleSync()

    initializeDrag([sticky.id])
    applyDragDelta([sticky.id], ON_PAGE.canvasX - OFF_PAGE.canvasX, ON_PAGE.canvasY - OFF_PAGE.canvasY)
    finalizeDrag()
    await settleSync()

    const entity = () => textEntities.find((candidate) => candidate.id === sticky.id)
    expect(entity()?.pageAnchor).toEqual({ pageId: PAGE_ID, pageUrl: PAGE_URL, scrollX: 0, scrollY: 0 })

    undo()
    expect(entity()?.canvasX).toBe(OFF_PAGE.canvasX)
    expect(entity()?.pageAnchor).toBeUndefined()

    redo()
    expect(entity()?.pageAnchor).toEqual({ pageId: PAGE_ID, pageUrl: PAGE_URL, scrollX: 0, scrollY: 0 })
  })

  it('drag off a page clears the anchor', async () => {
    loadHostPage()
    const sticky = createTextEntity({ ...ON_PAGE, text: 'departing sticky' })
    expect(sticky.pageAnchor).toBeDefined()
    await settleSync()

    initializeDrag([sticky.id])
    applyDragDelta([sticky.id], 1500, 1500)
    finalizeDrag()
    await settleSync()

    expect(textEntities.find((candidate) => candidate.id === sticky.id)?.pageAnchor).toBeUndefined()
  })

  it('expands a page drag set with its anchored entities, which then travel with the page', async () => {
    loadHostPage()
    const sticky = createTextEntity({ ...ON_PAGE, text: 'passenger sticky' })
    await settleSync()

    const dragIds = withPageAnchoredEntityIds([PAGE_ID])
    expect(dragIds).toContain(sticky.id)

    const page = pages.find((candidate) => candidate.id === PAGE_ID)!
    const pageX = page.canvasX
    const stickyX = sticky.canvasX
    initializeDrag(dragIds)
    applyDragDelta(dragIds, 240, 0)
    finalizeDrag()
    await settleSync()

    const movedSticky = textEntities.find((candidate) => candidate.id === sticky.id)!
    expect(page.canvasX).toBe(pageX + 240)
    // Text positions grid-snap during drags; the sticky must have traveled
    // with the page (within one grid step), not been left behind.
    expect(Math.abs(movedSticky.canvasX - (stickyX + 240))).toBeLessThanOrEqual(20)
    // Still anchored — it moved with its page, not off it.
    expect(movedSticky.pageAnchor?.pageId).toBe(PAGE_ID)
  })

  it('hides the entity while the page is on a different URL and dims its sidebar row', () => {
    loadHostPage()
    const sticky = createTextEntity({ ...ON_PAGE, text: 'document-bound sticky' })
    const entity = textEntities.find((candidate) => candidate.id === sticky.id)!
    expect(hiddenByPageAnchor(entity)).toBe(false)

    const sidebarPage = () => {
      const item = getLeftSidebarData().sections.pages.find((entry) => entry.id === PAGE_ID)
      return item && item.kind === 'page' ? item : null
    }
    expect(sidebarPage()?.children?.[0]).toMatchObject({ id: sticky.id, onCurrentPage: true })
    // Nested under the page — not in the root notes list.
    expect(
      getLeftSidebarData().sections.notes.some((entry) => entry.id === sticky.id),
    ).toBe(false)

    // did-navigate updates page.url in place.
    const page = pages.find((candidate) => candidate.id === PAGE_ID)!
    page.url = 'https://example.com/elsewhere'
    expect(hiddenByPageAnchor(entity)).toBe(true)
    expect(sidebarPage()?.children?.[0]).toMatchObject({ id: sticky.id, onCurrentPage: false })

    page.url = PAGE_URL
    expect(hiddenByPageAnchor(entity)).toBe(false)
  })

  it('clears anchors when the page is deleted; the entity goes free-form', async () => {
    loadHostPage()
    const sticky = createTextEntity({ ...ON_PAGE, text: 'orphaned sticky' })
    expect(sticky.pageAnchor).toBeDefined()
    await settleSync()

    removePageById(PAGE_ID)
    await settleSync()

    const entity = textEntities.find((candidate) => candidate.id === sticky.id)
    expect(entity).toBeDefined()
    expect(entity?.pageAnchor).toBeUndefined()
    expect(
      getLeftSidebarData().sections.notes.some((entry) => entry.id === sticky.id),
    ).toBe(true)
  })

  it('restores anchors from a .canvas file', () => {
    harness.loadFixture({
      name: 'Anchored restore',
      doc: {
        nodes: [
          hostPageNode(),
          {
            id: 'text-restored',
            type: 'text',
            x: 150,
            y: 150,
            width: 100,
            height: 100,
            text: 'restored sticky',
            color: '3',
            specular: { pageAnchor: { pageId: PAGE_ID, pageUrl: PAGE_URL } },
          } as JsonCanvasTextNode,
        ],
        edges: [],
        appState: { zoom: 1, pan: { x: 0, y: 0 } },
      },
    })
    const entity = textEntities.find((candidate) => candidate.id === 'text-restored')
    expect(entity?.pageAnchor).toEqual({ pageId: PAGE_ID, pageUrl: PAGE_URL })

    const drawing = drawingEntities.length
    expect(drawing).toBe(0)
  })
})

/**
 * Shapes anchor like other entities but additionally scroll-follow: the
 * anchor stamps the page's scroll offset at placement, the scene projection
 * renders shifted by the scroll delta since, and reanchoring folds the
 * accumulated shift into the stored coordinates (see shared/page-anchor.ts).
 *
 * Mutation-verified by:
 * - removing the scroll stamp in `reanchorEntityById` (page-anchor-state.ts)
 *   — the stamp/persistence case fails;
 * - dropping the `pageAnchorScrollShift` term in `buildShapeEntitySceneEntity`
 *   (shape-entity-state.ts) — the scroll-follow case fails;
 * - removing the `rebaseAnchorScroll` call in `reanchorEntityById` — the
 *   rebase case fails.
 */
describe('scroll-following shapes', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
  })

  const hostPage = () => pages.find((candidate) => candidate.id === PAGE_ID)!
  const shape = (id: string) => shapeEntities.find((candidate) => candidate.id === id)!
  const sceneCanvasY = (id: string) =>
    buildShapeEntitySceneEntity(shape(id), 1, { x: 0, y: 0 }, { x: 0, y: 0 }).canvasY

  it('anchors a shape created on a page, stamps the scroll reference, and persists it', async () => {
    loadHostPage()
    hostPage().scrollY = 120
    const created = createShapeEntity({ ...ON_PAGE, shapeKind: 'rectangle' })
    expect(created.pageAnchor).toEqual({
      pageId: PAGE_ID,
      pageUrl: PAGE_URL,
      scrollX: 0,
      scrollY: 120,
    })
    await settleSync()

    const node = harness
      .diskDoc()
      ?.nodes.find((candidate) => candidate.id === created.id) as JsonCanvasShapeNode | undefined
    expect(node?.pageAnchor).toEqual({
      pageId: PAGE_ID,
      pageUrl: PAGE_URL,
      scrollX: 0,
      scrollY: 120,
    })
  })

  it('scroll-follows: the scene position shifts with page scroll, stored coords unchanged', () => {
    loadHostPage()
    const created = createShapeEntity({ ...ON_PAGE, shapeKind: 'rectangle' })
    expect(sceneCanvasY(created.id)).toBe(ON_PAGE.canvasY)

    hostPage().scrollY = 200
    expect(sceneCanvasY(created.id)).toBe(ON_PAGE.canvasY - 200)
    expect(shape(created.id).canvasY).toBe(ON_PAGE.canvasY)
  })

  it('rebases on reanchor: the shift folds into stored coords, apparent position unchanged', async () => {
    loadHostPage()
    // Grid-aligned so the drag's snap doesn't move the shape on its own.
    const onGrid = { canvasX: 160, canvasY: 160, width: 100, height: 100 }
    const created = createShapeEntity({ ...onGrid, shapeKind: 'rectangle' })
    await settleSync()

    // Small enough that the shape's apparent center stays inside the page
    // body — a larger scroll legitimately unanchors it at drag end (the
    // placement rule sees it apparently off the page).
    hostPage().scrollY = 40
    const apparentBefore = sceneCanvasY(created.id)
    // A zero-delta drag end runs the reanchor pass the way the app does.
    initializeDrag([created.id])
    applyDragDelta([created.id], 0, 0)
    finalizeDrag()
    await settleSync()

    expect(shape(created.id).canvasY).toBe(onGrid.canvasY - 40)
    expect(shape(created.id).pageAnchor?.scrollY).toBe(40)
    expect(sceneCanvasY(created.id)).toBe(apparentBefore)

    // The rebase is a tracked mutation: undo restores coords and reference.
    undo()
    expect(shape(created.id).canvasY).toBe(onGrid.canvasY)
    expect(shape(created.id).pageAnchor?.scrollY).toBe(0)
  })

  it('travels with its page in a drag set like other anchored entities', () => {
    loadHostPage()
    const created = createShapeEntity({ ...ON_PAGE, shapeKind: 'rectangle' })
    expect(withPageAnchoredEntityIds([PAGE_ID])).toContain(created.id)
  })
})

/**
 * Text and drawings scroll-follow their page like shapes. Drawings store
 * stroke points in absolute canvas coords, so both the scene projection and
 * the reanchor rebase must shift the points along with the bounds.
 *
 * Mutation-verified by:
 * - dropping the `pageAnchorScrollShift` term in `buildTextEntitySceneEntity`
 *   — the text scroll-follow case fails;
 * - dropping the stroke shift in `buildDrawingEntitySceneEntity` — the
 *   drawing scene case fails on the stroke point;
 * - dropping the stroke shift in `rebaseAnchorScroll` (page-anchor-state.ts)
 *   — the rebase case fails on the stored stroke point.
 */
describe('scroll-following text and drawings', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
  })

  const hostPage = () => pages.find((candidate) => candidate.id === PAGE_ID)!
  const ORIGIN = { zoom: 1, pan: { x: 0, y: 0 }, canvasOrigin: { x: 0, y: 0 } }

  it('text scene position shifts with page scroll, stored coords unchanged', () => {
    loadHostPage()
    const created = createTextEntity({ ...ON_PAGE, text: 'following sticky' })
    hostPage().scrollY = 200
    const scene = buildTextEntitySceneEntity(created, ORIGIN.zoom, ORIGIN.pan, ORIGIN.canvasOrigin)
    expect(scene.canvasY).toBe(ON_PAGE.canvasY - 200)
    expect(created.canvasY).toBe(ON_PAGE.canvasY)
  })

  it('drawing scene bounds and stroke points shift with page scroll', () => {
    loadHostPage()
    const created = createDrawingEntity({
      ...ON_PAGE,
      strokes: [
        { id: 's1', color: '#ff0000', width: 4, points: [{ x: 160, y: 160 }] },
      ],
    })
    hostPage().scrollY = 200
    const scene = buildDrawingEntitySceneEntity(
      created, ORIGIN.zoom, ORIGIN.pan, ORIGIN.canvasOrigin,
    )
    expect(scene.canvasY).toBe(ON_PAGE.canvasY - 200)
    expect(scene.strokes[0].points[0]).toEqual({ x: 160, y: -40 })
    expect(created.strokes[0].points[0]).toEqual({ x: 160, y: 160 })
  })

  it('drawing rebase folds the shift into stored stroke points', async () => {
    loadHostPage()
    const created = createDrawingEntity({
      canvasX: 160,
      canvasY: 160,
      width: 100,
      height: 100,
      strokes: [
        { id: 's1', color: '#ff0000', width: 4, points: [{ x: 180, y: 180 }] },
      ],
    })
    await settleSync()

    hostPage().scrollY = 40
    initializeDrag([created.id])
    applyDragDelta([created.id], 0, 0)
    finalizeDrag()
    await settleSync()

    const entity = () => drawingEntities.find((candidate) => candidate.id === created.id)!
    expect(entity().canvasY).toBe(120)
    expect(entity().strokes[0].points[0]).toEqual({ x: 180, y: 140 })
    expect(entity().pageAnchor?.scrollY).toBe(40)

    undo()
    expect(entity().canvasY).toBe(160)
    expect(entity().strokes[0].points[0]).toEqual({ x: 180, y: 180 })
  })
})
