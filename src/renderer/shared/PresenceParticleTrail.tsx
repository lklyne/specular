// @ts-nocheck — TSL node types are intentionally loose.

import { useEffect, useRef } from 'react'
import * as THREE from 'three/webgpu'
import {
  Fn,
  If,
  float,
  hash,
  instanceIndex,
  instancedArray,
  mix,
  mx_noise_vec3,
  positionLocal,
  smoothstep,
  uint,
  uniform,
  uniformArray,
  uv,
  vec2,
  vec3,
  vec4,
} from 'three/tsl'

const MAX_POOL_SIZE = 16384
const MAX_CURSORS = 8
// Hardcoded dispatch ceiling. The actual emits-per-page is a runtime uniform
// and may be anything in [2, EMIT_PER_FRAME_MAX].
const EMIT_PER_FRAME_MAX = 16

// Screen-space offset from the cursor's translate origin to the tip of the
// FilledCursorIcon, so trail particles emit from the tip rather than the
// top-left of the icon. Shared by AgentCursorLayer (production) and
// PresencePlayground (debug defaults).
export const CURSOR_TRAIL_OFFSET = { x: 12, y: 16 } as const

export interface PresenceParticleCursor {
  id: string
  x: number
  y: number
  color: string
  intensity: number
}

interface Props {
  cursors: PresenceParticleCursor[]
  holdSeconds?: number
  lifetimeSeconds?: number
  size?: number
  /** Peak px/s of random drift (reached when particle is >= referenceDistance from cursor). */
  driftStrength?: number
  /** Half-width in px of the random spawn scatter around the emit point. 0 = spawn exactly on the line. */
  spawnJitter?: number
  /** Distance in px at which drift smoothstep-eases up to max. */
  driftReferenceDistance?: number
  /** How fast each particle's drift direction evolves over time. 0 = fixed direction; larger = more swirling. */
  driftTurnRate?: number
  /** Spatial coherence of the drift flow field. 0 = every particle independent; higher = neighbors drift together. */
  driftFlowScale?: number
  /** Active slots in the ring buffer; clamped to [64, MAX_POOL_SIZE]. */
  particleCount?: number
  /** Seconds a cursor must sit idle (intensity <= 0) before its trail starts a global fade. */
  fadeOutGraceSeconds?: number
  /** Duration of the global fade from full to invisible, in seconds. */
  fadeOutSeconds?: number
  /** Easing curve applied to the global fade. */
  fadeOutEasing?: FadeEasing
  /** Cursor speed (px/s) at which emission reaches full rate. */
  emitSpeedReferencePxPerSec?: number
  /** Power curve on the speed→emit mapping. >1 biases emission to the fast middle. */
  emitSpeedBias?: number
  /** Max particles spawned per cursor per page. Clamped to [2, 16]. */
  emitsPerFrame?: number
}

type FadeEasing = 'linear' | 'ease-in' | 'ease-out' | 'ease-in-out'

function applyEase(t: number, kind: FadeEasing): number {
  if (t <= 0) return 0
  if (t >= 1) return 1
  switch (kind) {
    case 'ease-in':
      return t * t
    case 'ease-out':
      return 1 - (1 - t) * (1 - t)
    case 'ease-in-out':
      return t * t * (3 - 2 * t)
    default:
      return t
  }
}

interface Handle {
  pushCursors: (cursors: PresenceParticleCursor[]) => void
  setParams: (p: {
    holdSeconds: number
    lifetimeSeconds: number
    size: number
    driftStrength: number
    spawnJitter: number
    driftReferenceDistance: number
    driftTurnRate: number
    driftFlowScale: number
    particleCount: number
    fadeOutGraceSeconds: number
    fadeOutSeconds: number
    fadeOutEasing: FadeEasing
    emitSpeedReferencePxPerSec: number
    emitSpeedBias: number
    emitsPerFrame: number
  }) => void
}

