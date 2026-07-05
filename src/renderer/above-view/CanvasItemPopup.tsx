// ADR 0008 — unified canvas-item popup compound component.

import { useLayoutEffect, useRef, type CSSProperties, type PointerEvent, type ReactNode } from 'react'
import { AlignHorizontalDistributeCenter, Maximize2 } from 'lucide-react'
import {
  paletteSlots,
  resolveCanvasColor,
  type CanvasColorRole,
  type CanvasColorSlot,
  type CanvasPalette,
} from '../../shared/canvas-colors'
import { TOOLBAR_HEIGHT } from '../../shared/constants'
import { focusContext } from '../../shared/focus-context'
import {
  CAMERA_SPRING_CSS_EASING,
  DEFAULT_CAMERA_TRANSITION_DURATION_MS,
} from '../../shared/camera-transition'
import type { LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import {
  useAnchoredPosition,
  useMultiAnchoredPosition,
  type AnchorSlot,
} from './useAnchoredPosition'

type Placement = 'above' | 'below' | 'overlay' | 'viewport-top'
type Align = 'stretch' | 'center'

type RootProps = {
  layout: LayoutUpdateData
  open: boolean
  slot?: AnchorSlot
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
    slot = 'body',
    placement = 'below',
    align = 'center',
    offset = 8,
    children,
  } = props
  const singleRect = useAnchoredPosition(
    layout,
    props.entityId ?? '',
    slot,
  )
  const multiRect = useMultiAnchoredPosition(layout, props.entityIds ?? [], slot)
  const rect = props.entityIds !== undefined ? multiRect : singleRect
  const popupMotion = usePopupFlipAnimation({
    active: open && rect !== null,
    placement,
    cameraTransitionStartedAt: layout.cameraTransitionStartedAt,
  })
  if (!open || !rect) return null
  const style = popupStyle(rect, placement, align, offset, layout)
  return (
    <div
      ref={popupMotion.layoutRef}
      data-overlay-ui
      data-popup-placement={placement}
      className="pointer-events-auto absolute"
      // z-index above EdgeLayer (5) so edges don't paint over canvas-item toolbars.
      style={{ zIndex: 20, ...style }}
    >
      <div ref={popupMotion.motionRef}>{children}</div>
    </div>
  )
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
      // Page-anchored placements (above/below) center via translateX(-50%), so
      // the wrapper recenters on the frame's *live* width as it tweens. Anchor
      // the FLIP on the box center there — anchoring on the left edge makes the
      // centering double-count the width delta and the frame jumps half the
      // width difference on the first frame. Viewport-top/overlay are
      // left-anchored, so anchor on the left edge.
      const destCenters = placement === 'above' || placement === 'below'
      const previousAnchorX = destCenters
        ? previousRect.left + previousRect.width / 2
        : previousRect.left
      const nextAnchorX = destCenters
        ? nextRect.left + nextRect.width / 2
        : nextRect.left
      const deltaX = previousAnchorX - nextAnchorX
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
  }, [active, placement])

  return { layoutRef, motionRef }
}

