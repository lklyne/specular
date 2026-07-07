import { createElement } from 'react'
import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import { focusContext } from '../../src/shared/focus-context'
import { CanvasItemPopup } from '../../src/renderer/above-view/CanvasItemPopup'
import { EMPTY_LAYOUT } from '../../src/renderer/canvas-bg/canvasBgConstants'
import {
  FOCUS_PRESENTATION_MENU_EDGE_INSET_PX,
  FOCUS_PRESENTATION_MENU_INSET,
} from '../../src/shared/featureFlags'
import type { CanvasScenePageEntity, LayoutUpdateData } from '../../src/shared/types'

function page(id: string, x: number): CanvasScenePageEntity {
  return {
    kind: 'page',
    id,
    label: id,
    url: 'https://example.com',
    canGoBack: false,
    canGoForward: false,
    isLoading: false,
    isCustomSize: false,
    canvasX: x,
    canvasY: 0,
    width: 300,
    height: 200,
    presetIndex: 0,
    synced: false,
    screenX: x,
    screenY: 100,
    screenWidth: 300,
    screenHeight: 200,
  }
}

function focusedLayout(annotationsVisible = false): LayoutUpdateData {
  const focused = page('focused', 100)
  const other = page('other', 500)
  return {
    ...EMPTY_LAYOUT,
    windowWidth: 1200,
    entityOrder: [focused.id, other.id],
    entities: [focused, other],
    selectedEntityIds: [focused.id],
    focusPresentation: {
      pageId: focused.id,
      mode: 'fit',
      authoredLabel: 'Desktop',
      authoredWidth: 300,
      authoredHeight: 200,
      effectiveWidth: 300,
      effectiveHeight: 200,
      annotationsVisible,
    },
  }
}

function cssLength(value: number): string {
  return value === 0 ? '0' : `${value}px`
}

describe('focus mode rendering', () => {
  it('hides context at rest and shows it once the eye is on (ADR 0021)', () => {
    // showsContext gates all surrounding context (other pages, annotations,
    // groups) — binary show/hide, never a dim.
    expect(focusContext(focusedLayout(false)).showsContext).toBe(false)
    expect(focusContext(focusedLayout(true)).showsContext).toBe(true)
    // Outside a focus session everything always renders.
    expect(focusContext(EMPTY_LAYOUT).showsContext).toBe(true)
    expect(focusContext(EMPTY_LAYOUT).pageId).toBeNull()
  })

  it('exposes the focused page id used by App to suppress selection chrome', () => {
    expect(focusContext(focusedLayout()).pageId).toBe('focused')
  })

  it('derives showsContext from the eye, not the active tool (ADR 0021)', () => {
    // Annotation visibility is latched session state; the selector reads it
    // straight from the broadcast, independent of whatever tool is active.
    const drawing = { ...focusedLayout(false), activeTool: { kind: 'draw' as const } }
    expect(focusContext(drawing).showsContext).toBe(false)
  })

  it('renders the focused action menu with the configured viewport-top layout', () => {
    const inset = FOCUS_PRESENTATION_MENU_INSET
      ? FOCUS_PRESENTATION_MENU_EDGE_INSET_PX
      : 0
    const html = renderToStaticMarkup(
      createElement(
        CanvasItemPopup.Root,
        {
          entityId: 'focused',
          layout: focusedLayout(),
          open: true,
          placement: 'viewport-top',
        },
        createElement(
          CanvasItemPopup.Frame,
          {
            isDark: false,
            flush: !FOCUS_PRESENTATION_MENU_INSET,
            fullWidth: true,
          },
          createElement('span', null, 'Actions'),
        ),
      ),
    )

    expect(html).toContain('data-popup-placement="viewport-top"')
    expect(html).toContain(`left:${cssLength(inset)}`)
    expect(html).toContain(`top:${cssLength(inset)}`)
    expect(html).toContain(`width:${cssLength(1200 - inset * 2)}`)
    expect(html).toContain('transform:none')
    expect(html).toContain('data-popup-frame-content')
    expect(html).toContain(
      FOCUS_PRESENTATION_MENU_INSET
        ? 'data-popup-frame="floating"'
        : 'data-popup-frame="flush"',
    )
    expect(html).toContain(FOCUS_PRESENTATION_MENU_INSET ? 'rounded-[10px]' : 'rounded-none')
    if (!FOCUS_PRESENTATION_MENU_INSET) {
      expect(html).toContain('box-shadow:none')
    }
  })

  it('sizes the focus menu between left chrome and open devtools', () => {
    const layout = {
      ...focusedLayout(),
      leftChromeWidth: 320,
      devtoolsOpen: true,
      devtoolsWidth: 400,
    }
    const html = renderToStaticMarkup(
      createElement(
        CanvasItemPopup.Root,
        {
          entityId: 'focused',
          layout,
          open: true,
          placement: 'viewport-top',
        },
        createElement(
          CanvasItemPopup.Frame,
          {
            isDark: false,
            flush: !FOCUS_PRESENTATION_MENU_INSET,
            fullWidth: true,
          },
          createElement('span', null, 'Actions'),
        ),
      ),
    )

    const inset = FOCUS_PRESENTATION_MENU_INSET
      ? FOCUS_PRESENTATION_MENU_EDGE_INSET_PX
      : 0
    expect(html).toContain(`left:${cssLength(layout.leftChromeWidth + inset)}`)
    expect(html).toContain(
      `width:${cssLength(layout.windowWidth - layout.devtoolsWidth - layout.leftChromeWidth - inset * 2)}`,
    )
  })

  it('does not subtract stale devtools width when the right panel is closed', () => {
    const layout = {
      ...focusedLayout(),
      leftChromeWidth: 320,
      devtoolsOpen: false,
      devtoolsWidth: 400,
    }
    const html = renderToStaticMarkup(
      createElement(
        CanvasItemPopup.Root,
        {
          entityId: 'focused',
          layout,
          open: true,
          placement: 'viewport-top',
        },
        createElement(
          CanvasItemPopup.Frame,
          {
            isDark: false,
            flush: !FOCUS_PRESENTATION_MENU_INSET,
            fullWidth: true,
          },
          createElement('span', null, 'Actions'),
        ),
      ),
    )

    const inset = FOCUS_PRESENTATION_MENU_INSET
      ? FOCUS_PRESENTATION_MENU_EDGE_INSET_PX
      : 0
    expect(html).toContain(`left:${cssLength(layout.leftChromeWidth + inset)}`)
    expect(html).toContain(
      `width:${cssLength(layout.windowWidth - layout.leftChromeWidth - inset * 2)}`,
    )
  })
})
