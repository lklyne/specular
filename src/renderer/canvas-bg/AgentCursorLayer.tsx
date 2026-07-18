import { type CSSProperties, useMemo, useEffect, useRef, useState } from 'react'
import type {
  AgentPresenceCursor,
  CanvasScenePageEntity,
  PresenceActivity,
} from '../../shared/types'
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
import { PRESENCE_STEP_DELAY_MS } from '../../shared/presence-timing'
import { ambientDriftOffset, sessionAmbientSeed } from '../../shared/presence-ambient'
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
    case 'refused':
      // A refused mirrored click (ADR 0030): a brief lateral shake, exactly
      // one meaning ("can't do that here"). `forwards` holds the keyframe's
      // final frame (already at rest) instead of popping back to the base
      // `transform` above the instant the one-shot animation ends.
      return {
        opacity: 1,
        transform: 'scale(1.02)',
        filter: 'saturate(1.15)',
        animation: 'synced-cursor-wiggle 360ms ease-in-out 1',
        animationFillMode: 'forwards',
      }
  }
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
  const positionStyle: CSSProperties = useMemo(
    () => ({
      left: 0,
      top: 0,
      transform: `translate3d(${point.x}px, ${point.y}px, 0)`,
      willChange: 'transform',
    }),
    [point.x, point.y],
  )

  // Counter-scale keeps the icon at constant screen size
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
          <FilledCursorIcon color={cursor.color} size={24} />
        </div>
      </div>
    </div>
  )
}

interface CursorAnim {
  point: Vec2
  tangent: Vec2
  spline: ReturnType<typeof foldSpline> | null
  startedAt: number
  duration: number
  target: Vec2
  // Ambient drift (issue #319 Phase 3) — a visual-only offset composited on
  // top of `point`, never fed back into it. `seed` is stable per session so
  // a cursor's wander is reproducible; `ambientModeStartedAt` is the RAF
  // clock time the mode most recently switched on, so `ambientDriftOffset`
  // always starts its ramp from zero instead of popping onto an arbitrary
  // point on the wander curve.
  seed: number
  ambientMode: AgentPresenceCursor['ambientMode']
  ambientModeStartedAt: number
  // Client-side analogue of the server's `lastMoveAt` (issue #319 Phase 5):
  // stamped when the target actually changes, left untouched when a
  // broadcast re-arrives at the same target (server-side reposition skip,
  // ADR 0029 amortization) — so it tracks the same "since when has the
  // cursor been heading here" clock the ripple delay reads.
  repositionedAt: number
}

interface AnimatedCursor {
  cursor: AgentPresenceCursor
  point: Vec2
  isAnimating: boolean
  repositionedAt: number
}

// Drives one RAF for all presence cursors so the DOM icon and particle trail
// read from the same interpolated positions. Target changes from the server
// start a new spline from the current (position, tangent). The RAF only runs
// while at least one spline is active, so a steady-state canvas idles.
function useAnimatedCursors(cursors: AgentPresenceCursor[]): AnimatedCursor[] {
  const animsRef = useRef<Map<string, CursorAnim>>(new Map())
  const rafIdRef = useRef(0)
  const [, setTick] = useState(0)

  // One RAF loop drives spline travel and ambient drift for every cursor by
  // design (single interpolation clock); splitting it would fork that clock.
  // fallow-ignore-next-line complexity
  useEffect(() => {
    const anims = animsRef.current
    let needsRaf = false
    const rafNow = performance.now()
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
          seed: sessionAmbientSeed(c.sessionId),
          ambientMode: c.ambientMode,
          ambientModeStartedAt: c.ambientMode !== 'none' ? rafNow : 0,
          repositionedAt: rafNow,
        })
        if (c.ambientMode !== 'none') needsRaf = true
        continue
      }
      // Real motion always wins: entering/leaving/switching ambient modes
      // just restarts the wander clock, never touches `point`/`spline`, so
      // a spline in flight is never fought (ADR 0029: never retro-animate,
      // never fight the spline).
      if (existing.ambientMode !== c.ambientMode) {
        existing.ambientMode = c.ambientMode
        existing.ambientModeStartedAt = c.ambientMode !== 'none' ? rafNow : 0
      }
      if (existing.ambientMode !== 'none') needsRaf = true
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
      existing.repositionedAt = rafNow
      needsRaf = true
    }
    const active = new Set(cursors.map((c) => c.sessionId))
    for (const id of anims.keys()) {
      if (!active.has(id)) anims.delete(id)
    }
    if (needsRaf && rafIdRef.current === 0) {
      const tick = () => {
        let advanced = false
        let stillLive = false
        const now = performance.now()
        for (const anim of animsRef.current.values()) {
          if (anim.spline) {
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
          }
          if (anim.spline) stillLive = true
          // Ambient drift has no terminal state — it keeps the RAF loop
          // alive for as long as the cursor sits in the inter-command gap,
          // and every tick is a re-render so the offset (computed fresh
          // below from `performance.now()`) stays current.
          if (anim.ambientMode !== 'none') {
            advanced = true
            stillLive = true
          }
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
    const base = anim?.point ?? { x: c.canvasX, y: c.canvasY }
    // Ambient drift composites visually on top of the truthful spline
    // position and is never fed back into `anim.point` or any server call —
    // ADR 0029 rule 4 (no speculative pre-positioning) and the dwell budget
    // (`waitForPresenceDwell` in app-control-server.ts) both depend on the
    // real `canvasX`/`canvasY` never being touched by this.
    const drift = anim
      ? ambientDriftOffset(anim.seed, performance.now() - anim.ambientModeStartedAt, anim.ambientMode)
      : { x: 0, y: 0 }
    return {
      cursor: c,
      point: { x: base.x + drift.x, y: base.y + drift.y },
      isAnimating: !!anim?.spline,
      repositionedAt: anim?.repositionedAt ?? performance.now(),
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
@keyframes synced-cursor-wiggle {
  0% { transform: translateX(0) scale(1.02); }
  20% { transform: translateX(-5px) scale(1.02); }
  40% { transform: translateX(5px) scale(1.02); }
  60% { transform: translateX(-3px) scale(1.02); }
  80% { transform: translateX(2px) scale(1.02); }
  100% { transform: translateX(0) scale(1); }
}`}
      </style>
      <PresenceParticleTrail cursors={trailCursors} />
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
