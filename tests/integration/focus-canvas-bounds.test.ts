/**
 * Universal canvas-bound reveal behavior at the real viewport-control seam.
 *
 * A focus request for an already-visible item is a no-op. An off-camera item
 * starts the shared spring transition instead of changing pan synchronously.
 *
 * Mutation-verified by removing the visibility guard from `focusCanvasBounds`
 * (the visible case starts a transition), or by changing its default animation
 * to false (the off-camera case changes pan immediately).
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import {
  cameraTransitionStartedAt,
  pan,
} from '../../src/main/runtime/runtime-context'
import {
  cancelCameraAnimation,
  focusCanvasBounds,
} from '../../src/main/runtime/viewport-control'

let harness: WorkspaceHarness

describe('focusCanvasBounds', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    cancelCameraAnimation()
  })

  afterAll(() => {
    cancelCameraAnimation()
    harness?.dispose()
  })

  it('does not move the camera when the target is already fully visible', () => {
    const before = { ...pan }

    focusCanvasBounds({ x: 400, y: 100, width: 200, height: 120 })

    expect(pan).toEqual(before)
    expect(cameraTransitionStartedAt).toBeNull()
  })

  it('starts a spring transition without jumping when the target is off camera', () => {
    const before = { ...pan }

    focusCanvasBounds({ x: 3000, y: 1800, width: 200, height: 120 })

    expect(pan).toEqual(before)
    expect(cameraTransitionStartedAt).not.toBeNull()
  })
})
