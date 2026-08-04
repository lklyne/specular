/**
 * FocusedNoteLayer — the focused note as a fullscreen, screen-fixed card.
 *
 * A file-target focus session (ADR 0021) blows one markdown note up to the
 * viewport. The card renders OUTSIDE the camera transform (not in a
 * `CanvasViewportLayer`) — its geometry is screen-fixed — and is tagged
 * `data-overlay-ui`: the pointer router
 * hit-tests canvas coordinates, so a fixed card has to be overlay UI for real
 * DOM events to reach the editor. The flip side is that overlay UI is invisible
 * to the router — including its focus-exit branch — so the backdrop carries an
 * explicit exit handler (see `exitFocus` below).
 *
 * The backdrop also swallows viewport wheel/middle-drag for free:
 * `blocksViewportGesture` treats `data-overlay-ui` (without
 * `data-viewport-passthrough`) as a dead zone, so a wheel over the card scrolls
 * the editor natively and a wheel over the gutters does nothing — neither pans
 * the camera, which would end the session.
 */

import { useLayoutEffect, useRef } from 'react'
import { TOOLBAR_HEIGHT } from '../../shared/constants'
import {
  CAMERA_SPRING_CSS_EASING,
  DEFAULT_CAMERA_TRANSITION_DURATION_MS,
} from '../../shared/camera-transition'
import {
  canvasToScreenX,
  canvasToScreenY,
  toOverlayY,
  type ScreenRect,
} from '../../shared/coords'
import { DRAG_THRESHOLD } from '../../shared/gesture-utils'
import { focusContext } from '../../shared/focus-context'
import type { CanvasSceneFileEntity, LayoutUpdateData } from '../../shared/types'
import { RendererSwitch } from '../canvas-bg/entity-renderers/RendererSwitch'
import { fileCardSurface } from './FileBodyLayer'

/** Reading column width; the note never stretches past this. */
const FOCUSED_NOTE_MAX_WIDTH = 720
/** Minimum breathing room between the column and the canvas span edges. */
const FOCUSED_NOTE_MIN_GUTTER = 32
/** Reading padding for the fullscreen card. Wider than a canvas note card's,
 *  and applied inside the editor's scroller (see `MarkdownEditor.contentPadding`)
 *  so the scrollbar stays on the card edge and the top gutter scrolls away with
 *  the text instead of clipping it. */
const FOCUSED_NOTE_PADDING = '36px'
/** Vertical gap between the card and the focus bar above / window edge below. */
const FOCUSED_NOTE_Y_MARGIN = 16

/**
 * The fixed card geometry, in overlay-local coords (y=0 is the toolbar's bottom
 * edge — the aboveView WCV starts at `canvasOrigin.y`).
 *
 * The layer spans from y=0 so it also covers the strip the flush focus bar
 * occupies (0..TOOLBAR_HEIGHT); the bar paints over it at a higher z-index, and
 * covering the strip means no wheel there reaches the viewport pan/zoom path.
 * The card itself floats FOCUSED_NOTE_Y_MARGIN below the bar and above the
 * window's bottom edge.
 */
function focusedNoteGeometry(layout: LayoutUpdateData): { span: ScreenRect; card: ScreenRect } {
  const left = layout.leftChromeWidth
  const rightInset = layout.devtoolsOpen ? layout.devtoolsWidth : 0
  const right = Math.max(left, layout.windowWidth - rightInset)
  const spanWidth = right - left
  // The aboveView WCV is already inset by the toolbar, so its own innerHeight is
  // the overlay height (no second `canvasOrigin.y` subtraction).
  const spanHeight = window.innerHeight
  const cardWidth = Math.max(
    0,
    Math.min(FOCUSED_NOTE_MAX_WIDTH, spanWidth - FOCUSED_NOTE_MIN_GUTTER * 2),
  )
  return {
    span: { left, top: 0, width: spanWidth, height: spanHeight },
    card: {
      // Card coords are relative to the span container.
      left: Math.round((spanWidth - cardWidth) / 2),
      top: TOOLBAR_HEIGHT + FOCUSED_NOTE_Y_MARGIN,
      width: cardWidth,
      height: Math.max(0, spanHeight - TOOLBAR_HEIGHT - FOCUSED_NOTE_Y_MARGIN * 2),
    },
  }
}

/** Where the note's ordinary canvas card sits right now, in overlay coords. */
function liveEntityRect(layout: LayoutUpdateData, entity: CanvasSceneFileEntity): ScreenRect {
  return {
    left: canvasToScreenX(layout, entity.canvasX),
    top: toOverlayY(layout, canvasToScreenY(layout, entity.canvasY)),
    width: entity.width * layout.zoom,
    height: entity.height * layout.zoom,
  }
}

/**
 * Enter morph: the card starts at the note's camera-derived rect and tweens to
 * the fixed rect, in phase with the main-process camera fit.
 *
 * Translate + width/height rather than a transform scale: scale distorts text
 * (the same reason `usePopupFlipAnimation` tweens width directly). The width
 * tween reflows the editor naturally, so glyph metrics stay right throughout.
 *
 * Fast-forward: the camera move starts the instant focus toggles, but this
 * layer only mounts once the layout broadcast round-trips back, so the
 * animation is seeked to however long the camera has already been running
 * (`Date.now` is the same wall clock in both processes). The `from` rect is
 * therefore already a frame or two into the camera tween — at ~16ms of 320 the
 * error is invisible, and skipping the seek would instead land the card
 * visibly after the camera settles.
 *
 * There is no exit morph: the session end unmounts the layer and the ordinary
 * canvas card reappears at its (still zoomed-in) live rect, which the camera
 * restore then tweens back out. Animating the exit would mean fighting that
 * live rect — the trap `usePopupFlipAnimation` documents for its width tween.
 */
