// ADR 0008 — unified canvas-item popup compound component.

import {
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent,
  type ReactNode,
} from 'react'
import { Columns2, Grid2x2, Maximize2, MessageCircle, Rows2 } from 'lucide-react'
import { TOOLBAR_HEIGHT } from '../../shared/constants'
import { POPUP_SURFACE_CLASS, popupSurfaceStyle } from '../shared/popupSurface'
import { Tooltip } from '../shared/Tooltip'
import { swatchDotShadow, swatchRingShadow } from './colorSwatchStyle'
import { focusContext } from '../../shared/focus-context'
import {
  CAMERA_SPRING_CSS_EASING,
  DEFAULT_CAMERA_TRANSITION_DURATION_MS,
} from '../../shared/camera-transition'
import {
  paletteSlots,
  resolveCanvasColor,
  type CanvasColorRole,
  type CanvasColorSlot,
  type CanvasPalette,
} from '../../shared/canvas-colors'
import type { Rect } from '../../shared/hit-regions'
import type { BatchLayoutMode, LayoutUpdateData, WorkspaceBounds } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { selectionAnnotationBounds } from './annotationMath'
import type { AnnotateHandler } from './annotationMath'
import {
  useAnchoredPosition,
  useMultiAnchoredPosition,
} from './useAnchoredPosition'

type Placement = 'above' | 'below' | 'overlay' | 'viewport-top'
type Align = 'stretch' | 'center'

type RootProps = {
  layout: LayoutUpdateData
  open: boolean
  placement?: Placement
  align?: Align
  offset?: number
  children: ReactNode
} & (
  | { entityId: string; entityIds?: undefined }
  | { entityIds: readonly string[]; entityId?: undefined }
)

function Root(props: RootProps) {
  const {
    layout,
    open,
    placement = 'below',
    align = 'center',
    offset = 8,
    children,
  } = props
  const singleRect = useAnchoredPosition(layout, props.entityId ?? '')
  const multiRect = useMultiAnchoredPosition(layout, props.entityIds ?? [])
  const rect = props.entityIds !== undefined ? multiRect : singleRect
  const anchorVisible =
    rect === null ||
    placement === 'overlay' ||
    placement === 'viewport-top' ||
    anchorIntersectsCanvas(rect, layout)
  const mounted = open && rect !== null && anchorVisible
  const popupMotion = usePopupFlipAnimation({
    active: mounted,
    placement,
    cameraTransitionStartedAt: layout.cameraTransitionStartedAt,
  })
  const frameSize = useMeasuredFrameSize(popupMotion.layoutRef, mounted)
  if (!mounted || !rect) return null
  const style = popupStyle(rect, placement, align, offset, layout, frameSize)
  return (
    <div
      ref={popupMotion.layoutRef}
      data-overlay-ui
      data-viewport-passthrough
      data-popup-placement={placement}
      className="pointer-events-auto absolute"
      // z-index above EdgeLayer (5) so edges don't paint over canvas-item toolbars.
      style={{ zIndex: 20, ...style }}
    >
      <div ref={popupMotion.motionRef}>{children}</div>
    </div>
  )
}

const POPUP_EDGE_MARGIN = 8

// Canvas span not covered by the left sidebar or the devtools panel, in
// window/overlay x coords.
function canvasHorizontalBounds(layout: LayoutUpdateData): { left: number; right: number } {
  const rightInset = layout.devtoolsOpen ? layout.devtoolsWidth : 0
  return {
    left: layout.leftChromeWidth,
    right: Math.max(0, layout.windowWidth - rightInset),
  }
}

// Page-anchored popups stick to the canvas edges while any part of the anchor
// is visible, and unmount once it scrolls fully past a panel or window edge.
function anchorIntersectsCanvas(rect: Rect, layout: LayoutUpdateData): boolean {
  const bounds = canvasHorizontalBounds(layout)
  // Overlay-local y: 0 is the toolbar's bottom edge.
  const overlayHeight = window.innerHeight - layout.canvasOrigin.y
  return (
    rect.x < bounds.right &&
    rect.x + rect.width > bounds.left &&
    rect.y < overlayHeight &&
    rect.y + rect.height > 0
  )
}