function buildSystem(initial: {
  holdSeconds: number
  lifetimeSeconds: number
  size: number
  driftStrength: number
  spawnJitter: number
  driftReferenceDistance: number
  driftTurnRate: number
  driftFlowScale: number
  particleCount: number
  fadeOutGraceSeconds: number
  fadeOutSeconds: number
  fadeOutEasing: FadeEasing
  emitSpeedReferencePxPerSec: number
  emitSpeedBias: number
  emitsPerFrame: number
}) {
  const positionBuffer = instancedArray(MAX_POOL_SIZE, 'vec3') // (x, y, 0)
  const stateBuffer = instancedArray(MAX_POOL_SIZE, 'vec3')    // (age, cursorIdx, unused)
  const velocityBuffer = instancedArray(MAX_POOL_SIZE, 'vec3') // (vx, vy, 0)

  const cursorPos = uniformArray(
    Array.from({ length: MAX_CURSORS }, () => new THREE.Vector2(-10000, -10000)),
  )
  const cursorPrevPos = uniformArray(
    Array.from({ length: MAX_CURSORS }, () => new THREE.Vector2(-10000, -10000)),
  )
  const cursorColor = uniformArray(
    Array.from({ length: MAX_CURSORS }, () => new THREE.Color('#ffffff')),
  )
  const cursorIntensity = uniformArray(
    Array.from({ length: MAX_CURSORS }, () => 0),
  )
  // Per-cursor global fade multiplier, driven CPU-side. Multiplies the
  // per-particle age-based alpha, so we can collapse a resting-cursor trail
  // faster than its natural lifetime.
  const cursorFadeAlpha = uniformArray(
    Array.from({ length: MAX_CURSORS }, () => 1),
  )
  const cursorFadeStates = Array.from({ length: MAX_CURSORS }, () => ({
    idleElapsed: 0,
    alpha: 1,
  }))
  const fadeOutGraceU = uniform(initial.fadeOutGraceSeconds)
  const fadeOutSecondsU = uniform(initial.fadeOutSeconds)
  let fadeEasing: FadeEasing = initial.fadeOutEasing
  // Speed-gated emission. cursorEmitScale is a per-cursor 0..1 multiplier on
  // the spawn probability, computed CPU-side from prev→curr displacement.
  const cursorEmitScale = uniformArray(
    Array.from({ length: MAX_CURSORS }, () => 0),
  )
  const emitSpeedReferenceU = uniform(initial.emitSpeedReferencePxPerSec)
  const emitSpeedBiasU = uniform(initial.emitSpeedBias)
  const clampedInitialEmits = Math.max(
    2,
    Math.min(EMIT_PER_FRAME_MAX, Math.round(initial.emitsPerFrame)),
  )
  const emitPerFrameU = uniform(clampedInitialEmits, 'uint')
  const cursorCount = uniform(0, 'uint')
  const writeHead = uniform(0, 'uint')
  // Track which cursor ids we've seen so we can snap prev=curr on first
  // appearance (otherwise the first page interpolates from (-10000,-10000)).
  const knownIds = new Set<string>()
  const holdU = uniform(initial.holdSeconds)
  const lifetimeU = uniform(initial.lifetimeSeconds)
  const sizeU = uniform(initial.size)
  const driftStrengthU = uniform(initial.driftStrength)
  const spawnJitterU = uniform(initial.spawnJitter)
  const driftReferenceDistanceU = uniform(initial.driftReferenceDistance)
  const driftTurnRateU = uniform(initial.driftTurnRate)
  const driftFlowScaleU = uniform(initial.driftFlowScale)
  const timeU = uniform(0)
  const poolSizeU = uniform(
    Math.max(64, Math.min(MAX_POOL_SIZE, initial.particleCount)),
    'uint',
  )
  const deltaU = uniform(1 / 60) // CPU-driven; compute shaders don't see built-in deltaTime

  // --- Kernels

  const initKernel = Fn(() => {
    positionBuffer.element(instanceIndex).assign(vec3(float(-10000), float(-10000), float(0)))
    stateBuffer.element(instanceIndex).assign(vec3(lifetimeU.add(1), float(0), float(0)))
    velocityBuffer.element(instanceIndex).assign(vec3(0, 0, 0))
  })().compute(MAX_POOL_SIZE)

  const perCursorU = emitPerFrameU
  const perCursorFloat = float(emitPerFrameU)
  const spawnKernel = Fn(() => {
    const cIdx = instanceIndex.div(perCursorU).toVar()
    If(cIdx.lessThan(cursorCount), () => {
      const gate = hash(instanceIndex.add(writeHead).add(uint(101)))
      const threshold = cursorIntensity.element(cIdx).mul(cursorEmitScale.element(cIdx))
      If(gate.lessThan(threshold), () => {
        const slot = writeHead.add(instanceIndex).mod(poolSizeU)

        // Distribute emissions along the segment prev→curr so fast motion
        // leaves a tight line instead of a gap. Local index within the
        // cursor's batch: 0..EMIT_PER_FRAME-1.
        const localIdx = instanceIndex.mod(perCursorU)
        const t = float(localIdx).div(perCursorFloat.sub(1))
        const prev = cursorPrevPos.element(cIdx)
        const curr = cursorPos.element(cIdx)
        const originX = mix(prev.x, curr.x, t)
        const originY = mix(prev.y, curr.y, t)

        const jitterSeed = writeHead.add(instanceIndex.mul(uint(17))).add(uint(7))
        const jx = hash(jitterSeed).sub(0.5).mul(2.0).mul(spawnJitterU)
        const jy = hash(jitterSeed.add(uint(53))).sub(0.5).mul(2.0).mul(spawnJitterU)
        positionBuffer
          .element(slot)
          .assign(vec3(originX.add(jx), originY.add(jy), float(0)))
        stateBuffer.element(slot).assign(vec3(float(0), float(cIdx), float(0)))
        velocityBuffer.element(slot).assign(vec3(0, 0, 0))
      })
    })
  })().compute(EMIT_PER_FRAME_MAX * MAX_CURSORS)

  const simulateKernel = Fn(() => {
    const pos = positionBuffer.element(instanceIndex)
    const vel = velocityBuffer.element(instanceIndex)
    const state = stateBuffer.element(instanceIndex)
    const age = state.x.add(deltaU).toVar()

    If(age.greaterThan(lifetimeU), () => {
      pos.assign(vec3(float(-10000), float(-10000), float(0)))
      state.x.assign(lifetimeU)
    }).Else(() => {
      If(age.greaterThan(holdU), () => {
        // Direction is sampled from a noise field at (seed + pos*flowScale,
        // time*turnRate). turnRate controls how fast each particle's
        // direction evolves; flowScale controls how much neighbors share a
        // direction (0 = fully independent random walk, >0 = flow field).
        const seed = float(instanceIndex).mul(0.137)
        const t = timeU.mul(driftTurnRateU)
        const nx = seed.add(pos.x.mul(driftFlowScaleU))
        const ny = seed.add(float(31.7)).add(pos.y.mul(driftFlowScaleU))
        const n = mx_noise_vec3(vec3(nx, ny, t))
        const dir = vec3(n.x, n.y, float(0))

        // Drift strength eases from 0 (near cursor) to max (far from cursor).
        // This means particles clustered at a resting cursor stay calm, while
        // particles at the far end of a motion trail scatter.
        const cIdx = uint(state.y)
        const cursor = cursorPos.element(cIdx)
        const deltaX = pos.x.sub(cursor.x)
        const deltaY = pos.y.sub(cursor.y)
        const dist = deltaX.mul(deltaX).add(deltaY.mul(deltaY)).sqrt()
        const distFactor = smoothstep(0, driftReferenceDistanceU, dist)
        const strength = driftStrengthU.mul(distFactor)
        vel.assign(dir.mul(strength))
      })
      pos.addAssign(vel.mul(deltaU))
      state.x.assign(age)
    })
  })().compute(MAX_POOL_SIZE)

  // Manual per-vertex offset; SpriteNodeMaterial billboarding is unnecessary
  // under an orthographic pixel camera (planes at z=0 already face -Z).
  const material = new THREE.MeshBasicNodeMaterial({
    transparent: true,
    depthWrite: false,
    blending: THREE.NormalBlending,
    side: THREE.DoubleSide,
  })

  material.positionNode = Fn(() => {
    const state = stateBuffer.toAttribute()
    const center = positionBuffer.toAttribute()
    const lifeT = state.x.div(lifetimeU).clamp(0, 1)
    const shrink = float(1).sub(smoothstep(0.4, 1, lifeT))
    const perVertexSize = sizeU.mul(shrink.max(0.15))
    return positionLocal.mul(perVertexSize).add(center)
  })()

  material.colorNode = Fn(() => {
    const state = stateBuffer.toAttribute()
    const age = state.x
    const cIdx = uint(state.y)
    const base = cursorColor.element(cIdx)
    const lifeT = age.div(lifetimeU).clamp(0, 1)
    const fadeIn = smoothstep(0, 0.05, lifeT)
    const ageFade = float(1).sub(smoothstep(0.3, 1, lifeT))
    // Collapse as a front that descends through age, not a flat multiply: as
    // the per-cursor fade alpha drops (release / rest), the front sweeps from
    // old to young so the stroke dissipates from the end drawn first instead
    // of dimming evenly. front=1 (active) leaves everything lit.
    const SWEEP_BAND = 0.2
    const front = cursorFadeAlpha.element(cIdx).mul(1 + SWEEP_BAND).sub(SWEEP_BAND)
    const sweep = float(1).sub(smoothstep(front, front.add(SWEEP_BAND), lifeT))
    // Radial mask: solid core, smoothstep to 0 before the quad edge so the
    // corners are fully transparent and the particle reads as a round point.
    const dist = uv().sub(vec2(0.5, 0.5)).length()
    const radial = float(1).sub(smoothstep(0.1, 0.5, dist))
    return vec4(base, fadeIn.mul(ageFade).mul(sweep).mul(radial))
  })()

  const geometry = new THREE.PlaneGeometry(1, 1)
  const mesh = new THREE.InstancedMesh(geometry, material, MAX_POOL_SIZE)
  mesh.frustumCulled = false
  mesh.count = Math.max(64, Math.min(MAX_POOL_SIZE, initial.particleCount))

  const scene = new THREE.Scene()
  scene.add(mesh)

  const camera = new THREE.OrthographicCamera(0, 1, 0, 1, -100, 100)
  camera.position.z = 10

  let writeHeadJS = 0
  let activePoolSize = mesh.count

  const stepCompute = (renderer: THREE.WebGPURenderer, dtSeconds: number) => {
    const dt = Math.min(Math.max(dtSeconds, 0), 1 / 15)
    deltaU.value = dt
    timeU.value += dt

    const activeCount = cursorCount.value
    const fadeDuration = Math.max(fadeOutSecondsU.value, 0.0001)
    const emitRef = Math.max(emitSpeedReferenceU.value, 0.01)
    const emitBias = emitSpeedBiasU.value
    const dtForSpeed = Math.max(dt, 1 / 240)
    for (let i = 0; i < MAX_CURSORS; i++) {
      const s = cursorFadeStates[i]
      const active = i < activeCount && cursorIntensity.array[i] > 0
      if (active) {
        s.idleElapsed = 0
        s.alpha = 1
      } else {
        s.idleElapsed += dt
        const fadeT = Math.min(
          Math.max((s.idleElapsed - fadeOutGraceU.value) / fadeDuration, 0),
          1,
        )
        s.alpha = 1 - applyEase(fadeT, fadeEasing)
      }
      cursorFadeAlpha.array[i] = s.alpha

      if (i < activeCount) {
        const curr = cursorPos.array[i]
        const prev = cursorPrevPos.array[i]
        const dx = curr.x - prev.x
        const dy = curr.y - prev.y
        const speed = Math.sqrt(dx * dx + dy * dy) / dtForSpeed
        const normalized = Math.min(speed / emitRef, 1)
        cursorEmitScale.array[i] = Math.pow(normalized, emitBias)
      } else {
        cursorEmitScale.array[i] = 0
      }
    }

    renderer.compute(spawnKernel)
    renderer.compute(simulateKernel)
    // Advance past only the slots this frame's cursors actually claimed
    // (cIdx<cursorCount, so instanceIndex<cursorCount*emitPerFrame). Advancing
    // by MAX_CURSORS instead would lap the pool 8× faster for a single cursor,
    // overwriting still-alive particles and making long lifetimes jank.
    const slotsUsed = Math.max(cursorCount.value, 1) * emitPerFrameU.value
    writeHeadJS = (writeHeadJS + slotsUsed) % activePoolSize
    writeHead.value = writeHeadJS
    // After dispatch, current page's cursor positions become "previous"
    // for the next page's segment interpolation.
    for (let i = 0; i < cursorCount.value; i++) {
      cursorPrevPos.array[i].copy(cursorPos.array[i])
    }
  }

  const init = (renderer: THREE.WebGPURenderer) => {
    renderer.compute(initKernel)
  }

  const resize = (w: number, h: number) => {
    camera.left = 0
    camera.right = w
    camera.top = 0
    camera.bottom = h
    camera.updateProjectionMatrix()
  }

  const handle: Handle = {
    pushCursors(cursors) {
      const n = Math.min(cursors.length, MAX_CURSORS)
      cursorCount.value = n
      const seen = new Set<string>()
      for (let i = 0; i < n; i++) {
        const c = cursors[i]
        seen.add(c.id)
        // First time we see this id, snap prev=curr so the first emission
        // doesn't interpolate from a stale/off-screen previous position.
        if (!knownIds.has(c.id)) {
          cursorPrevPos.array[i].set(c.x, c.y)
          knownIds.add(c.id)
        }
        cursorPos.array[i].set(c.x, c.y)
        cursorColor.array[i].set(c.color)
        cursorIntensity.array[i] = c.intensity
      }
      for (const id of knownIds) {
        if (!seen.has(id)) knownIds.delete(id)
      }
    },
    setParams(p) {
      holdU.value = p.holdSeconds
      lifetimeU.value = p.lifetimeSeconds
      sizeU.value = p.size
      driftStrengthU.value = p.driftStrength
      spawnJitterU.value = p.spawnJitter
      driftReferenceDistanceU.value = p.driftReferenceDistance
      driftTurnRateU.value = p.driftTurnRate
      driftFlowScaleU.value = p.driftFlowScale
      fadeOutGraceU.value = p.fadeOutGraceSeconds
      fadeOutSecondsU.value = p.fadeOutSeconds
      fadeEasing = p.fadeOutEasing
      emitSpeedReferenceU.value = p.emitSpeedReferencePxPerSec
      emitSpeedBiasU.value = p.emitSpeedBias
      emitPerFrameU.value = Math.max(
        2,
        Math.min(EMIT_PER_FRAME_MAX, Math.round(p.emitsPerFrame)),
      )
      const next = Math.max(64, Math.min(MAX_POOL_SIZE, p.particleCount))
      if (next !== activePoolSize) {
        activePoolSize = next
        poolSizeU.value = next
        mesh.count = next
        writeHeadJS = writeHeadJS % next
        writeHead.value = writeHeadJS
      }
    },
  }

  const dispose = () => {
    geometry.dispose()
    material.dispose()
  }

  return { scene, camera, stepCompute, init, resize, handle, dispose }
}

