import { createElement } from 'react'
import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import { GroupBoundsLayer } from '../../src/renderer/above-view/GroupBoundsLayer'
import { GroupBackgroundLayer } from '../../src/renderer/canvas-bg/GroupBackgroundLayer'
import type { CanvasSceneGroupEntity } from '../../src/shared/types'

function group(overrides: Partial<CanvasSceneGroupEntity> = {}): CanvasSceneGroupEntity {
  return {
    kind: 'group',
    id: 'g1',
    label: 'Group',
    canvasX: 100,
    canvasY: 120,
    width: 300,
    height: 180,
    screenX: 140,
    screenY: 160,
    screenWidth: 360,
    screenHeight: 216,
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
      }),
    )

    expect(html).toContain('background:')
    expect(html).toContain('left:140px')
    expect(html).toContain('top:160px')
  })
})
