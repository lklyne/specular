import { type CSSProperties, useMemo, useEffect, useRef, useState } from 'react'
import type {
  AgentPresenceCursor,
  CanvasScenePageEntity,
  PresenceActivity,
} from '../../shared/types'
import { labelForPresenceCursor } from '../../shared/agent-presence'
import {
  DEFAULT_CURSOR_MOTION,
  easeAt,
  type Vec2,
} from '../../shared/cursor-motion'
import {
  DEFAULT_CURSOR_TUNING,
  distanceSpeedScale,
} from '../../shared/cursor-tuning'
import { foldSpline } from '../../shared/cursor-spline'
import { pagePointMatchesTargetRect } from '../../shared/presence-targeting'
import { PRESENCE_STEP_DELAY_MS } from '../../shared/presence-timing'
import { FilledCursorIcon } from '../shared/FilledCursorIcon'
import {
  CURSOR_TRAIL_OFFSET,
  PresenceParticleTrail,
  type PresenceParticleCursor,
} from '../shared/PresenceParticleTrail'

// Floor so a sub-pixel hop still reads as motion instead of a snap.
const MIN_ANIMATE_DURATION_MS = 60
const POSITION_EPSILON = 0.5

/**
 * Travel duration for a hop of `lengthPx` along the folded spline, using the
 * same speed model as the debug playground (speed = baseSpeedPxS *
 * distanceSpeedScale(length)) so long hops read at a plausible walking pace
 * instead of stretching or compressing to a fixed duration.
 *
 * Capped at `dwellBudgetMs` — the server's pre-act dwell budget for the act
 * this hop is traveling toward (ADR 0029 adaptive dwell). Without the cap, a
 * long hop under the short burst-regime budget would let travel outlive the
 * dwell and the act would fire mid-flight, breaking visible causality.
 * `dwellBudgetMs` is absent before a session's first act; PRESENCE_STEP_DELAY_MS
 * is the pre-adaptive-dwell fallback.
 */
function travelDurationMs(
  lengthPx: number,
  dwellBudgetMs: number | null | undefined,
): number {
  if (lengthPx <= 0) return 0
  const speedScale = distanceSpeedScale(DEFAULT_CURSOR_TUNING, lengthPx)
  const effectiveSpeedPxS = DEFAULT_CURSOR_TUNING.baseSpeedPxS * speedScale
  const speedBasedMs =
    effectiveSpeedPxS > 0 ? (lengthPx / effectiveSpeedPxS) * 1000 : Infinity
  const capMs = dwellBudgetMs ?? PRESENCE_STEP_DELAY_MS
  return Math.max(MIN_ANIMATE_DURATION_MS, Math.min(speedBasedMs, capMs))
}

function activityStyle(activity: PresenceActivity): CSSProperties {
  switch (activity) {
    case 'traveling':
      return { opacity: 1, transform: 'scale(1)', filter: 'saturate(1.1)' }
    case 'acting':
      return { opacity: 1, transform: 'scale(1.02)', filter: 'saturate(1.15)' }
    case 'waiting':
      return {
        opacity: 0.95,
        transform: 'scale(1)',
        animation: 'agent-presence-pulse 1.3s ease-in-out infinite',
      }
    case 'thinking':
      return { opacity: 1, transform: 'scale(0.98)' }
    case 'idle':
      return { opacity: 0.38, transform: 'scale(0.96)' }
    case 'departing':
      return { opacity: 0.7, transform: 'scale(0.96)' }
  }
}

function TargetHalo({
  cursor,
  page,
  overlayOffsetY,
}: {
  cursor: AgentPresenceCursor
  page: CanvasScenePageEntity | null
  overlayOffsetY: number
}) {
  if (
    !page ||
    !cursor.targetRect ||
    !pagePointMatchesTargetRect(
      cursor.pageX,
      cursor.pageY,
      cursor.targetRect,
    )
  ) {
    return null
  }
  const scaleX = page.screenWidth / Math.max(page.width, 1)
  const scaleY = page.screenHeight / Math.max(page.height, 1)
  return (
    <div
      className="absolute rounded-xl border"
      style={{
        left: page.screenX + cursor.targetRect.x * scaleX - 6,
        top: page.screenY + cursor.targetRect.y * scaleY - overlayOffsetY - 6,
        width: cursor.targetRect.width * scaleX + 12,
        height: cursor.targetRect.height * scaleY + 12,
        borderColor: cursor.color,
        boxShadow: `0 0 0 2px color-mix(in srgb, ${cursor.color} 32%, transparent)`,
        background: `color-mix(in srgb, ${cursor.color} 14%, transparent)`,
      }}
    />
  )
}

const RIPPLE_SIZE = 96
const RIPPLE_DURATION_MS = 100

const RIPPLE_DELAY_MS = PRESENCE_STEP_DELAY_MS - RIPPLE_DURATION_MS