export function PresenceParticleTrail({
  cursors,
  holdSeconds = 0.3,
  lifetimeSeconds = 2.5,
  size = 4,
  driftStrength = 30,
  spawnJitter = 1,
  driftReferenceDistance = 180,
  driftTurnRate = 0.7,
  driftFlowScale = 0.001,
  particleCount = 8192,
  fadeOutGraceSeconds = 0.2,
  fadeOutSeconds = 1.2,
  fadeOutEasing = 'ease-in',
  emitSpeedReferencePxPerSec = 1250,
  emitSpeedBias = 2.35,
  emitsPerFrame = 16,
}: Props) {
  const hostRef = useRef<HTMLDivElement | null>(null)
  const handleRef = useRef<Handle | null>(null)

  useEffect(() => {
    const host = hostRef.current
    if (!host) return
    if (typeof navigator === 'undefined' || !('gpu' in navigator)) {
      // eslint-disable-next-line no-console
      console.warn('[trail] WebGPU not available')
      return
    }

    const canvas = document.createElement('canvas')
    canvas.style.width = '100%'
    canvas.style.height = '100%'
    canvas.style.display = 'block'
    host.appendChild(canvas)

    const renderer = new THREE.WebGPURenderer({ canvas, antialias: true, alpha: true })
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
    renderer.setClearColor(0x000000, 0)

    let rafId = 0
    let disposed = false
    let system: ReturnType<typeof buildSystem> | null = null

    const resize = () => {
      const w = host.clientWidth
      const h = host.clientHeight
      if (w === 0 || h === 0) return
      renderer.setSize(w, h, false)
      if (system) system.resize(w, h)
    }
    const ro = new ResizeObserver(resize)
    ro.observe(host)

    renderer
      .init()
      .then(() => {
        if (disposed) return
        system = buildSystem({
          holdSeconds,
          lifetimeSeconds,
          size,
          driftStrength,
          spawnJitter,
          driftReferenceDistance,
          driftTurnRate,
          driftFlowScale,
          particleCount,
          fadeOutGraceSeconds,
          fadeOutSeconds,
          fadeOutEasing,
          emitSpeedReferencePxPerSec,
          emitSpeedBias,
          emitsPerFrame,
        })
        handleRef.current = system.handle
        system.handle.pushCursors(cursors)
        system.init(renderer)
        resize()

        let lastT = performance.now()
        const tick = () => {
          if (disposed || !system) return
          const now = performance.now()
          const dtSeconds = (now - lastT) / 1000
          lastT = now
          system.stepCompute(renderer, dtSeconds)
          try {
            renderer.render(system.scene, system.camera)
          } catch (err) {
            // eslint-disable-next-line no-console
            console.error('[trail] render', err)
          }
          rafId = requestAnimationFrame(tick)
        }
        rafId = requestAnimationFrame(tick)
      })
      .catch((err) => {
        // eslint-disable-next-line no-console
        console.error('[trail] init failed', err)
      })

    return () => {
      disposed = true
      cancelAnimationFrame(rafId)
      ro.disconnect()
      system?.dispose()
      renderer.dispose()
      canvas.remove()
      handleRef.current = null
    }
  }, [])

  useEffect(() => {
    handleRef.current?.pushCursors(cursors)
  }, [cursors])

  useEffect(() => {
    handleRef.current?.setParams({
      holdSeconds,
      lifetimeSeconds,
      size,
      driftStrength,
      spawnJitter,
      driftReferenceDistance,
      driftTurnRate,
      driftFlowScale,
      particleCount,
      fadeOutGraceSeconds,
      fadeOutSeconds,
      fadeOutEasing,
      emitSpeedReferencePxPerSec,
      emitSpeedBias,
      emitsPerFrame,
    })
  }, [
    holdSeconds,
    lifetimeSeconds,
    size,
    driftStrength,
    spawnJitter,
    driftReferenceDistance,
    driftTurnRate,
    driftFlowScale,
    particleCount,
    fadeOutGraceSeconds,
    fadeOutSeconds,
    fadeOutEasing,
    emitSpeedReferencePxPerSec,
    emitSpeedBias,
    emitsPerFrame,
  ])

  return (
    <div
      ref={hostRef}
      className="pointer-events-none absolute inset-0"
      style={{ zIndex: 9998 }}
    />
  )
}