function popupStyle(
  rect: { x: number; y: number; width: number; height: number },
  placement: Placement,
  align: Align,
  offset: number,
  layout: LayoutUpdateData,
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
  const top = isAbove ? rect.y - offset : rect.y + rect.height + offset
  const verticalTransform = isAbove ? 'translateY(-100%)' : ''
  if (align === 'stretch') {
    return {
      left: rect.x + rect.width / 2,
      top,
      minWidth: rect.width,
      transform: `translateX(-50%) ${verticalTransform}`.trim(),
    }
  }
  return {
    left: rect.x + rect.width / 2,
    top,
    transform: `translateX(-50%) ${verticalTransform}`.trim(),
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
      <div
        data-overlay-ui
        aria-hidden
        className="pointer-events-auto absolute left-0 right-0"
        style={{ top: 0, height: top }}
      />
      <div
        data-overlay-ui
        className="pointer-events-auto absolute"
        style={{
          top,
          left: layout.toolbarCenterX,
          transform: 'translateX(-50%)',
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
    flush ? 'rounded-none border-b flex items-center' : 'rounded-[10px] border'
  } p-1`
  const frameProps = {
    'data-popup-frame': flush ? 'flush' : 'floating',
    className: `${shapeClass} ${
      isDark ? 'text-zinc-100' : 'text-zinc-900'
    } ${className}`.trim(),
    style: {
      // Flush focus bar matches the toolbar height so 'fill' page content,
      // which starts at the toolbar inset, butts directly against it.
      height: flush ? TOOLBAR_HEIGHT : undefined,
      background: 'var(--surface-popup)',
      borderColor: 'var(--surface-popup-border)',
      boxShadow: flush
        ? 'none'
        : isDark
        ? '0 10px 8px -6px rgba(0,0,0,.58), 0 4px 16px 0 rgba(0,0,0,.5)'
        : '0 10px 8px -6px rgba(0,0,0,.18), 0 4px 16px 0 rgba(199,193,188,.5)',
    },
    onPointerDown: (event: PointerEvent) => event.stopPropagation(),
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
      ? `${base} bg-[rgba(253,248,245,0.1)] text-zinc-100`
      : `${base} bg-[var(--color-stone-200)] text-zinc-900`
  }
  return isDark
    ? `${base} text-zinc-300 hover:bg-[rgba(253,248,245,0.1)] hover:text-zinc-100`
    : `${base} text-zinc-600 hover:bg-[var(--color-stone-100)] hover:text-zinc-900`
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
    <button
      type="button"
      className={`${popupIconButtonClass(isDark, active)} disabled:cursor-default disabled:opacity-30 disabled:hover:bg-transparent`}
      onClick={onClick}
      disabled={disabled}
      title={title}
      aria-label={ariaLabel}
      aria-pressed={active}
    >
      {children}
    </button>
  )
}

function ColorSwatch({
  active,
  color,
  ariaLabel,
  onClick,
}: {
  active: boolean
  color: string
  ariaLabel: string
  onClick: () => void
}) {
  return (
    <button
      type="button"
      aria-label={ariaLabel}
      className="flex h-5 w-5 items-center justify-center rounded-full border transition-colors bg-[var(--surface-popup)]"
      style={{ borderColor: active ? color : 'transparent' }}
      onClick={onClick}
    >
      <span
        className="block h-3 w-3 rounded-full"
        style={{ background: color }}
      />
    </button>
  )
}

function EntityActions({
  isDark,
  noun,
  count,
  api,
}: {
  isDark: boolean
  noun: string
  count: number
  api?: Pick<CanvasBgElectronAPI, 'focusSelection'> &
    Partial<Pick<CanvasBgElectronAPI, 'distributeSelection'>>
}) {
  return (
    <Section>
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
      {api?.distributeSelection && count >= 3 && (
        <IconButton
          isDark={isDark}
          title="Distribute spacing"
          ariaLabel="Distribute spacing"
          onClick={() => api.distributeSelection?.()}
        >
          <AlignHorizontalDistributeCenter size={14} />
        </IconButton>
      )}
    </Section>
  )
}

function PaletteSection({
  isDark,
  palette,
  activeSlot,
  role,
  noun,
  onPick,
}: {
  isDark: boolean
  palette: CanvasPalette
  activeSlot: CanvasColorSlot | null
  role: CanvasColorRole
  noun?: string
  onPick: (storage: string) => void
}) {
  return (
    <Section>
      {paletteSlots(palette).map((slot) => {
        const swatch = slot.hex ?? resolveCanvasColor(slot.storage, { role, isDark })
        const ariaLabel = noun
          ? `Set ${noun} color to ${slot.label}`
          : `Set color to ${slot.label}`
        return (
          <ColorSwatch
            key={slot.id}
            active={activeSlot === slot.id}
            color={swatch}
            ariaLabel={ariaLabel}
            onClick={() => onPick(slot.storage)}
          />
        )
      })}
    </Section>
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
  EntityActions,
  PaletteSection,
}
