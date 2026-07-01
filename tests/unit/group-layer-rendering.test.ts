import { createElement } from 'react'
import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import { GroupBoundsLayer } from '../../src/renderer/above-view/GroupBoundsLayer'
import { GroupBackgroundLayer } from '../../src/renderer/canvas-bg/GroupBackgroundLayer'
import type { Camera } from '../../src/shared/coords'
import type { CanvasSceneGroupEntity } from '../../src/shared/types'

// Identity camera: the group's canvas rect projects 1:1 to screen space.
const camera: Camera = { pan: { x: 0, y: 0 }, zoom: 1, canvasOrigin: { x: 0, y: 0 } }

function group(overrides: Partial<CanvasSceneGroupEntity> = {}): CanvasSceneGroupEntity {
  return {
    kind: 'group',
    id: 'g1',
    label: 'Group',
    canvasX: 140,
    canvasY: 160,
    width: 360,
    height: 216,
    layoutMode: 'freeform',
    managedLayout: false,
    entityIds: [],
    ...overrides,
  }
}

describe('group layer rendering', () => {
  it('keeps the above-page group layer border-only', () => {
    const html = renderToStaticMarkup(
      createElement(GroupBoundsLayer, {
        groups: [group()],
        isDark: false,
        zoom: 1,
        canvasOrigin: { x: 0, y: 0 },
        pan: { x: 0, y: 0 },
      }),
    )

    expect(html).toContain('border:')
    expect(html).not.toContain('background:')
  })

  it('renders group backgrounds in the bottom canvas layer', () => {
    const html = renderToStaticMarkup(
      createElement(GroupBackgroundLayer, {
        groups: [group()],
        isDark: false,
        camera,
      }),
    )

    expect(html).toContain('background:')
    expect(html).toContain('left:140px')
    expect(html).toContain('top:160px')
  })
})
