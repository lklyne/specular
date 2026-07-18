import { describe, expect, it } from 'vitest'
import { buildPendingPlacementPreview } from '../../src/renderer/canvas-bg/canvasBgSelectors'
import { PlacementPreviewLayer } from '../../src/renderer/canvas-bg/CanvasGridSurface'
import { EMPTY_LAYOUT } from '../../src/renderer/canvas-bg/canvasBgConstants'
import { PLAIN_TEXT_PLACEHOLDER } from '../../src/shared/constants'

describe('placement preview', () => {
  it('preserves text style while snapping the pending text preview', () => {
    const preview = buildPendingPlacementPreview(
      {
        ...EMPTY_LAYOUT,
        zoom: 2,
        canvasOrigin: { x: 10, y: 44 },
        pan: { x: 3, y: 5 },
        pendingPlacement: {
          entityKind: 'text',
          textStyle: 'plain',
          width: 200,
          height: 200,
        },
      },
      { clientX: 54, clientY: 97 },
    )

    expect(preview).toMatchObject({
      entityKind: 'text',
      textStyle: 'plain',
      left: 53,
      top: 89,
      width: 400,
      height: 400,
    })
  })

  it('renders plain text placement as an Add text placeholder, not a sticky box', () => {
    const element = PlacementPreviewLayer({
      isDark: false,
      preview: {
        entityKind: 'text',
        textStyle: 'plain',
        left: 20,
        top: 30,
        width: 200,
        height: 200,
      },
    }) as { props: { className: string; children: string; style: Record<string, unknown> } }

    expect(element.props.children).toBe(PLAIN_TEXT_PLACEHOLDER)
    expect(element.props.className).not.toContain('border')
    expect(element.props.style).toMatchObject({ left: 20, top: 30 })
    expect(element.props.style).not.toHaveProperty('background')
  })

  it('keeps generic placement placeholders square', () => {
    const element = PlacementPreviewLayer({
      isDark: false,
      preview: {
        entityKind: 'page',
        left: 20,
        top: 30,
        width: 200,
        height: 120,
      },
    }) as { props: { className: string } }

    expect(element.props.className).not.toContain('rounded')
  })
})
