/**
 * Main and the renderers project the same scene independently: main stamps
 * `screen*` onto every entity in `buildCanvasLayoutData`, and each renderer
 * recomputes those coordinates from the entity's canvas geometry and the
 * camera slice (`src/shared/scene-projection.ts`).
 *
 * Two implementations of one formula is a seam, and its failure mode is quiet:
 * DOM chrome sits a few pixels off the native page view it is supposed to hug,
 * or drifts only at fractional zoom, or only for shelled entities. So this
 * drives a real workspace — every entity kind, both device-shell conventions,
 * a fractional zoom and a fractional pan — and asserts the two agree exactly.
 *
 * When phase 2 deletes the `screen*` fields this test goes with them; until
 * then it is what licenses that deletion.
 *
 * Mutation-verified by dropping the `Math.round` from `snapLeft` in
 * `projectPageToScreen` (the page row fails on the fractional pan) and by
 * dropping the `* zoom` from `outsetByShell` (the shelled file row fails).
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { applyCanvasPatch } from '../../src/main/canvas-apply'
import { getCanvasLayoutData } from '../../src/main/runtime/canvas-layout-data'
import {
  setPagePreset,
  toggleDeviceShell,
  toggleFileDeviceShell,
} from '../../src/main/runtime/document-commands'
import { setPan, setZoom } from '../../src/main/runtime/viewport-control'
import { projectSceneEntity } from '../../src/shared/scene-projection'
import type { CanvasSceneEntity } from '../../src/shared/types'

let harness: WorkspaceHarness

const PAGE_ID = 'page-projection-host'

/** Every screen field an entity can carry, read off whichever kind it is. */
function screenFieldsOf(entity: CanvasSceneEntity | ReturnType<typeof projectSceneEntity>) {
  const record = entity as unknown as Record<string, number | undefined>
  return {
    screenX: record.screenX,
    screenY: record.screenY,
    screenWidth: record.screenWidth,
    screenHeight: record.screenHeight,
    contentScreenX: record.contentScreenX,
    contentScreenY: record.contentScreenY,
    contentScreenWidth: record.contentScreenWidth,
    contentScreenHeight: record.contentScreenHeight,
  }
}

describe('scene projection', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('agrees with the screen geometry main stamps on every entity kind', async () => {
    harness.loadFixture({
      name: 'Projection',
      doc: {
        nodes: [
          {
            id: PAGE_ID,
            type: 'link',
            x: 120,
            y: 140,
            width: 375,
            height: 667,
            url: 'https://example.com/',
            presetIndex: 0,
          },
        ],
        edges: [],
        appState: { zoom: 1, pan: { x: 0, y: 0 } },
      },
    })

    const created = applyCanvasPatch({
      entities: [
        { kind: 'text', canvasX: 17, canvasY: 23, width: 200, height: 100, text: 'projected' },
        { kind: 'shape', canvasX: 311, canvasY: 29, width: 120, height: 80, shapeKind: 'rectangle' },
        {
          kind: 'drawing',
          canvasX: 7, canvasY: 313, width: 150, height: 150,
          strokes: [{ id: 's1', color: '#f00', width: 2, points: [{ x: 0, y: 0 }, { x: 9, y: 9 }] }],
        },
        { kind: 'file', canvasX: 307, canvasY: 311, file: 'notes/a.md', width: 220, height: 180 },
        { kind: 'file', canvasX: 703, canvasY: 311, file: 'notes/b.md', width: 220, height: 180 },
      ],
    })
    const [textId, , , plainFileId, shelledFileId] = created.created
    applyCanvasPatch({ entities: [{ kind: 'group', entityIds: [textId, plainFileId], label: 'Pair' }] })

    // Both shell conventions in one scene: a page anchors at its bezel and
    // insets the body inward, a file entity anchors at its body and grows the
    // shell outward.
    setPagePreset(PAGE_ID, 1)
    toggleDeviceShell(PAGE_ID)
    toggleFileDeviceShell(shelledFileId)

    // Fractional on both axes and both terms — whole numbers would hide a
    // rounding disagreement, which is the one the page path can actually have.
    setZoom(1.37)
    setPan(-83.5, 47.25)
    await settleSync()

    const layout = getCanvasLayoutData()
    const camera = { zoom: layout.zoom, pan: layout.pan }
    expect(layout.entities.map((entity) => entity.kind).sort()).toEqual([
      'drawing',
      'file',
      'file',
      'group',
      'page',
      'shape',
      'text',
    ])
    const shelled = layout.entities.filter(
      (entity) =>
        (entity.kind === 'page' || entity.kind === 'file') && entity.showDeviceFrame === true,
    )
    expect(shelled).toHaveLength(2)

    for (const entity of layout.entities) {
      expect({ id: entity.id, ...screenFieldsOf(entity) }).toEqual({
        id: entity.id,
        ...screenFieldsOf(projectSceneEntity(entity, camera, layout.canvasOrigin)),
      })
    }
  })
})
