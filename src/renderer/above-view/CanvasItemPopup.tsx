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
import {
  FOCUS_PRESENTATION_MENU_EDGE_INSET_PX,
  FOCUS_PRESENTATION_MENU_INSET,
} from '../../shared/featureFlags'
import {
  CAMERA_SPRING_CSS_EASING,
  DEFAULT_CAMERA_TRANSITION_DURATION_MS,
} from '../../shared/camera-transition'
import type { CanvasBgElectronAPI, LayoutUpdateData } from '../../shared/types'
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
      // The viewport-top frame is `w-full`, so its resting width is the live
      // container width. Tween toward `100%` (not a frozen px snapshot) so a
      // chrome inset/recenter that lands mid-flight doesn't snap at the end —
      // mirrors how `left` stays live and only the translate delta decays.
      const targetWidth = placement === 'viewport-top' ? '100%' : `${nextRect.width}px`
      widthAnimRef.current = frameElement.animate(
        [{ width: `${previousRect.width}px` }, { width: targetWidth }],
        timing,
      )
      positionAnimRef.current.onfinish = () => {
        positionAnimRef.current = null
      }
      widthAnimRef.current.onfinish = () => {
        widthAnimRef.current = null
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
        widthAnimRef.current.currentTime = elapsed
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
    const inset = FOCUS_PRESENTATION_MENU_INSET
      ? FOCUS_PRESENTATION_MENU_EDGE_INSET_PX
      : 0
    const left = layout.leftChromeWidth + inset
    const rightInset = layout.devtoolsOpen ? layout.devtoolsWidth : 0
    const rightEdge = Math.max(0, layout.windowWidth - rightInset - inset)
    return {
      left,
      top: inset,
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
  return (
    <>
      {/* Bridge across the gap between the toolbar and the popup. Marked as
          overlay UI so the placement-preview ghost clears while the cursor
          is in this strip, instead of stamping through the gap. */}
      <div
        data-overlay-ui
        aria-hidden
        className="pointer-events-auto absolute left-0 right-0"
        style={{ top: 0, height: offset }}
      />
      <div
        data-overlay-ui
        className="pointer-events-auto absolute"
        style={{
          top: offset,
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
    flush ? 'rounded-none' : 'rounded-[10px]'
  } border p-1`
  const frameProps = {
    'data-popup-frame': flush ? 'flush' : 'floating',
    className: `${shapeClass} ${
      isDark ? 'text-zinc-100' : 'text-zinc-900'
    } ${className}`.trim(),
    style: {
      background: isDark ? '#3a3836' : '#ece9e7',
      borderColor: isDark ? '#414141' : '#dcdcda',
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
      : `${base} bg-[#fdf8f5] text-zinc-900`
  }
  return isDark
    ? `${base} text-zinc-300 hover:bg-[rgba(253,248,245,0.1)] hover:text-zinc-100`
    : `${base} text-zinc-600 hover:bg-[#fdf8f5] hover:text-zinc-900`
}

function IconButton({
  isDark,
  active = false,
  title,
  ariaLabel,
  onClick,
  children,
}: {
  isDark: boolean
  active?: boolean
  title: string
  ariaLabel: string
  onClick: () => void
  children: ReactNode
}) {
  return (
    <button
      type="button"
      className={popupIconButtonClass(isDark, active)}
      onClick={onClick}
      title={title}
      aria-label={ariaLabel}
      aria-pressed={active}
    >
      {children}
    </button>
  )
}

function ColorSwatch({
  isDark,
  active,
  color,
  ariaLabel,
  onClick,
}: {
  isDark: boolean
  active: boolean
  color: string
  ariaLabel: string
  onClick: () => void
}) {
  return (
    <button
      type="button"
      aria-label={ariaLabel}
      className={`flex h-5 w-5 items-center justify-center rounded-full border transition-colors ${
        isDark ? 'bg-[#3a3836]' : 'bg-[#ece9e7]'
      }`}
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
            isDark={isDark}
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