function ClickRipple({ color }: { color: string }) {
  return (
    <div
      className="absolute rounded-full"
      style={{
        width: RIPPLE_SIZE,
        height: RIPPLE_SIZE,
        left: -(RIPPLE_SIZE / 2),
        top: -(RIPPLE_SIZE / 2),
        background: `color-mix(in srgb, ${color} 40%, transparent)`,
        animation: `agent-click-ripple ${RIPPLE_DURATION_MS}ms ease-out ${RIPPLE_DELAY_MS}ms forwards`,
        opacity: 0,
        pointerEvents: 'none',
      }}
    />
  )
}

function AgentCursor({
  cursor,
  point,
  zoom,
}: {
  cursor: AgentPresenceCursor
  point: Vec2
  zoom: number
}) {
  const label = labelForPresenceCursor(cursor)
  const [rippleKey, setRippleKey] = useState<number | null>(null)
  const rippleCounterRef = useRef(0)
  const prevActivity = useRef(cursor.activity)

  useEffect(() => {
    const wasClick =
      cursor.activity === 'acting' &&
      cursor.labelKey === 'click_target' &&
      prevActivity.current !== 'acting'
    prevActivity.current = cursor.activity
    if (wasClick) {
      setRippleKey(++rippleCounterRef.current)
    }
  }, [cursor.activity, cursor.labelKey])

  const positionStyle: CSSProperties = useMemo(
    () => ({
      left: 0,
      top: 0,
      transform: `translate3d(${point.x}px, ${point.y}px, 0)`,
      willChange: 'transform',
    }),
    [point.x, point.y],
  )

  // Counter-scale keeps icon, label, and ripple at constant screen size
  // regardless of canvas zoom.
  const counterScaleStyle: CSSProperties = {
    transform: `scale(${1 / zoom})`,
    transformOrigin: 'top left',
  }

  // Transition transform/opacity/filter so activity changes (acting ↔ idle,
  // fade on departing) ease instead of snapping.
  const activityTransformStyle: CSSProperties = {
    ...activityStyle(cursor.activity),
    transition: 'transform 800ms ease-out, opacity 800ms ease-out, filter 800ms ease-out',
  }

  return (
    <div className="absolute" style={positionStyle}>
      <div style={counterScaleStyle}>
        <div style={activityTransformStyle}>
          {rippleKey !== null && (
            <ClickRipple key={rippleKey} color={cursor.color} />
          )}
          <FilledCursorIcon color={cursor.color} size={24} />
          {label ? (
            <div
              className="ml-4 -mt-1.5 whitespace-nowrap rounded px-2 py-0.5"
              style={{
                backgroundColor: cursor.color,
                fontSize: 10,
                lineHeight: '14px',
                color: 'white',
                boxShadow:
                  cursor.activity === 'acting'
                    ? '0 2px 8px rgba(0,0,0,0.28)'
                    : '0 1px 3px rgba(0,0,0,0.2)',
              }}
            >
              {label}
            </div>
          ) : null}
        </div>
      </div>
    </div>
  )
}

/** Halo around an agent-active page so it's discoverable while CDP runs.
 *  Pass `originY={canvasOrigin.y}` when mounted in aboveView so page coords
 *  align against the WCV whose origin sits at the toolbar inset. */
export function ActivePageHighlightLayer({
  cursors,
  pages,
  originY = 0,
}: {
  cursors: AgentPresenceCursor[]
  pages: CanvasScenePageEntity[]
  originY?: number
}) {
  const activePages = useMemo(() => {
    const map = new Map<string, string>()
    for (const cursor of cursors) {
      if (cursor.pageId && !map.has(cursor.pageId)) {
        map.set(cursor.pageId, cursor.color)
      }
    }
    return map
  }, [cursors])

  if (activePages.size === 0) return null
  const inset = -4
  return (
    <>
      {pages
        .filter((page) => activePages.has(page.id))
        .map((page) => (
          <div
            key={`page-highlight-${page.id}`}
            className="absolute rounded-sm pointer-events-none"
            style={{
              left: page.screenX + inset,
              top: page.screenY + inset - originY,
              width: page.screenWidth - inset * 2,
              height: page.screenHeight - inset * 2,
              boxShadow: `0 0 0 2px ${activePages.get(page.id)!}, 0 0 24px 4px color-mix(in srgb, ${activePages.get(page.id)!} 25%, transparent)`,
              transition: 'box-shadow 300ms ease-out',
            }}
          />
        ))}
    </>
  )
}

interface CursorAnim {
  point: Vec2
  tangent: Vec2
  spline: ReturnType<typeof foldSpline> | null
  startedAt: number
  duration: number
  target: Vec2
}

interface AnimatedCursor {
  cursor: AgentPresenceCursor
  point: Vec2
  isAnimating: boolean
}

