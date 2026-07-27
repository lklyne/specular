/**
 * `annotateSelectionRegion` — the one main-side door behind the selection
 * popup's Annotate button, `POST /selection/annotate`, and the
 * `specular annotate-selection` CLI verb. A multi-selection becomes one plain
 * region annotation over the selection's union bounds; membership rides in
 * metadata (`selectionEntityIds`), and the artifact the request is about
 * (`selectionTarget`) is derived: exactly one page → that page + its url, else
 * exactly one file entity → that file + its absolute path, else omitted.
 *
 * Capture is unavailable in-process (no live window), so these also pin the
 * degrade: the annotation is still created, just without a screenshot.
 *
 * Mutation-verified by:
 * - dropping the `metadata` spread in `executeRegionSelect`'s `createAnnotation`
 *   call (region-select.ts) — every metadata case fails;
 * - returning `groupBoundsForEntityIds([entityIds[0]])` instead of the whole
 *   set in `annotateSelectionRegion` — the union-rect case fails;
 * - removing the group expansion in `targetCandidateIds` — the group case fails;
 * - loosening `pageIds.length === 1` to `>= 1` — the two-page case fails;
 * - re-throwing instead of returning `{ intersectingPages: [] }` in
 *   `captureRegionOrNone` — every case fails (no annotation is created);
 * - stamping the selection metadata in a second `mutateWorkspace` call after
 *   `executeRegionSelect` instead of passing it into creation — the undo case
 *   fails (the metadata patch undoes first, leaving the annotation behind).
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import type { ServerResponse } from 'node:http'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import type { Annotation } from '../../src/shared/types'
import { annotateSelectionRegion } from '../../src/main/runtime/annotate-selection'
import { applyCanvasPatch } from '../../src/main/canvas-apply'
import { createFileEntity, createShapeEntity } from '../../src/main/runtime/document-commands'
import { createUserGroup } from '../../src/main/workspace-groups'
import { selectEntities, selectNone } from '../../src/main/runtime/selection-controller'
import { workspaceAnnotations } from '../../src/main/runtime/workspace-model'
import { undo } from '../../src/main/runtime/workspace-undo'
import { workspaceRoutes } from '../../src/main/routes/workspace'

let harness: WorkspaceHarness

const PAGE_URL = 'https://example.com/pricing'

function createPage(url = PAGE_URL): string {
  return applyCanvasPatch({
    entities: [{ kind: 'page', url, canvasX: 0, canvasY: 0, presetIndex: 2 }],
  }).created[0]
}

describe('annotate selection', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
  })

  afterAll(() => harness?.dispose())

  it('anchors one region over the union bounds and records the selected ids', async () => {
    const a = createShapeEntity({ canvasX: 0, canvasY: 0, width: 100, height: 100 })
    const b = createShapeEntity({ canvasX: 400, canvasY: 250, width: 100, height: 50 })
    await settleSync()

    const annotation = await annotateSelectionRegion({
      entityIds: [a.id, b.id],
      text: 'align these two',
    })
    await settleSync()

    expect(annotation.anchor).toEqual({
      type: 'region',
      canvasRect: { x: 0, y: 0, width: 500, height: 300 },
    })
    expect(annotation.text).toBe('align these two')
    expect(annotation.metadata?.selectionEntityIds).toEqual([a.id, b.id])
    // Two shapes name neither a page nor a file — no target is invented.
    expect(annotation.metadata?.selectionTarget).toBeUndefined()
    // No live window in-process: the comment survives, the pixels don't.
    expect(annotation.metadata?.regionScreenshot).toBeUndefined()

    const onDisk = (harness.diskDoc()?.annotations as Annotation[] | undefined)?.find(
      (record) => record.id === annotation.id,
    )
    expect(onDisk?.metadata?.selectionEntityIds).toEqual([a.id, b.id])
  })

  it('names the single selected page as the target, with its url', async () => {
    const pageId = createPage()
    const shape = createShapeEntity({ canvasX: 900, canvasY: 0, width: 100, height: 100 })
    await settleSync()

    const annotation = await annotateSelectionRegion({
      entityIds: [pageId, shape.id],
      text: 'move this note onto the hero',
    })

    expect(annotation.metadata?.selectionTarget).toEqual({
      entityId: pageId,
      kind: 'page',
      url: PAGE_URL,
    })
  })

  it('names the single selected file entity as the target, with an absolute path', async () => {
    const file = createFileEntity({
      canvasX: 0,
      canvasY: 0,
      file: '/tmp/specular-space/hero.md',
    })
    const shape = createShapeEntity({ canvasX: 500, canvasY: 0, width: 100, height: 100 })
    await settleSync()

    const annotation = await annotateSelectionRegion({
      entityIds: [file.id, shape.id],
      text: 'tighten the copy',
    })

    expect(annotation.metadata?.selectionTarget).toEqual({
      entityId: file.id,
      kind: 'file',
      filePath: '/tmp/specular-space/hero.md',
    })
  })

  it('omits the target when the selection holds two pages', async () => {
    const first = createPage()
    const second = createPage('https://example.com/about')
    await settleSync()

    const annotation = await annotateSelectionRegion({
      entityIds: [first, second],
      text: 'these two disagree',
    })

    expect(annotation.metadata?.selectionEntityIds).toEqual([first, second])
    expect(annotation.metadata?.selectionTarget).toBeUndefined()
  })

  it('reads through a selected group to the page it holds', async () => {
    const pageId = createPage()
    const shape = createShapeEntity({ canvasX: 900, canvasY: 0, width: 100, height: 100 })
    await settleSync()
    const group = createUserGroup([pageId, shape.id])
    await settleSync()

    const annotation = await annotateSelectionRegion({
      entityIds: [group.id],
      text: 'this whole cluster',
    })

    expect(annotation.metadata?.selectionEntityIds).toEqual([group.id])
    expect(annotation.metadata?.selectionTarget?.entityId).toBe(pageId)
    expect(annotation.metadata?.selectionTarget?.kind).toBe('page')
    // The group's own rect is the region — same bounds the group was built from.
    const rect = 'canvasRect' in annotation.anchor ? annotation.anchor.canvasRect : null
    expect(rect).toEqual({
      x: group.canvasX,
      y: group.canvasY,
      width: group.width,
      height: group.height,
    })
  })

  it('falls back to the current selection when no ids are given', async () => {
    const a = createShapeEntity({ canvasX: 0, canvasY: 0, width: 100, height: 100 })
    const b = createShapeEntity({ canvasX: 200, canvasY: 0, width: 100, height: 100 })
    await settleSync()
    selectEntities([a.id, b.id])

    const annotation = await annotateSelectionRegion({ text: 'from the live selection' })

    expect(annotation.metadata?.selectionEntityIds).toEqual([a.id, b.id])
  })

  it('refuses when nothing is selected and no ids are given', async () => {
    await expect(annotateSelectionRegion({ text: 'nothing here' })).rejects.toThrow(
      /No entities selected/,
    )
  })

  it('one undo removes the annotation', async () => {
    const a = createShapeEntity({ canvasX: 0, canvasY: 0, width: 100, height: 100 })
    const b = createShapeEntity({ canvasX: 200, canvasY: 0, width: 100, height: 100 })
    await settleSync()

    await annotateSelectionRegion({ entityIds: [a.id, b.id], text: 'one step' })
    await settleSync()
    expect(workspaceAnnotations).toHaveLength(1)

    undo()
    await settleSync()
    expect(workspaceAnnotations).toHaveLength(0)
    expect(harness.diskDoc()?.annotations ?? []).toHaveLength(0)
  })
})

// The CLI's `annotate-selection` verb and the popup's Annotate button both POST
// here; guarding the route keeps those doors in sync with the seam.
describe('POST /selection/annotate', () => {
  const route = workspaceRoutes.find((r) => r.pattern === '/selection/annotate')!

  function invoke(body: unknown) {
    let status = 200
    let json: unknown
    const response = {
      statusCode: 200,
      setHeader() {},
      end(payload?: string) {
        status = response.statusCode
        json = payload ? JSON.parse(payload) : undefined
      },
    } as unknown as ServerResponse
    return route
      .handler({ response, body } as never)
      .then(() => ({
        status,
        json: json as {
          id?: string
          selectionEntityIds?: string[]
          selectionTarget?: { kind?: string }
          error?: string
        },
      }))
  }

  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
  })

  afterAll(() => harness?.dispose())

  it('annotates the ids in the body', async () => {
    const pageId = createPage()
    const shape = createShapeEntity({ canvasX: 900, canvasY: 0, width: 100, height: 100 })
    await settleSync()

    const { status, json } = await invoke({ entityIds: [pageId, shape.id], text: 'fix the nav' })
    expect(status).toBe(200)
    expect(json.selectionEntityIds).toEqual([pageId, shape.id])
    expect(json.selectionTarget?.kind).toBe('page')
    expect(workspaceAnnotations.find((a) => a.id === json.id)?.text).toBe('fix the nav')
  })

  it('rejects an empty text', async () => {
    const { status, json } = await invoke({ entityIds: ['whatever'], text: '  ' })
    expect(status).toBe(400)
    expect(json.error).toMatch(/text is required/)
  })

  it('rejects when nothing is selected', async () => {
    const { status, json } = await invoke({ text: 'no target' })
    expect(status).toBe(400)
    expect(json.error).toMatch(/No entities selected/)
  })
})