// Measures the visible popup frame so popupStyle can clamp an explicit
// position (a CSS-only clamp can't know the rendered size). Size updates flow
// through state, but the frame is stable outside camera transitions so this
// rarely re-renders.
function useMeasuredFrameSize(
  hostRef: { current: HTMLDivElement | null },
  active: boolean,
): { width: number; height: number } | null {
  const [size, setSize] = useState<{ width: number; height: number } | null>(null)
  useLayoutEffect(() => {
    if (!active) {
      setSize(null)
      return
    }
    const frame = hostRef.current?.querySelector<HTMLElement>('[data-popup-frame]')
    if (!frame) return
    const measure = () => {
      const box = frame.getBoundingClientRect()
      setSize((prev) =>
        prev && prev.width === box.width && prev.height === box.height
          ? prev
          : { width: box.width, height: box.height },
      )
    }
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(frame)
    return () => observer.disconnect()
  }, [hostRef, active])
  return size
}

// Morph the toolbar between its page-anchored placement and the focused
// viewport-top bar. Height is constant across placements, so we animate width
// directly (no transform scale — that distorts the flex row) plus a translate
// for the position delta. The width tween reflows the flex children naturally,
// so glyphs stay undistorted without any counter-scaling.
function usePopupFlipAnimation({
  active,
  placement,
  cameraTransitionStartedAt,
}: {
  active: boolean
  placement: Placement
  cameraTransitionStartedAt: number | null
}) {
  const layoutRef = useRef<HTMLDivElement | null>(null)
  const motionRef = useRef<HTMLDivElement | null>(null)
  const previousRectRef = useRef<DOMRect | null>(null)
  const previousPlacementRef = useRef<Placement | null>(null)
  const positionAnimRef = useRef<Animation | null>(null)
  const widthAnimRef = useRef<Animation | null>(null)

  useLayoutEffect(() => {
    const motionElement = motionRef.current
    const frameElement = motionElement?.querySelector<HTMLElement>('[data-popup-frame]')
    if (!active || !motionElement || !frameElement) {
      previousRectRef.current = null
      previousPlacementRef.current = null
      positionAnimRef.current?.cancel()
      widthAnimRef.current?.cancel()
      positionAnimRef.current = null
      widthAnimRef.current = null
      return
    }

    // Measure the frame (the visible box) rather than the positioned wrapper so
    // the width tween matches what the user sees.
    const nextRect = frameElement.getBoundingClientRect()
    const previousRect = previousRectRef.current
    const previousPlacement = previousPlacementRef.current
    const shouldFlip =
      previousRect &&
      previousPlacement &&
      previousPlacement !== placement &&
      (previousPlacement === 'viewport-top' || placement === 'viewport-top')

    if (shouldFlip) {
      positionAnimRef.current?.cancel()
      widthAnimRef.current?.cancel()
      // Every placement is left-anchored (page-anchored popups emit a clamped
      // explicit `left` px; viewport-top/overlay always did), so the FLIP
      // deltas anchor on left edges. Re-centering on the frame's live width as
      // it tweens happens in popupStyle, which recomputes `left` from the
      // measured width each layout broadcast.
      const deltaX = previousRect.left - nextRect.left
      const deltaY = previousRect.top - nextRect.top
      const timing = {
        duration: DEFAULT_CAMERA_TRANSITION_DURATION_MS,
        easing: CAMERA_SPRING_CSS_EASING,
      }

      positionAnimRef.current = motionElement.animate(
        [{ transform: `translate(${deltaX}px, ${deltaY}px)` }, { transform: 'translate(0, 0)' }],
        timing,
      )
      // Width only needs an explicit tween when entering the viewport-top bar:
      // its resting width is the live container width, so we tween toward `100%`
      // (not a frozen px snapshot) — a chrome inset/recenter landing mid-flight
      // then can't snap at the end, mirroring how `left` stays live.
      //
      // Page-anchored placements get their width morph for free: popupStyle sets
      // `minWidth: rect.width`, and `rect.width` is the live page width shrinking
      // as the camera restores. A frozen px tween here fights that live width and
      // snaps to the real (smaller) size only once the camera settles, so we let
      // the live minWidth carry it instead.
      if (placement === 'viewport-top') {
        widthAnimRef.current = frameElement.animate(
          [{ width: `${previousRect.width}px` }, { width: '100%' }],
          timing,
        )
        widthAnimRef.current.onfinish = () => {
          widthAnimRef.current = null
        }
      }
      positionAnimRef.current.onfinish = () => {
        positionAnimRef.current = null
      }

      // The main-process camera move starts the instant focus toggles, but this
      // renderer animation only starts once the layout broadcast round-trips
      // back. Fast-forward by however long the camera has already been running
      // (Date.now is the same wall clock in both processes) so the popup morph
      // stays in phase with the page instead of finishing a few frames late.
      if (cameraTransitionStartedAt !== null) {
        const elapsed = Math.min(
          DEFAULT_CAMERA_TRANSITION_DURATION_MS,
          Math.max(0, Date.now() - cameraTransitionStartedAt),
        )
        positionAnimRef.current.currentTime = elapsed
        if (widthAnimRef.current) widthAnimRef.current.currentTime = elapsed
      }
    }

    previousRectRef.current = nextRect
    previousPlacementRef.current = placement
    // No deps: re-measure after every commit so `previousRectRef` tracks the
    // popup's *settled* resting rect. A placement change (open/close) can fire
    // while the camera is still mid-restore — the frame there sits clamped to a
    // transient corner. If we only captured on placement change, the next FLIP
    // would tween from that stale rect (~zero delta = a snap instead of a
    // slide). Refreshing each render lets the baseline converge to rest before
    // the next flip. `shouldFlip` still gates on the placement change, so this
    // only refreshes the baseline; it never re-fires the animation.
  })

  return { layoutRef, motionRef }
}

