/**
 * A 'fill' focus session places its page through the camera, like any other
 * page. The page must land exactly on `focusFillRegion()` — flush under the
 * focus bar, edge to edge — or its top hides beneath the bar.
 *
 * Mutation-verified by: dropping `- TOOLBAR_HEIGHT` from
 * `focusFillFrameBounds`, or dropping its shell insets — both cases fail
 * either way (a preset page carries a device frame by default).
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { applyCanvasPatch } from '../../src/main/canvas-apply'
import { findPageById } from '../../src/main/runtime/runtime-context'
import {
  boundScreenBoundsForPage,
  focusFillRegion,
} from '../../src/main/runtime/runtime-geometry'
import { selectPageById } from '../../src/main/runtime/selection-controller'
import {
  cancelCameraAnimation,
  focusSelection,
  recenterFocusPresentation,
  setFocusPresentationMode,
} from '../../src/main/runtime/viewport-control'

let harness: WorkspaceHarness

function fillFocusedPageRect(entity: Record<string, unknown>) {
  const pageId = applyCanvasPatch({
    entities: [{ kind: 'page', url: 'https://example.com/', canvasX: 340, canvasY: 120, ...entity }],
  }).created[0]
  selectPageById(pageId)
  focusSelection({ animate: false })
  setFocusPresentationMode('fill')
  // The mode switch tweens; seat the camera at its target instead.
  recenterFocusPresentation(undefined, { animate: false })
  const page = findPageById(pageId)
  if (!page) throw new Error('page not created')
  return boundScreenBoundsForPage(page).page
}

describe('fill focus placement', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    cancelCameraAnimation()
  })

  afterAll(() => {
    cancelCameraAnimation()
    harness?.dispose()
  })

  it('lands the page on the fill region, below the focus bar', () => {
    expect(fillFocusedPageRect({ presetIndex: 2 })).toEqual(focusFillRegion())
  })

  it('lands a device-framed page body on the fill region, bezel off-screen', () => {
    expect(
      fillFocusedPageRect({ deviceId: 'iphone-15-pro', showDeviceFrame: true }),
    ).toEqual(focusFillRegion())
  })
})