function useFocusedNoteEnterMorph({
  cardRef,
  backdropRef,
  from,
  to,
  cameraTransitionStartedAt,
}: {
  cardRef: React.RefObject<HTMLDivElement | null>
  backdropRef: React.RefObject<HTMLDivElement | null>
  from: ScreenRect
  to: ScreenRect
  cameraTransitionStartedAt: number | null
}) {
  // Read at mount only — later renders must not restart the morph.
  const mountRef = useRef({ from, to, startedAt: cameraTransitionStartedAt })
  useLayoutEffect(() => {
    const card = cardRef.current
    if (!card) return
    const { from: start, to: end, startedAt } = mountRef.current
    const timing = {
      duration: DEFAULT_CAMERA_TRANSITION_DURATION_MS,
      easing: CAMERA_SPRING_CSS_EASING,
    }
    const animations: Animation[] = []
    animations.push(
      card.animate(
        [
          {
            transform: `translate(${start.left - end.left}px, ${start.top - end.top}px)`,
            width: `${start.width}px`,
            height: `${start.height}px`,
          },
          { transform: 'translate(0px, 0px)', width: `${end.width}px`, height: `${end.height}px` },
        ],
        timing,
      ),
    )
    const backdrop = backdropRef.current
    if (backdrop) {
      animations.push(backdrop.animate([{ opacity: 0 }, { opacity: 1 }], timing))
    }
    if (startedAt !== null) {
      const elapsed = Math.min(
        DEFAULT_CAMERA_TRANSITION_DURATION_MS,
        Math.max(0, Date.now() - startedAt),
      )
      for (const animation of animations) animation.currentTime = elapsed
    }
    return () => {
      for (const animation of animations) animation.cancel()
    }
  }, [backdropRef, cardRef])
}

export function FocusedNoteLayer({
  layout,
  entity,
  isDark,
  editingEntityId,
  onExitFocus,
  onTextEditingChange,
  onOpenLink,
}: {
  layout: LayoutUpdateData
  /** The focused note, resolved by the mount site — which also mounts this layer
   *  only while a file-target session frames a live entity. */
  entity: CanvasSceneFileEntity
  isDark: boolean
  /** id of the entity in inline-edit mode; drives the editor's `canEdit`. */
  editingEntityId: string | null
  /** Backdrop/gutter click — the router can't see overlay UI, so we call the
   *  same graceful exit its dimmed-canvas branch does. */
  onExitFocus: () => void
  onTextEditingChange: (active: boolean) => void
  onOpenLink: (id: string, url: string) => void
}) {
  const cardRef = useRef<HTMLDivElement | null>(null)
  const backdropRef = useRef<HTMLDivElement | null>(null)
  // Exit on a click, not on any pointerup: a text selection dragged out of the
  // card and released over a gutter is not a request to leave focus. Mirrors
  // the router's `!dragged` guard on its own focus-exit branch.
  const pressRef = useRef<{ x: number; y: number } | null>(null)
  const focus = focusContext(layout)
  const { span, card } = focusedNoteGeometry(layout)
  // The live rect is overlay-absolute; card coords are span-relative.
  const live = liveEntityRect(layout, entity)
  useFocusedNoteEnterMorph({
    cardRef,
    backdropRef,
    from: { ...live, left: live.left - span.left },
    to: card,
    cameraTransitionStartedAt: layout.cameraTransitionStartedAt,
  })

  const surface = fileCardSurface(isDark)
  return (
    <div
      data-overlay-ui
      // `pointer-events-auto` because the pan wrapper this sits in is
      // `pointer-events-none`; without it the editor never sees a DOM event.
      className="pointer-events-auto absolute"
      // Above every canvas layer — EdgeLayer (5), comment badges (15) — and
      // below the canvas-item popups (20) so the focus bar still paints over
      // the top strip. Comparable only because this shares the pan wrapper's
      // stacking context with them (see the mount site in above-view/App.tsx).
      style={{ left: span.left, top: span.top, width: span.width, height: span.height, zIndex: 19 }}
    >
      {/* Opaque, never a dim: with the eye off the surrounding context is gone,
          not faded (ADR 0021 Amendment 2). The canvas's own `--surface-canvas`
          is translucent over the window vibrancy, so the backdrop uses the same
          stone tones at full opacity. With the eye on, context comes back at
          full opacity — the backdrop drops to transparent and the canvas shows
          through around the card, while still owning the gutter click-to-exit
          and the wheel dead zone. */}
      <div
        ref={backdropRef}
        className="absolute inset-0"
        style={{ background: focus.showsContext ? 'transparent' : isDark ? '#292524' : '#e7e5e4' }}
        onPointerDown={(event) => {
          pressRef.current = { x: event.clientX, y: event.clientY }
        }}
        onPointerUp={(event) => {
          const press = pressRef.current
          pressRef.current = null
          if (!press) return
          if (Math.hypot(event.clientX - press.x, event.clientY - press.y) > DRAG_THRESHOLD) return
          onExitFocus()
        }}
      />
      <div
        ref={cardRef}
        className="absolute overflow-hidden"
        style={{
          left: card.left,
          top: card.top,
          width: card.width,
          height: card.height,
          background: surface.background,
          boxShadow: surface.boxShadow,
        }}
      >
        <RendererSwitch
          entity={entity}
          canEdit={editingEntityId === entity.id}
          isDark={isDark}
          isInteractive={false}
          contentPadding={FOCUSED_NOTE_PADDING}
          onTextEditingChange={onTextEditingChange}
          onOpenLink={onOpenLink}
        />
      </div>
    </div>
  )
}