function popupStyle(
  rect: { x: number; y: number; width: number; height: number },
  placement: Placement,
  align: Align,
  offset: number,
  layout: LayoutUpdateData,
  frameSize: { width: number; height: number } | null,
): CSSProperties {
  if (placement === 'viewport-top') {
    const left = layout.leftChromeWidth
    const rightInset = layout.devtoolsOpen ? layout.devtoolsWidth : 0
    const rightEdge = Math.max(0, layout.windowWidth - rightInset)
    return {
      left,
      top: 0,
      width: Math.max(0, rightEdge - left),
      transform: 'none',
    }
  }
  if (placement === 'overlay') {
    return { left: rect.x, top: rect.y, width: rect.width, height: rect.height }
  }
  const isAbove = placement === 'above'
  const bounds = canvasHorizontalBounds(layout)
  const availableSpan = Math.max(0, bounds.right - bounds.left - POPUP_EDGE_MARGIN * 2)
  const minWidth =
    align === 'stretch' ? Math.min(rect.width, availableSpan) : undefined
  // First render only: frame size unknown, position via percent transforms.
  // The measure effect fires before paint, so the clamped explicit style
  // below is what actually paints.
  if (frameSize === null) {
    return {
      left: rect.x + rect.width / 2,
      top: isAbove ? rect.y - offset : rect.y + rect.height + offset,
      minWidth,
      transform: `translateX(-50%) ${isAbove ? 'translateY(-100%)' : ''}`.trim(),
    }
  }
  // Clamp inside the canvas span. When the span is narrower than the popup,
  // pin to the leading edge and let it overflow — never compress content.
  const centeredLeft = rect.x + rect.width / 2 - frameSize.width / 2
  const left = Math.max(
    bounds.left + POPUP_EDGE_MARGIN,
    Math.min(centeredLeft, bounds.right - POPUP_EDGE_MARGIN - frameSize.width),
  )
  const anchoredTop = isAbove
    ? rect.y - offset - frameSize.height
    : rect.y + rect.height + offset
  const top = Math.max(POPUP_EDGE_MARGIN, anchoredTop)
  // Position via transform, not left/top: layout offsets snap to the pixel
  // grid each frame, which reads as jitter against the subpixel-smooth page
  // views while panning.
  return {
    left: 0,
    top: 0,
    minWidth,
    transform: `translate(${left}px, ${top}px)`,
  }
}

// ADR 0008 §1 — viewport-anchored (tool mode) strip below toolbar.
// Uses `layout.toolbarCenterX` (window-coord px from main) verbatim; no
// devtools-panel compensation needed because toolbar layout drives placement.
function ViewportAnchor({
  layout,
  open,
  offset = 8,
  children,
}: {
  layout: LayoutUpdateData
  open: boolean
  offset?: number
  children: ReactNode
}) {
  if (!open) return null
  // A focus session pins the focus bar to the viewport top (placement
  // 'viewport-top', height TOOLBAR_HEIGHT). Drop the tool popup below it so the
  // two stack instead of colliding.
  const top = offset + (focusContext(layout).active ? TOOLBAR_HEIGHT : 0)
  return (
    <>
      {/* Bridge across the gap between the toolbar and the popup. Marked as
          overlay UI so the placement-preview ghost clears while the cursor
          is in this strip, instead of stamping through the gap. */}
      {/* zIndex matches Root: without it, a selected page's resize handles
          (positioned, zIndex 1) paint above the popup in the root stacking
          context and steal the pointer — `data-resize-handle` is routable, so
          clicks fall through to the canvas. */}
      <div
        data-overlay-ui
        data-viewport-passthrough
        aria-hidden
        className="pointer-events-auto absolute left-0 right-0"
        style={{ top: 0, height: top, zIndex: 20 }}
      />
      <div
        data-overlay-ui
        data-viewport-passthrough
        className="pointer-events-auto absolute"
        style={{
          top,
          left: layout.toolbarCenterX,
          transform: 'translateX(-50%)',
          zIndex: 20,
        }}
      >
        {children}
      </div>
    </>
  )
}

