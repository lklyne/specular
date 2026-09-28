// @ts-nocheck — TSL node types are intentionally loose.

import { useEffect, useRef } from 'react'
import * as THREE from 'three/webgpu'
import {
  Fn,
  Loop,
  dot,
  float,
  floor,
  fract,
  int,
  length,
  max,
  pow,
  smoothstep,
  screenCoordinate,
  sin,
  uniform,
  uniformArray,
  uv,
  vec2,
  vec3,
  vec4,
} from 'three/tsl'

/**
 * Mesh gradient with film grain, drifting rightward. Colour points sit
 * evenly around a horizontal loop and bob up and down; each pixel blends them
 * by inverse distance in OKLab. The strip shows a window onto that loop, so
 * the scroll never runs out. On the same beat as `Shimmer`, one extra colour
 * point sweeps left to right through the blend, taking the next palette colour
 * each pass. Adapted from the grain-gradient experiment on lyle-klyne.com.
 * Fills its nearest positioned ancestor.
 */

const DEFAULT_COLORS = ['#ff37d0', '#e4c4ff', '#8ea9ff', '#ffae00', '#ff7200']
// The panel strip is tiny and the motion is slow; 30fps is indistinguishable.
const FRAME_INTERVAL_MS = 1000 / 30
// Matches Shimmer's default sweep duration.
const PULSE_SECONDS = 2
// Loop units per second of steady drift, and extra loop units each pulse adds.
const DRIFT_SPEED = 0.05
const PULSE_PUSH = 0.06

const rgbToOklab = Fn(([rgb]) => {
  const l = dot(rgb, vec3(0.4122214708, 0.5363325363, 0.0514459929))
  const m = dot(rgb, vec3(0.2119034982, 0.6806995451, 0.1073969566))
  const s = dot(rgb, vec3(0.0883024619, 0.2817188376, 0.6299787005))
  const lms = pow(max(vec3(l, m, s), 0), vec3(1 / 3))
  return vec3(
    dot(lms, vec3(0.2104542553, 0.793617785, -0.0040720468)),
    dot(lms, vec3(1.9779984951, -2.428592205, 0.4505937099)),
    dot(lms, vec3(0.0259040371, 0.7827717662, -0.808675766)),
  )
})

const oklabToRgb = Fn(([lab]) => {
  const l_ = dot(lab, vec3(1, 0.3963377774, 0.2158037573))
  const m_ = dot(lab, vec3(1, -0.1055613458, -0.0638541728))
  const s_ = dot(lab, vec3(1, -0.0894841775, -1.291485548))
  const lms = vec3(l_, m_, s_).pow(3)
  return vec3(
    dot(lms, vec3(4.0767416621, -3.3077115913, 0.2309699292)),
    dot(lms, vec3(-1.2684380046, 2.6097574011, -0.3413193965)),
    dot(lms, vec3(-0.0041960863, -0.7034186147, 1.707614701)),
  )
})

const hash21 = Fn(([p]) => fract(sin(dot(p, vec2(12.9898, 78.233))).mul(43758.5453)))

