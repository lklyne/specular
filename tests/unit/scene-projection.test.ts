/**
 * `projectToScreen` and friends are the renderer's copy of the formulas
 * `buildCanvasLayoutData` uses to stamp `screen*` onto every scene entity.
 * Both sides project the same geometry, so a disagreement puts DOM chrome at a
 * different place than the native page view it is supposed to hug.
 *
 * These cases pin the projection against the builders directly: the same page
 * and file entity are projected here and by main, and the two must match field
 * for field. The device-shell cases are the ones worth spelling out — pages
 * anchor at the bezel and inset inward, file entities anchor at the body and
 * grow outward, and only one of the two rounds.
 *
 * Mutation-verified by dropping the `Math.round` from `snapTop` in
 * `projectPageToScreen` (the page cases fail on a fractional pan) and by
 * flipping `outsetByShell` to `insetByShell` in `projectFileToScreen` (the
 * file shell case fails).
 */

import { describe, expect, it } from 'vitest'
import {
  CARD_BORDER_WIDTH,
  insetByShell,
  outsetByShell,
  projectFileToScreen,
  projectPageToScreen,
  projectSceneEntity,
  projectToScreen,
  unprojectFromScreen,
} from '../../src/shared/scene-projection'
import { CUSTOM_SHELL_INSETS, shellInsetsForDevice } from '../../src/shared/device-catalog'

const camera = { zoom: 1.5, pan: { x: 40, y: -25 } }
const origin = { x: 0, y: 44 }

describe('projectToScreen', () => {
  it('places a canvas rect at origin + canvas * zoom + pan', () => {
    expect(projectToScreen({ x: 100, y: 200, width: 300, height: 400 }, camera, origin)).toEqual({
      x: 0 + 100 * 1.5 + 40,
      y: 44 + 200 * 1.5 - 25,
      width: 450,
      height: 600,
    })
  })

  it('round-trips through unprojectFromScreen', () => {
    const rect = { x: -37.5, y: 118.25, width: 640, height: 480 }
    const back = unprojectFromScreen(projectToScreen(rect, camera, origin), camera, origin)
    expect(back.x).toBeCloseTo(rect.x, 9)
    expect(back.y).toBeCloseTo(rect.y, 9)
    expect(back.width).toBeCloseTo(rect.width, 9)
    expect(back.height).toBeCloseTo(rect.height, 9)
  })
})

describe('projectPageToScreen', () => {
  const page = { canvasX: 120.5, canvasY: 80.25, width: 390, height: 844 }

  it('rounds to whole pixels and adds the scene origin after the round', () => {
    const { shell, content } = projectPageToScreen(page, camera, origin)
    expect(content).toEqual({
      x: Math.round(120.5 * 1.5 + 40),
      y: 44 + Math.round(80.25 * 1.5 - 25),
      width: Math.round(390 * 1.5),
      height: Math.round(844 * 1.5),
    })
    // Without a shell the outer rect is the body plus the 1px card border.
    expect(shell).toEqual({
      x: content.x - CARD_BORDER_WIDTH,
      y: content.y - CARD_BORDER_WIDTH,
      width: content.width + 2 * CARD_BORDER_WIDTH,
      height: content.height + 2 * CARD_BORDER_WIDTH,
    })
  })

  it('anchors a shelled page at the bezel and insets the body inward', () => {
    const shelled = {
      ...page,
      showDeviceFrame: true,
      deviceId: 'iphone-14-pro',
      deviceOrientation: 'portrait' as const,
    }
    const insets = shellInsetsForDevice('iphone-14-pro', 'portrait')
    const { shell, content } = projectPageToScreen(shelled, camera, origin)

    // The shell sits where the unshelled body would: canvasX/canvasY is the
    // bezel's top-left for a page.
    const unshelled = projectPageToScreen(page, camera, origin)
    expect(shell.x).toBe(unshelled.content.x)
    expect(shell.y).toBe(unshelled.content.y)

    expect(content.x).toBe(shell.x + Math.round(insets.left * camera.zoom))
    expect(content.y).toBe(shell.y + Math.round(insets.top * camera.zoom))
    expect(shell.width).toBe(
      content.width + Math.round(insets.left * camera.zoom) + Math.round(insets.right * camera.zoom),
    )
    expect(shell.height).toBe(
      content.height + Math.round(insets.top * camera.zoom) + Math.round(insets.bottom * camera.zoom),
    )
  })

  it('uses the custom shell insets when a framed page names no device', () => {
    const { shell, content } = projectPageToScreen(
      { ...page, showDeviceFrame: true, deviceId: null },
      camera,
      origin,
    )
    expect(content.x - shell.x).toBe(Math.round(CUSTOM_SHELL_INSETS.left * camera.zoom))
  })
})

describe('projectFileToScreen', () => {
  const file = { canvasX: 10, canvasY: 20, width: 200, height: 100 }

  it('leaves the shell equal to the body when no frame is shown', () => {
    const { shell, content } = projectFileToScreen(file, camera, origin)
    expect(shell).toEqual(content)
    expect(content).toEqual(projectToScreen({ ...file, x: 10, y: 20 }, camera, origin))
  })

  it('anchors a shelled file at the body and grows the shell outward', () => {
    const shelled = { ...file, showDeviceFrame: true, deviceId: null }
    const { shell, content } = projectFileToScreen(shelled, camera, origin)
    expect(content).toEqual(projectFileToScreen(file, camera, origin).content)
    expect(shell).toEqual(outsetByShell(content, CUSTOM_SHELL_INSETS, camera.zoom))
    expect(insetByShell(shell, CUSTOM_SHELL_INSETS, camera.zoom)).toEqual(content)
  })
})

describe('projectSceneEntity', () => {
  it('reports a page body rect whether or not the bezel is drawn', () => {
    // Bezel off: the outer rect is the body itself, not the card-border box —
    // the border is drawn around the body, so chrome that hugs `screen*` sits
    // flush against the native view.
    const plain = projectSceneEntity(
      { kind: 'page', canvasX: 0, canvasY: 0, width: 100, height: 100 },
      camera,
      origin,
    )
    expect(plain.contentScreenX).toBe(plain.screenX)
    expect(plain.contentScreenWidth).toBe(plain.screenWidth)

    const shelled = projectSceneEntity(
      {
        kind: 'page',
        canvasX: 0,
        canvasY: 0,
        width: 100,
        height: 100,
        showDeviceFrame: true,
        deviceId: null,
      },
      camera,
      origin,
    )
    expect(shelled.contentScreenX).toBe(
      shelled.screenX + Math.round(CUSTOM_SHELL_INSETS.left * camera.zoom),
    )
  })

  it('omits a file body rect when the bezel is off', () => {
    const plain = projectSceneEntity(
      { kind: 'file', canvasX: 0, canvasY: 0, width: 100, height: 100 },
      camera,
      origin,
    )
    expect(plain.contentScreenX).toBeUndefined()
    expect(plain.screenWidth).toBe(150)
  })

  it('projects the unshelled kinds straight through', () => {
    for (const kind of ['text', 'shape', 'drawing', 'group']) {
      expect(
        projectSceneEntity({ kind, canvasX: 8, canvasY: 12, width: 40, height: 60 }, camera, origin),
      ).toEqual({
        screenX: 8 * 1.5 + 40,
        screenY: 44 + 12 * 1.5 - 25,
        screenWidth: 60,
        screenHeight: 90,
      })
    }
  })
})