// Stops mousedown so clicks inside don't fall through to canvas gestures.
function Frame({
  isDark,
  flush = false,
  fullWidth = false,
  className = '',
  children,
}: {
  isDark: boolean
  flush?: boolean
  fullWidth?: boolean
  className?: string
  children: ReactNode
}) {
  const shapeClass = `${fullWidth ? 'w-full ' : ''}${
    flush ? 'rounded-none border-b flex items-center p-1' : POPUP_SURFACE_CLASS
  }`
  const frameProps = {
    'data-popup-frame': flush ? 'flush' : 'floating',
    className: `${shapeClass} text-[var(--surface-panel-foreground)] ${className}`.trim(),
    style: flush
      ? {
          // Flush focus bar matches the toolbar height so 'fill' page content,
          // which starts at the toolbar inset, butts directly against it.
          height: TOOLBAR_HEIGHT,
          background: 'var(--surface-popup)',
          borderColor: 'var(--surface-popup-border)',
          boxShadow: 'none',
        }
      : popupSurfaceStyle(isDark),
    // Primary clicks stay with popup controls. Middle-drag bubbles to the
    // viewport hook so a visible popup never creates a dead pan region.
    onPointerDown: (event: PointerEvent) => {
      if (event.button === 0) event.stopPropagation()
    },
  }
  const contentProps = {
    'data-popup-frame-content': true,
    className: 'flex min-w-0 w-full items-center gap-1',
  }
  return (
    <div {...frameProps}>
      <div {...contentProps}>{children}</div>
    </div>
  )
}

function Section({ children, grow = false }: { children: ReactNode; grow?: boolean }) {
  return (
    <div className={`flex items-center gap-1${grow ? ' min-w-0 flex-1' : ''}`}>
      {children}
    </div>
  )
}

function Divider({ isDark }: { isDark: boolean }) {
  return (
    <div
      aria-hidden
      className={`mx-1 h-4 w-px shrink-0 ${isDark ? 'bg-white/20' : 'bg-zinc-900/20'}`}
    />
  )
}

function popupIconButtonClass(isDark: boolean, active = false): string {
  const base =
    'flex h-6 w-6 items-center justify-center rounded-[6px] border-0 transition-colors'
  if (active) {
    return isDark
      ? `${base} bg-[rgba(253,248,245,0.1)] text-[var(--surface-panel-foreground)]`
      : `${base} bg-[var(--color-stone-200)] text-[var(--surface-panel-foreground)]`
  }
  return isDark
    ? `${base} text-[var(--surface-panel-foreground-muted)] hover:bg-[rgba(253,248,245,0.1)] hover:text-[var(--surface-panel-foreground)]`
    : `${base} text-[var(--surface-panel-foreground-muted)] hover:bg-[var(--color-stone-100)] hover:text-[var(--surface-panel-foreground)]`
}

function IconButton({
  isDark,
  active = false,
  disabled = false,
  title,
  ariaLabel,
  onClick,
  children,
}: {
  isDark: boolean
  active?: boolean
  disabled?: boolean
  title: string
  ariaLabel: string
  onClick: () => void
  children: ReactNode
}) {
  return (
    <Tooltip label={title}>
      <button
        type="button"
        className={`${popupIconButtonClass(isDark, active)} disabled:cursor-default disabled:opacity-30 disabled:hover:bg-transparent`}
        onClick={onClick}
        disabled={disabled}
        aria-label={ariaLabel}
        aria-pressed={active}
      >
        {children}
      </button>
    </Tooltip>
  )
}

function ColorSwatch({
  active,
  color,
  ariaLabel,
  isDark,
  onClick,
}: {
  active: boolean
  color: string
  ariaLabel: string
  isDark: boolean
  onClick: () => void
}) {
  return (
    <button
      type="button"
      aria-label={ariaLabel}
      className="flex h-5 w-5 items-center justify-center rounded-full transition-shadow bg-[var(--surface-popup)]"
      style={{ boxShadow: swatchRingShadow(color, active, isDark) }}
      onClick={onClick}
    >
      <span
        className="block h-3 w-3 rounded-full"
        style={{ background: color, boxShadow: swatchDotShadow(isDark) }}
      />
    </button>
  )
}