function buildScene(colors: string[], span: number) {
  const time = uniform(0)
  const aspect = uniform(1)
  const colorArray = uniformArray(colors.map((hex) => new THREE.Color(hex)))
  const count = colors.length

  const colorNode = Fn(() => {
    const t = time.add(41.5)
    const beat = t.div(PULSE_SECONDS)
    const pass = fract(beat)
    // Steady drift plus an eased push per pulse, in step with the sweep.
    const scroll = t.mul(DRIFT_SPEED).add(floor(beat).add(smoothstep(0, 1, pass)).mul(PULSE_PUSH))
    // One loop unit spans 1/span strip widths; y keeps the same scale so
    // blobs stay round instead of stretching with the strip.
    const q = vec2(uv().x.mul(span).sub(scroll), uv().y.sub(0.5).mul(span).div(aspect)).toVar()
    q.y.addAssign(sin(q.x.mul(Math.PI * 4).add(t.mul(0.7))).mul(0.03))

    const lab = vec3(0).toVar()
    const total = float(0).toVar()
    Loop(count, ({ i }) => {
      const idx = float(i)
      const phase = idx.mul(2.1)
      const pos = vec2(
        idx.div(count).add(sin(t.mul(0.23).add(phase)).mul(0.06)),
        sin(t.mul(float(0.3).add(fract(idx.mul(0.37)).mul(0.3))).add(phase)).mul(0.12),
      )
      // Shortest distance around the loop, so the seam is invisible.
      const dx = fract(q.x.sub(pos.x).add(0.5)).sub(0.5)
      const weight = float(1).div(pow(length(vec2(dx, q.y.sub(pos.y))), 3.5).add(0.0001))
      lab.addAssign(rgbToOklab(colorArray.element(i)).mul(weight))
      total.addAssign(weight)
    })

    // The sweeping point, in strip space so it crosses at the same pace
    // whatever the drift. Its weight fades to zero at both ends of the pass,
    // so the colour change between passes never shows at the strip's edges.
    const sweepX = pass.mul(1.6).sub(0.3)
    const sweepColor = colorArray.element(int(floor(beat).mod(count)))
    const sweepD = length(vec2(uv().x.sub(sweepX).mul(span), q.y))
    const sweepWeight = sin(pass.mul(Math.PI)).div(pow(sweepD, 3.5).add(0.0001))
    lab.addAssign(rgbToOklab(sweepColor).mul(sweepWeight))
    total.addAssign(sweepWeight)

    const base = oklabToRgb(lab.div(max(total, 0.0001)))

    // Per-device-pixel grain, stable across frames so it reads as texture.
    const grain = hash21(screenCoordinate.xy).sub(0.5).mul(0.09)
    return vec4(base.add(grain).clamp(0, 1), 1)
  })()

  const material = new THREE.MeshBasicNodeMaterial()
  material.colorNode = colorNode
  const geometry = new THREE.PlaneGeometry(1, 1)
  const scene = new THREE.Scene()
  scene.add(new THREE.Mesh(geometry, material))
  const camera = new THREE.OrthographicCamera(-0.5, 0.5, 0.5, -0.5, -1, 1)

  return {
    scene,
    camera,
    time,
    aspect,
    dispose: () => {
      geometry.dispose()
      material.dispose()
    },
  }
}

export function GrainGradient({
  colors = DEFAULT_COLORS,
  speed = 1,
  span = 0.5,
  className,
}: {
  colors?: string[]
  speed?: number
  /** Fraction of the colour loop visible across the width; smaller = bigger blobs. */
  span?: number
  className?: string
}) {
  const hostRef = useRef<HTMLDivElement | null>(null)
  const colorKey = colors.join(',')

  useEffect(() => {
    const host = hostRef.current
    if (!host || !('gpu' in navigator)) return

    const canvas = document.createElement('canvas')
    canvas.style.cssText = 'position:absolute;inset:0;width:100%;height:100%;display:block'
    host.appendChild(canvas)

    const renderer = new THREE.WebGPURenderer({ canvas, alpha: true })
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
    const view = buildScene(colorKey.split(','), span)
    const still = window.matchMedia('(prefers-reduced-motion: reduce)').matches

    let rafId = 0
    let disposed = false
    let ready = false
    let last = 0

    const draw = () => {
      if (ready) renderer.render(view.scene, view.camera)
    }
    const resize = () => {
      const w = host.clientWidth
      const h = host.clientHeight
      if (w === 0 || h === 0) return
      renderer.setSize(w, h, false)
      view.aspect.value = w / h
      draw()
    }
    const ro = new ResizeObserver(resize)
    ro.observe(host)

    const tick = (now: number) => {
      if (disposed) return
      rafId = requestAnimationFrame(tick)
      if (now - last < FRAME_INTERVAL_MS) return
      view.time.value += (last ? now - last : 0) * 0.001 * speed
      last = now
      draw()
    }

    renderer
      .init()
      .then(() => {
        if (disposed) return
        ready = true
        resize()
        if (!still) rafId = requestAnimationFrame(tick)
      })
      .catch((err) => {
        console.error('[grain-gradient] init failed', err)
      })

    return () => {
      disposed = true
      cancelAnimationFrame(rafId)
      ro.disconnect()
      view.dispose()
      renderer.dispose()
      canvas.remove()
    }
  }, [colorKey, speed, span])

  return (
    <div
      ref={hostRef}
      aria-hidden
      className={`absolute inset-0 overflow-hidden ${className ?? ''}`}
      // Shown until WebGPU paints, and in place of it where there's no WebGPU.
      style={{ background: `linear-gradient(100deg, ${colors.join(', ')})` }}
    />
  )
}
