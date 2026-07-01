import { useEffect, useRef, useState } from 'react'
import type { LayoutUpdateData } from '../../shared/types'

// ~2s of frames at 120fps — long enough that a spike stays readable.
const RING = 240
const FRAME_BUDGET_MS = 1000 / 60

function pushCapped(buf: number[], value: number): void {
  buf.push(value)
  if (buf.length > RING) buf.shift()
}

function maxOf(values: number[]): number {
  let m = 0
  for (const v of values) if (v > m) m = v
  return m
}

function Spark({
  values,
  ceiling,
  target,
  width = 148,
  height = 26,
}: {
  values: number[]
  ceiling: number
  target: number
  width?: number
  height?: number
}) {
  const hi = Math.max(ceiling, maxOf(values))
  const step = width / (RING - 1)
  const points = values
    .map((v, i) => `${(i * step).toFixed(1)},${(height - (v / hi) * height).toFixed(1)}`)
    .join(' ')
  const targetY = height - (target / hi) * height
  return (
    <svg width={width} height={height} className="block text-zinc-400 dark:text-zinc-500">
      <line
        x1={0}
        y1={targetY}
        x2={width}
        y2={targetY}
        stroke="currentColor"
        strokeOpacity={0.5}
        strokeDasharray="2 2"
      />
      {values.length > 1 ? (
        <polyline
          points={points}
          fill="none"
          stroke="currentColor"
          strokeWidth={1}
          className="text-zinc-700 dark:text-zinc-200"
        />
      ) : null}
    </svg>
  )
}

// A single slow frame is what reads as a hitch — colour by the worst recent
// frame, not the average. Thresholds are display-agnostic: a >2-frame stall is
// visible on any refresh rate.
function hitchColor(worstFrameMs: number): string {
  if (worstFrameMs <= FRAME_BUDGET_MS * 1.5) return 'text-emerald-600 dark:text-emerald-400'
  if (worstFrameMs <= FRAME_BUDGET_MS * 3) return 'text-amber-500'
  return 'text-red-500'
}

/**
 * Dev-only heads-up display for canvas rendering cost. Left readout measures
 * real renderer frame intervals (rAF); right readout charts the main-process
 * `buildCanvasLayoutData` duration stamped onto each layout-update. Peaks are
 * held over the sample window so a spike during pan/zoom stays readable, and
 * the two columns discriminate the bottleneck: a build spike points at the
 * O(entities) rebuild (#265 Option B), a frame spike with a flat build points
 * at renderer churn instead. Click the pill to expand the sparklines. Never a
 * pointer target except the collapsed pill.
 */
export function PerfHudOverlay({
  isDev,
  layoutData,
}: {
  isDev: boolean
  layoutData: LayoutUpdateData
}) {
  const [expanded, setExpanded] = useState(false)
  const [, forceTick] = useState(0)
  const frameMs = useRef<number[]>([])
  const buildMs = useRef<number[]>([])
  const lastFrameTs = useRef(0)

  // Off by default; opt in per-machine with `localStorage.perfHud = '1'` in the
  // canvas devtools console, then reload. Dev-only tool, dev-only switch — not
  // worth the cross-window preference plumbing a UI toggle would need.
  const [enabled] = useState(() => {
    try {
      return localStorage.getItem('perfHud') === '1'
    } catch {
      return false
    }
  })
  const active = isDev && enabled

  // Sample every real frame into a ring buffer; never setState here — that
  // would re-render 120x/sec and pollute the very numbers we're measuring.
  useEffect(() => {
    if (!active) return
    let raf = 0
    const loop = (ts: number) => {
      if (lastFrameTs.current) pushCapped(frameMs.current, ts - lastFrameTs.current)
      lastFrameTs.current = ts
      raf = requestAnimationFrame(loop)
    }
    raf = requestAnimationFrame(loop)
    return () => cancelAnimationFrame(raf)
  }, [active])

  // Repaint the readout at a fixed 4Hz, decoupled from the frame loop.
  useEffect(() => {
    if (!active) return
    const id = setInterval(() => forceTick((n) => n + 1), 250)
    return () => clearInterval(id)
  }, [active])

  // Record each layout build cost as its payload lands.
  useEffect(() => {
    if (layoutData.buildMs != null) pushCapped(buildMs.current, layoutData.buildMs)
  }, [layoutData])

  if (!active) return null

  const frames = frameMs.current
  const recentFrames = frames.slice(-15)
  const avgFrame = recentFrames.length
    ? recentFrames.reduce((a, b) => a + b, 0) / recentFrames.length
    : 0
  const fps = avgFrame ? Math.round(1000 / avgFrame) : 0
  const worstFrame = maxOf(frames) // held over the ~2s window

  const builds = buildMs.current
  const lastBuild = builds.length ? builds[builds.length - 1] : 0
  const peakBuild = maxOf(builds)

  return (
    <div className="pointer-events-none absolute left-2 top-10 z-[71] select-none font-mono text-[10px] text-zinc-700 dark:text-zinc-200">
      <div className="pointer-events-auto inline-block rounded border border-zinc-300/80 bg-white/90 shadow dark:border-zinc-600 dark:bg-zinc-900/90">
        <button
          type="button"
          className="flex w-full items-center gap-2 px-2 py-1"
          onClick={() => setExpanded((e) => !e)}
        >
          <span>{fps} fps</span>
          <span className="text-zinc-400">·</span>
          {/* Worst frame in the window — the number that tracks felt jitter. */}
          <span className={hitchColor(worstFrame)}>hitch {worstFrame.toFixed(0)}ms</span>
        </button>
        {expanded ? (
          <div className="border-t border-zinc-200 px-2 pb-2 pt-1 dark:border-zinc-700">
            <div className="flex justify-between gap-3">
              <span>frame</span>
              <span className={hitchColor(worstFrame)}>peak {worstFrame.toFixed(1)}ms</span>
            </div>
            <Spark values={frames} ceiling={FRAME_BUDGET_MS * 2} target={FRAME_BUDGET_MS} />
            <div className="mt-1 flex justify-between gap-3">
              <span>build</span>
              <span>
                {lastBuild.toFixed(1)} · peak {peakBuild.toFixed(1)}ms
              </span>
            </div>
            <Spark values={builds} ceiling={FRAME_BUDGET_MS} target={FRAME_BUDGET_MS} />
          </div>
        ) : null}
      </div>
    </div>
  )
}