// The tool popups' color row: one swatch per palette hue, ring on the active
// one. Callers vary the palette, resolve role, and target scope; the layout is
// shared.
function PaletteRow({
  isDark,
  palette,
  role,
  activeSlot,
  ariaLabel,
  onPick,
}: {
  isDark: boolean
  palette: CanvasPalette
  role: CanvasColorRole
  activeSlot: CanvasColorSlot | null
  ariaLabel: (slotLabel: string) => string
  onPick: (storage: string) => void
}) {
  return (
    <Section>
      {paletteSlots(palette).map((slot) => {
        const swatch = slot.hex ?? resolveCanvasColor(slot.storage, { role, isDark })
        return (
          <ColorSwatch
            key={slot.id}
            isDark={isDark}
            active={activeSlot === slot.id}
            color={swatch}
            ariaLabel={ariaLabel(slot.label)}
            onClick={() => onPick(slot.storage)}
          />
        )
      })}
    </Section>
  )
}

function EntityActions({
  isDark,
  noun,
  count,
  api,
  layout,
  entityIds,
  onAnnotate,
}: {
  isDark: boolean
  noun: string
  count: number
  api?: Pick<CanvasBgElectronAPI, 'focusSelection'> &
    Partial<Pick<CanvasBgElectronAPI, 'arrangeSelection'>>
  /** Layout + the ids this popup's actions apply to — both needed to derive
   *  the Annotate button's union bounds. Omit to suppress the button (e.g.
   *  a popup that has no natural id set to hand it). */
  layout?: LayoutUpdateData
  entityIds?: readonly string[]
  /** Opens the region composer pre-anchored to the selection's union bounds
   *  (renderer-local handoff — see useAnnotationDraftState.beginSelectionAnnotation).
   *  Omitted → no Annotate button, regardless of `layout`/`entityIds`. */
  onAnnotate?: AnnotateHandler
}) {
  const arrange = api?.arrangeSelection
  // Null only when none of the ids resolve against the current layout.
  const annotateRect =
    onAnnotate && layout && entityIds
      ? selectionAnnotationBounds(layout.entities, entityIds)
      : null
  // Focus stays pinned to the right; arrange (row/column/grid) and annotate
  // sit before it.
  return (
    <Section>
      <ArrangeButtons isDark={isDark} count={count} arrange={arrange} />
      {annotateRect && onAnnotate && entityIds && (
        <IconButton
          isDark={isDark}
          title={`Annotate ${noun}`}
          ariaLabel={`Annotate ${noun}`}
          onClick={() => onAnnotate([...entityIds], annotateRect)}
        >
          <MessageCircle size={14} />
        </IconButton>
      )}
      {api && (
        <IconButton
          isDark={isDark}
          title={`Focus ${noun}`}
          ariaLabel={`Focus ${noun}`}
          onClick={() => api.focusSelection()}
        >
          <Maximize2 size={14} />
        </IconButton>
      )}
    </Section>
  )
}

// Icon visuals match the arrangement, not the lucide name: a row of items reads
// as side-by-side vertical bars (Columns3); a column reads as stacked bars
// (Rows3).
const ARRANGE_MODES: {
  mode: BatchLayoutMode
  Icon: typeof Grid2x2
  label: string
}[] = [
  { mode: 'row', Icon: Columns2, label: 'Arrange in a row' },
  { mode: 'column', Icon: Rows2, label: 'Arrange in a column' },
  { mode: 'grid', Icon: Grid2x2, label: 'Arrange in a grid' },
]

// Row/column/grid pack buttons. Shown for a real multi-selection (count >= 2);
// nothing renders when the host lacks the arrange API. Standalone so PagePopup,
// which builds a bespoke action row, can drop it in alongside EntityActions.
function ArrangeButtons({
  isDark,
  count,
  arrange,
}: {
  isDark: boolean
  count: number
  arrange?: (mode: BatchLayoutMode) => void
}) {
  if (!arrange || count < 2) return null
  return (
    <>
      {ARRANGE_MODES.map(({ mode, Icon, label }) => (
        <IconButton
          key={mode}
          isDark={isDark}
          title={label}
          ariaLabel={label}
          onClick={() => arrange(mode)}
        >
          <Icon size={14} />
        </IconButton>
      ))}
    </>
  )
}

export const CanvasItemPopup = {
  Root,
  ViewportAnchor,
  Frame,
  Section,
  Divider,
  IconButton,
  ColorSwatch,
  PaletteRow,
  EntityActions,
  ArrangeButtons,
}
