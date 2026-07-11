/**
 * Hand-rolled inline SVG chart primitives for the Performance debug section.
 * No chart libraries: scales are plain functions, marks are computed path
 * strings. Colors come from the theme-aware --perf-* CSS custom properties
 * defined in styles.css, scoped under the .perf-viz wrapper class — callers
 * are expected to render these inside that wrapper.
 */

import { useRef, useState, type PointerEvent, type ReactNode } from 'react'
import { formatMs } from './format'

export function scaleLinear(domain: [number, number], range: [number, number]) {
  const [d0, d1] = domain
  const [r0, r1] = range
  const span = d1 - d0 || 1
  return (value: number) => r0 + ((value - d0) / span) * (r1 - r0)
}

/** Rounds a domain max up to a "nice" 1/2/5×10^n step, so axis/gridline
 * labels don't land on values like 743.2. */
export function niceMax(value: number): number {
  if (!isFinite(value) || value <= 0) return 1
  const exp = Math.floor(Math.log10(value))
  const base = Math.pow(10, exp)
  const fraction = value / base
  const niceFraction = fraction <= 1 ? 1 : fraction <= 2 ? 2 : fraction <= 5 ? 5 : 10
  return niceFraction * base
}

/** Path for a bar rounded only on the data (right) end — square at the
 * baseline, matching the "bar grows from a fixed zero edge" reading. */
function roundedRightRectPath(width: number, height: number, rx: number, ry: number): string {
  if (width <= 0) return ''
  const rxc = Math.max(0, Math.min(rx, width))
  const ryc = Math.max(0, Math.min(ry, height / 2))
  if (rxc <= 0 || ryc <= 0) {
    return `M0,0 H${width} V${height} H0 Z`
  }
  return [
    'M0,0',
    `H${width - rxc}`,
    `A${rxc},${ryc} 0 0 1 ${width},${ryc}`,
    `V${height - ryc}`,
    `A${rxc},${ryc} 0 0 1 ${width - rxc},${height}`,
    'H0',
    'Z',
  ].join(' ')
}

const BAR_HEIGHT = 12

export interface BarDatum {
  key: string
  label: string
  value: number
}

/** Single-hue horizontal bar list — used for both the thread and event
 * "busiest" tables, since both are the same measure (self time) sliced
 * along a different dimension and should read as one visual language. */
export function HorizontalBarChart({
  items,
  valueFormatter = formatMs,
}: {
  items: BarDatum[]
  valueFormatter?: (value: number) => string
}) {
  const max = niceMax(Math.max(0, ...items.map((d) => d.value)))

  return (
    <div className="flex flex-col gap-3">
      {items.map((item) => {
        const widthPct = max > 0 ? Math.max(0, Math.min(100, (item.value / max) * 100)) : 0
        const path = roundedRightRectPath(widthPct, BAR_HEIGHT, 1.2, 4)
        return (
          <div key={item.key} className="min-w-0">
            <div
              className="mb-1 truncate text-[11px] text-zinc-500 dark:text-zinc-400"
              title={item.label}
            >
              {item.label}
            </div>
            <div className="flex items-center gap-2">
              <div className="min-w-0 flex-1 border-l border-[var(--perf-baseline)]">
                <svg
                  viewBox={`0 0 100 ${BAR_HEIGHT}`}
                  preserveAspectRatio="none"
                  width="100%"
                  height={BAR_HEIGHT}
                  className="block"
                >
                  {path ? <path d={path} fill="var(--perf-hue)" /> : null}
                </svg>
              </div>
              <div className="w-16 shrink-0 text-right text-[11px] tabular-nums text-zinc-800 dark:text-zinc-100">
                {valueFormatter(item.value)}
              </div>
            </div>
          </div>
        )
      })}
    </div>
  )
}

export interface TimelineSeriesInput {
  key: string
  label: string
  values: number[]
}

const CHART_WIDTH = 600
const CHART_HEIGHT = 170
const PADDING = { top: 8, right: 12, bottom: 8, left: 40 }
const GRIDLINE_COUNT = 4
const SERIES_COLOR_SLOTS = 6

function seriesColor(index: number): string {
  return `var(--perf-series-${(index % SERIES_COLOR_SLOTS) + 1})`
}

/** Multi-series line chart over the trace timeline buckets. Hover shows a
 * crosshair, nearest-bucket markers, and a tooltip; legend is always shown
 * above the plot since colors are otherwise unlabeled. */
