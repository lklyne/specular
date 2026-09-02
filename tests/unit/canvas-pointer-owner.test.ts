import { describe, expect, it } from 'vitest'
import {
  annotationOverlayActive,
  canvasPointerOwner,
  type CanvasPointerOwner,
  type CanvasPointerOwnerState,
} from '../../src/shared/canvas-pointer-owner'
import type { ToolKind } from '../../src/shared/tool'

const TOOL_KINDS: ToolKind[] = [
  'select',
  'hand',
  'add-page',
  'add-text',
  'add-sticky',
  'add-shape',
  'comment',
  'draw',
  'inspect',
]

function state(over: Partial<CanvasPointerOwnerState> = {}): CanvasPointerOwnerState {
  return {
    toolKind: 'select',
    pendingPlacement: false,
    pendingAnnotation: false,
    pendingRegionRect: false,
    drawingSession: false,
    ...over,
  }
}

/**
 * Oracle: the arbitration booleans this selector replaced, verbatim from
 * App.tsx at the time of the collapse. `canvasPointerOwner` must agree with
 * them on every cell — the selector encodes today's policy, it does not
 * design a new one. (A focused thread is not a dimension: its conversation
 * lives in the right panel and its canvas trace is a passive ring.)
 */
function legacyOwner(s: CanvasPointerOwnerState): CanvasPointerOwner {
  const isAnnotationTool = s.toolKind === 'comment' || s.toolKind === 'draw'
  const overlayInteractive =
    s.pendingAnnotation ||
    s.pendingRegionRect ||
    s.drawingSession ||
    s.toolKind === 'draw'
  const routerOwnsCanvasPointers =
    !overlayInteractive && !s.pendingPlacement && !isAnnotationTool
  const commentToolBlocked = s.drawingSession || s.toolKind === 'draw'
  const skipPointerCapture =
    s.toolKind === 'comment' ? commentToolBlocked : overlayInteractive
  const toolGestureActive =
    !skipPointerCapture && (s.pendingPlacement || s.toolKind === 'comment')
  if (routerOwnsCanvasPointers) return 'router'
  if (toolGestureActive) return 'tool-gesture'
  return 'annotation-overlay'
}

describe('canvasPointerOwner', () => {
  // Representative rows of the ownership matrix. Page focus is not a
  // dimension: entering a page changes what the router dispatches
  // (forward-pointer-down), never who owns the pointerdown — the legacy
  // booleans had no focus input and neither does the selector's state.
  const matrix: Array<{
    name: string
    state: CanvasPointerOwnerState
    owner: CanvasPointerOwner
  }> = [
    { name: 'select tool, idle', state: state(), owner: 'router' },
    { name: 'hand tool, idle', state: state({ toolKind: 'hand' }), owner: 'router' },
    { name: 'inspect tool, idle', state: state({ toolKind: 'inspect' }), owner: 'router' },
    {
      name: 'placement tool with a pending placement',
      state: state({ toolKind: 'add-shape', pendingPlacement: true }),
      owner: 'tool-gesture',
    },
    {
      name: 'placement tool before its broadcast lands',
      state: state({ toolKind: 'add-shape' }),
      owner: 'router',
    },
    { name: 'comment tool, idle', state: state({ toolKind: 'comment' }), owner: 'tool-gesture' },
    {
      name: 'comment tool with its composer open (retargeting stays live)',
      state: state({ toolKind: 'comment', pendingAnnotation: true }),
      owner: 'tool-gesture',
    },
    {
      name: 'comment tool with a pending region rect (retargeting stays live)',
      state: state({ toolKind: 'comment', pendingRegionRect: true }),
      owner: 'tool-gesture',
    },
    {
      name: 'comment tool while a drawing stroke is in flight',
      state: state({ toolKind: 'comment', drawingSession: true }),
      owner: 'annotation-overlay',
    },
    { name: 'draw tool', state: state({ toolKind: 'draw' }), owner: 'annotation-overlay' },
    {
      name: 'draw tool with an active stroke',
      state: state({ toolKind: 'draw', drawingSession: true }),
      owner: 'annotation-overlay',
    },
    {
      name: 'select tool with a composer open',
      state: state({ pendingAnnotation: true }),
      owner: 'annotation-overlay',
    },
  ]

  for (const row of matrix) {
    it(`${row.name} → ${row.owner}`, () => {
      expect(canvasPointerOwner(row.state)).toBe(row.owner)
    })
  }

  it('a pointerdown on [data-overlay-ui] is owned by nobody, whatever the state (I8′)', () => {
    for (const row of matrix) {
      expect(canvasPointerOwner({ ...row.state, overlayUiTarget: true })).toBe('none')
    }
  })

  it('agrees with the legacy arbitration booleans on every cell', () => {
    for (const toolKind of TOOL_KINDS) {
      for (let bits = 0; bits < 16; bits++) {
        const s = state({
          toolKind,
          pendingPlacement: Boolean(bits & 1),
          pendingAnnotation: Boolean(bits & 2),
          pendingRegionRect: Boolean(bits & 4),
          drawingSession: Boolean(bits & 8),
        })
        expect(canvasPointerOwner(s), JSON.stringify(s)).toBe(legacyOwner(s))
      }
    }
  })

  it('the root surface stays interactive in every state (I7)', () => {
    // Legacy: overlayInteractive || routerOwns || toolGestureOwns gated the
    // root's pointer-events class — provably constant-true, so App renders
    // pointer-events-auto unconditionally. The sweep documents that no
    // state leaves canvas pointerdowns without an owner.
    for (const toolKind of TOOL_KINDS) {
      for (let bits = 0; bits < 16; bits++) {
        const s = state({
          toolKind,
          pendingPlacement: Boolean(bits & 1),
          pendingAnnotation: Boolean(bits & 2),
          pendingRegionRect: Boolean(bits & 4),
          drawingSession: Boolean(bits & 8),
        })
        expect(canvasPointerOwner(s)).not.toBe('none')
      }
    }
  })
})

describe('annotationOverlayActive', () => {
  it('mirrors the legacy overlayInteractive boolean', () => {
    for (const toolKind of TOOL_KINDS) {
      for (let bits = 0; bits < 8; bits++) {
        const s = state({
          toolKind,
          pendingAnnotation: Boolean(bits & 1),
          pendingRegionRect: Boolean(bits & 2),
          drawingSession: Boolean(bits & 4),
        })
        const legacy =
          s.pendingAnnotation ||
          s.pendingRegionRect ||
          s.drawingSession ||
          s.toolKind === 'draw'
        expect(annotationOverlayActive(s)).toBe(legacy)
      }
    }
  })
})
