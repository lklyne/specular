import { describe, expect, it } from 'vitest'
import {
  createOptionDragCopySession,
  type DragCopyPreviewBox,
} from '../../src/renderer/above-view/optionDragCopy'
import type { LayoutUpdateData } from '../../src/shared/types'

function layout(): LayoutUpdateData {
  return {
    zoom: 1,
    canvasOrigin: { x: 0, y: 40 },
    pan: { x: 200, y: 20 },
    selectedEntityIds: ['t1'],
    selectedGroupId: null,
    entities: [
      {
        id: 't1',
        kind: 'text',
        canvasX: 100,
        canvasY: 200,
        width: 160,
        height: 80,
        screenX: 300,
        screenY: 260,
        screenWidth: 160,
        screenHeight: 80,
        text: 'note',
      },
    ],
    edges: [],
    groups: [],
    annotations: [],
    presenceCursors: [],
    interaction: { kind: 'idle' },
    hover: null,
    activeTool: { kind: 'select' },
  } as LayoutUpdateData
}

describe('createOptionDragCopySession', () => {
  it('resets the origin drag and shows a ghost when option is pressed mid-drag', () => {
    let optionHeld = false
    const deltas: Array<[number, number]> = []
    const previews: DragCopyPreviewBox[][] = []
    const copies: Array<{ canvasX: number; canvasY: number }> = []
    let ended = 0

    const session = createOptionDragCopySession({
      layout: layout(),
      entityIds: ['t1'],
      anchorEntityId: 't1',
      startScreenX: 0,
      startScreenY: 0,
      isOptionHeld: () => optionHeld,
      applyDelta: (dx, dy) => deltas.push([dx, dy]),
      previewDelta: () => undefined,
      setPreview: (preview) => previews.push(preview),
      endDrag: () => ended += 1,
      copyAt: (canvasX, canvasY) => copies.push({ canvasX, canvasY }),
    })

    session.move({ screenX: 30, screenY: 10, altKey: false })
    optionHeld = true
    session.setOptionHeld(true)
    session.finish({ screenX: 30, screenY: 10, altKey: true })

    expect(deltas).toEqual([[30, 10], [-30, -10]])
    expect(previews.at(-2)).toMatchObject([
      { id: 't1', left: 340, top: 240, width: 160, height: 80 },
    ])
    expect(ended).toBe(1)
    expect(copies).toEqual([{ canvasX: 140, canvasY: 220 }])
  })

  it('applies the full drag to the origin again when option is released before mouseup', () => {
    let optionHeld = true
    const deltas: Array<[number, number]> = []
    const copies: Array<{ canvasX: number; canvasY: number }> = []

    const session = createOptionDragCopySession({
      layout: layout(),
      entityIds: ['t1'],
      anchorEntityId: 't1',
      startScreenX: 0,
      startScreenY: 0,
      isOptionHeld: () => optionHeld,
      applyDelta: (dx, dy) => deltas.push([dx, dy]),
      previewDelta: () => undefined,
      setPreview: () => undefined,
      endDrag: () => undefined,
      copyAt: (canvasX, canvasY) => copies.push({ canvasX, canvasY }),
    })

    session.move({ screenX: 40, screenY: 20, altKey: true })
    optionHeld = false
    session.setOptionHeld(false)
    session.finish({ screenX: 40, screenY: 20, altKey: false })

    expect(deltas).toEqual([[40, 20]])
    expect(copies).toEqual([])
  })
})
