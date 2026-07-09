import { useCallback, useEffect, useRef, useState } from 'react'
import { isOverlayUiTarget } from '../../shared/gesture-utils'
import type { PresenceParticleCursor } from '../shared/PresenceParticleTrail'

// Classic laser red. Fed as the trail cursor's color, so the whole beam picks
// it up (the particle system tints per-cursor).
const LASER_COLOR = '#ff2d2d'

/**
 * Laser-pointer drag gesture. Reuses the agent-presence particle trail
 * (`PresenceParticleTrail`), but instead of tracking a broadcast cursor it
 * tracks a live pointer drag: while the primary button is held, the current
 * pointer position is emitted into the trail at full intensity, drawing a
 * dissipating beam. Nothing is persisted — releasing drops the cursor and the
 * particles fade on their own.
 *
 * The point is fed in raw client coords, which line up 1:1 with the trail's
 * canvas (both live in the aboveView DOM, outside the pan transform).
 */
export function useLaserPointerGesture({ enabled }: { enabled: boolean }) {
  const [point, setPoint] = useState<{ x: number; y: number } | null>(null)
  const [active, setActive] = useState(false)
  const pointerIdRef = useRef<number | null>(null)

  // Leaving laser mode mid-drag: drop the cursor so a stray in-flight stroke
  // doesn't linger, and reset for the next activation.
  useEffect(() => {
    if (enabled) return
    pointerIdRef.current = null
    setActive(false)
    setPoint(null)
  }, [enabled])

  const onPointerDown = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (!enabled) return
      if (event.pointerType === 'mouse' && event.button !== 0) return
      if (isOverlayUiTarget(event.target)) return
      pointerIdRef.current = event.pointerId
      setPoint({ x: event.clientX, y: event.clientY })
      setActive(true)
      event.currentTarget.setPointerCapture(event.pointerId)
      event.preventDefault()
    },
    [enabled],
  )

  const onPointerMove = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    if (pointerIdRef.current !== event.pointerId) return
    setPoint({ x: event.clientX, y: event.clientY })
  }, [])

  const endStroke = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    if (pointerIdRef.current !== event.pointerId) return
    pointerIdRef.current = null
    setActive(false)
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId)
    }
  }, [])

  // Only present while dragging. Dropping the cursor on release lets the next
  // stroke re-appear as a fresh id (the trail snaps prev=curr on first sight,
  // so a far-apart new stroke doesn't streak a line from the last release).
  const laserCursors: PresenceParticleCursor[] =
    active && point
      ? [{ id: 'laser', x: point.x, y: point.y, color: LASER_COLOR, intensity: 1 }]
      : []

  return {
    laserCursors,
    handleLaserPointerDown: onPointerDown,
    handleLaserPointerMove: onPointerMove,
    handleLaserPointerUp: endStroke,
    handleLaserPointerCancel: endStroke,
  }
}