// Drives one RAF for all presence cursors so the DOM icon and particle trail
// read from the same interpolated positions. Target changes from the server
// start a new spline from the current (position, tangent). The RAF only runs
// while at least one spline is active, so a steady-state canvas idles.
function useAnimatedCursors(cursors: AgentPresenceCursor[]): AnimatedCursor[] {
  const animsRef = useRef<Map<string, CursorAnim>>(new Map())
  const rafIdRef = useRef(0)
  const [, setTick] = useState(0)

  useEffect(() => {
    const anims = animsRef.current
    let installedSpline = false
    for (const c of cursors) {
      const target: Vec2 = { x: c.canvasX, y: c.canvasY }
      const existing = anims.get(c.sessionId)
      if (!existing) {
        anims.set(c.sessionId, {
          point: target,
          tangent: { x: 1, y: 0 },
          spline: null,
          startedAt: 0,
          duration: 0,
          target,
        })
        continue
      }
      const dx = target.x - existing.point.x
      const dy = target.y - existing.point.y
      if (Math.abs(dx) < POSITION_EPSILON && Math.abs(dy) < POSITION_EPSILON) {
        existing.target = target
        existing.spline = null
        continue
      }
      const spline = foldSpline(existing.point, existing.tangent, [target])
      existing.spline = spline
      existing.startedAt = 0
      existing.duration = travelDurationMs(spline.totalLength, c.dwellBudgetMs)
      existing.target = target
      installedSpline = true
    }
    const active = new Set(cursors.map((c) => c.sessionId))
    for (const id of anims.keys()) {
      if (!active.has(id)) anims.delete(id)
    }
    if (installedSpline && rafIdRef.current === 0) {
      const tick = () => {
        let advanced = false
        let stillLive = false
        const now = performance.now()
        for (const anim of animsRef.current.values()) {
          if (!anim.spline) continue
          if (anim.startedAt === 0) anim.startedAt = now
          const progress =
            anim.duration <= 0
              ? 1
              : Math.min(1, (now - anim.startedAt) / anim.duration)
          const sample = anim.spline.sampleT(
            easeAt(DEFAULT_CURSOR_MOTION.easing, progress),
          )
          anim.point = sample.position
          anim.tangent = sample.tangent
          if (progress >= 1) {
            anim.point = anim.target
            anim.spline = null
          }
          advanced = true
          if (anim.spline) stillLive = true
        }
        if (advanced) setTick((t) => t + 1)
        rafIdRef.current = stillLive ? requestAnimationFrame(tick) : 0
      }
      rafIdRef.current = requestAnimationFrame(tick)
    }
  }, [cursors])

  useEffect(
    () => () => {
      if (rafIdRef.current !== 0) {
        cancelAnimationFrame(rafIdRef.current)
        rafIdRef.current = 0
      }
    },
    [],
  )

  return cursors.map((c) => {
    const anim = animsRef.current.get(c.sessionId)
    return {
      cursor: c,
      point: anim?.point ?? { x: c.canvasX, y: c.canvasY },
      isAnimating: !!anim?.spline,
    }
  })
}

export function AgentCursorLayer({
  cursors,
  pages,
  canvasOrigin,
  pan,
  zoom,
  overlayOffsetY = 0,
}: {
  cursors: AgentPresenceCursor[]
  pages: CanvasScenePageEntity[]
  canvasOrigin: { x: number; y: number }
  pan: { x: number; y: number }
  zoom: number
  overlayOffsetY?: number
}) {
  const animated = useAnimatedCursors(cursors)

  // animated is a fresh array per render, so memoizing trailCursors would
  // invalidate every tick — just compute inline.
  const trailCursors: PresenceParticleCursor[] = animated.map(
    ({ cursor, point, isAnimating }) => ({
      id: cursor.sessionId,
      x: canvasOrigin.x + pan.x + point.x * zoom + CURSOR_TRAIL_OFFSET.x,
      y:
        canvasOrigin.y +
        pan.y -
        overlayOffsetY +
        point.y * zoom +
        CURSOR_TRAIL_OFFSET.y,
      color: cursor.color,
      intensity: isAnimating ? 1 : 0,
    }),
  )

  if (cursors.length === 0) return null

  return (
    <div
      className="pointer-events-none absolute inset-0"
      style={{ zIndex: 9999 }}
    >
      <style>
        {`@keyframes agent-presence-pulse { 0%, 100% { transform: scale(1); } 50% { transform: scale(1.05); } }
@keyframes agent-click-ripple { 0% { transform: scale(0); opacity: 0.6; } 100% { transform: scale(1); opacity: 0; } }`}
      </style>
      <PresenceParticleTrail cursors={trailCursors} />
      {cursors.map((cursor) => (
        <TargetHalo
          key={`halo-${cursor.sessionId}`}
          cursor={cursor}
          page={
            cursor.pageId
              ? (pages.find((page) => page.id === cursor.pageId) ?? null)
              : null
          }
          overlayOffsetY={overlayOffsetY}
        />
      ))}
      <div
        className="absolute left-0 top-0 origin-top-left"
        style={{
          transform: `translate(${canvasOrigin.x + pan.x}px, ${canvasOrigin.y + pan.y - overlayOffsetY}px) scale(${zoom})`,
        }}
      >
        {animated.map(({ cursor, point }) => (
          <AgentCursor
            key={cursor.sessionId}
            cursor={cursor}
            point={point}
            zoom={zoom}
          />
        ))}
      </div>
    </div>
  )
}