export function TimelineChart({
  series,
  bucketMs,
}: {
  series: TimelineSeriesInput[]
  bucketMs: number
}) {
  const svgRef = useRef<SVGSVGElement>(null)
  const [hoverIndex, setHoverIndex] = useState<number | null>(null)

  const bucketCount = Math.max(1, ...series.map((s) => s.values.length))
  const maxValue = niceMax(Math.max(0, ...series.flatMap((s) => s.values)))

  const plotW = CHART_WIDTH - PADDING.left - PADDING.right
  const plotH = CHART_HEIGHT - PADDING.top - PADDING.bottom
  const xScale = scaleLinear([0, Math.max(1, bucketCount - 1)], [0, plotW])
  const yScale = scaleLinear([0, maxValue], [plotH, 0])

  const handleMove = (e: PointerEvent<SVGSVGElement>) => {
    const rect = svgRef.current?.getBoundingClientRect()
    if (!rect || rect.width === 0) return
    const relX = (e.clientX - rect.left) / rect.width
    const vbX = relX * CHART_WIDTH - PADDING.left
    const bucketFloat = (vbX / plotW) * (bucketCount - 1)
    setHoverIndex(Math.round(Math.max(0, Math.min(bucketCount - 1, bucketFloat))))
  }

  const gridlineValues = Array.from(
    { length: GRIDLINE_COUNT },
    (_, i) => (maxValue * (i + 1)) / GRIDLINE_COUNT,
  )

  const hoverX = hoverIndex !== null ? PADDING.left + xScale(hoverIndex) : null
  const hoverLeftPct = hoverX !== null ? (hoverX / CHART_WIDTH) * 100 : null

  return (
    <div>
      <div className="mb-2 flex flex-wrap gap-x-4 gap-y-1">
        {series.map((s, i) => (
          <div key={s.key} className="flex items-center gap-1.5">
            <span
              className="inline-block h-2.5 w-2.5 shrink-0 rounded-[2px]"
              style={{ background: seriesColor(i) }}
            />
            <span className="truncate text-[11px] text-zinc-500 dark:text-zinc-400">
              {s.label}
            </span>
          </div>
        ))}
      </div>

      <div className="relative">
        {/* Y-axis labels live in HTML, not the SVG: the plot stretches
           horizontally (preserveAspectRatio="none"), which would distort
           glyphs. Height is fixed, so px tops are exact. */}
        <YAxisLabel top={PADDING.top - 7}>{formatMs(maxValue)}</YAxisLabel>
        <YAxisLabel top={PADDING.top + plotH - 13}>0</YAxisLabel>
        <svg
          ref={svgRef}
          viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
          preserveAspectRatio="none"
          width="100%"
          height={CHART_HEIGHT}
          className="block"
          onPointerMove={handleMove}
          onPointerLeave={() => setHoverIndex(null)}
        >
          {gridlineValues.map((v, i) => (
            <line
              key={`grid-${i}`}
              x1={PADDING.left}
              x2={CHART_WIDTH - PADDING.right}
              y1={PADDING.top + yScale(v)}
              y2={PADDING.top + yScale(v)}
              stroke="var(--perf-gridline)"
              strokeWidth={1}
              vectorEffect="non-scaling-stroke"
            />
          ))}

          <line
            x1={PADDING.left}
            x2={CHART_WIDTH - PADDING.right}
            y1={PADDING.top + plotH}
            y2={PADDING.top + plotH}
            stroke="var(--perf-baseline)"
            strokeWidth={1}
            vectorEffect="non-scaling-stroke"
          />

          {series.map((s, i) => {
            const d = s.values
              .map((v, idx) => {
                const x = PADDING.left + xScale(idx)
                const y = PADDING.top + yScale(v)
                return `${idx === 0 ? 'M' : 'L'}${x},${y}`
              })
              .join(' ')
            return (
              <path
                key={s.key}
                d={d}
                fill="none"
                stroke={seriesColor(i)}
                strokeWidth={2}
                strokeLinejoin="round"
                vectorEffect="non-scaling-stroke"
              />
            )
          })}

          {hoverIndex !== null && hoverX !== null ? (
            <>
              <line
                x1={hoverX}
                x2={hoverX}
                y1={PADDING.top}
                y2={PADDING.top + plotH}
                stroke="var(--perf-baseline)"
                strokeWidth={1}
                vectorEffect="non-scaling-stroke"
              />
              {series.map((s, i) => {
                const v = s.values[hoverIndex]
                if (v === undefined) return null
                return (
                  <circle
                    key={s.key}
                    cx={hoverX}
                    cy={PADDING.top + yScale(v)}
                    r={2}
                    fill={seriesColor(i)}
                  />
                )
              })}
            </>
          ) : null}
        </svg>

        {hoverIndex !== null && hoverLeftPct !== null ? (
          <div
            className="pointer-events-none absolute top-1 z-10 -translate-x-1/2 rounded-md border border-[var(--surface-popover-border)] bg-[var(--surface-popover)] px-2 py-1.5 text-[11px] shadow-sm"
            style={{ left: `${Math.min(96, Math.max(4, hoverLeftPct))}%` }}
          >
            <div className="mb-1 tabular-nums text-zinc-500 dark:text-zinc-400">
              {formatMs(hoverIndex * bucketMs)}
            </div>
            <div className="flex flex-col gap-0.5">
              {series.map((s, i) => (
                <div key={s.key} className="flex items-center gap-1.5">
                  <span
                    className="inline-block h-2 w-2 shrink-0 rounded-[1px]"
                    style={{ background: seriesColor(i) }}
                  />
                  <span className="truncate text-zinc-600 dark:text-zinc-300">{s.label}</span>
                  <span className="ml-auto tabular-nums text-zinc-800 dark:text-zinc-100">
                    {formatMs(s.values[hoverIndex] ?? 0)}
                  </span>
                </div>
              ))}
            </div>
          </div>
        ) : null}
      </div>

      <div className="mt-1 flex justify-between text-[11px]" style={{ color: 'var(--perf-muted)' }}>
        <span>0s</span>
        <span>{(((bucketCount - 1) * bucketMs) / 1000).toFixed(1)}s</span>
      </div>
    </div>
  )
}

function YAxisLabel({ top, children }: { top: number; children: ReactNode }) {
  return (
    <div
      className="pointer-events-none absolute left-0 text-right text-[11px] tabular-nums"
      style={{
        color: 'var(--perf-muted)',
        width: `calc(${(PADDING.left / CHART_WIDTH) * 100}% - 6px)`,
        top,
      }}
    >
      {children}
    </div>
  )
}
