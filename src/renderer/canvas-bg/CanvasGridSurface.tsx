import { useCallback, useEffect, useMemo, useRef } from 'react'
import { PLAIN_TEXT_PLACEHOLDER } from '../../shared/constants'
import type { TextEntityStyle } from '../../shared/types'
import { resolveCanvasColor } from '../../shared/canvas-colors'
import { shapeDef, shapeRender } from '../../shared/shapes'
import { buildCanvasGridStyle, drawCanvasGrid } from './canvasGridStyle'

function previewBoxStyle(
  isDark: boolean,
  preview: { left: number; top: number; width: number; height: number },
) {
  return {
    left: preview.left,
    top: preview.top,
    width: preview.width,
    height: preview.height,
    borderColor: isDark ? 'rgba(214, 211, 209, 0.22)' : 'rgba(87, 83, 78, 0.28)',
    background: isDark ? 'rgba(41, 37, 36, 0.78)' : 'rgba(214, 211, 209, 0.92)',
    boxShadow: isDark
      ? '0 1px 2px rgba(0, 0, 0, 0.18)'
      : '0 1px 2px rgba(87, 83, 78, 0.08)',
  }
}

export function CanvasDebugBadge({
  annotationCount,
  activeTool,
  isDev,
  layoutTick,
}: {
  annotationCount: number
  activeTool: import('../../shared/types').Tool
  isDev: boolean
  layoutTick: number
}) {
  if (!isDev) return null
  return (
    <div
      className="pointer-events-auto absolute left-2 top-2 z-[70] rounded border border-zinc-300/80 bg-white/90 px-2 py-1 text-[10px] text-zinc-700 shadow dark:border-zinc-600 dark:bg-zinc-900/90 dark:text-zinc-200"
      data-overlay-ui
    >
      tool:{activeTool.kind} updates:{layoutTick} pending: 0 anns:{annotationCount} hits:0
    </div>
  )
}

export function CanvasGridSurface({
  bgRef,
  isDark,
  canvasOrigin,
  pan,
  zoom,
}: {
  bgRef: React.RefObject<HTMLDivElement | null>
  isDark: boolean
  canvasOrigin: { x: number; y: number }
  pan: { x: number; y: number }
  zoom: number
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const gridStyle = useMemo(() => buildCanvasGridStyle(), [])

  // Latest draw inputs, read by the resize-triggered redraw without re-binding
  // the observer every pan tick (#265).
  const drawInputs = useRef({ canvasOrigin, pan, zoom, isDark })
  drawInputs.current = { canvasOrigin, pan, zoom, isDark }

  const draw = useCallback(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const { canvasOrigin: o, pan: p, zoom: z, isDark: dark } = drawInputs.current
    drawCanvasGrid({
      canvas,
      color: dark ? '#78716c' : '#a8a29e',
      canvasOrigin: o,
      pan: p,
      zoom: z,
      isDark: dark,
      devicePixelRatio: window.devicePixelRatio || 1,
    })
  }, [])

  // Redraw when the viewport changes.
  useEffect(() => {
    draw()
  }, [canvasOrigin, isDark, pan, zoom, draw])

  // Bind resize listeners once — not on every pan.
  useEffect(() => {
    const surface = bgRef.current
    if (!surface) return
    const resizeObserver = new ResizeObserver(draw)
    resizeObserver.observe(surface)
    window.addEventListener('resize', draw)
    return () => {
      resizeObserver.disconnect()
      window.removeEventListener('resize', draw)
    }
  }, [bgRef, draw])

  return (
    <div
      ref={bgRef}
      tabIndex={0}
      className="absolute inset-0 outline-none"
      style={{
        touchAction: 'none',
        ...gridStyle,
      }}
    >
      <canvas
        ref={canvasRef}
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 h-full w-full"
      />
    </div>
  )
}

export function PlacementPreviewLayer({
  isDark,
  preview,
}: {
  isDark: boolean
  preview: {
    entityKind?: string
    shapeKind?: string
    textStyle?: TextEntityStyle
    color?: string
    textSize?: number
    zoom?: number
    left: number
    top: number
    width: number
    height: number
  } | null
}) {
  if (!preview) return null
  const isTextEntity = preview.entityKind === 'text'
  const isFileEntity = preview.entityKind === 'file'
  const isShape = preview.entityKind === 'shape'
  const isStickyPreview = isTextEntity && preview.textStyle === 'sticky'
  if (isTextEntity && preview.textStyle === 'plain') {
    const scaledFontSize = (preview.textSize ?? 14) * (preview.zoom ?? 1)
    const scaledLineHeight = scaledFontSize * 1.5
    return (
      <div
        className="pointer-events-none absolute select-none font-normal"
        style={{
          left: preview.left,
          top: preview.top,
          fontSize: scaledFontSize,
          lineHeight: `${scaledLineHeight}px`,
          color: isDark ? 'rgba(231, 229, 228, 0.58)' : 'rgba(28, 25, 23, 0.48)',
          fontFamily: 'system-ui, sans-serif',
        }}
      >
        {PLAIN_TEXT_PLACEHOLDER}
      </div>
    )
  }
  if (isShape) {
    const baseStyle = previewBoxStyle(isDark, preview)
    const stroke = isDark ? 'rgba(168, 162, 158, 0.6)' : 'rgba(120, 113, 108, 0.6)'
    const def = shapeDef(preview.shapeKind ?? 'rectangle')
    // The preview box is in screen px; `shapeRender` works in canvas units, so
    // un-zoom the size before building the path and let the svg scale it back.
    const zoom = preview.zoom ?? 1
    const render = shapeRender(def, baseStyle.width / zoom, baseStyle.height / zoom)
    return (
      <svg
        className="pointer-events-none absolute"
        width={baseStyle.width}
        height={baseStyle.height}
        viewBox={render.viewBox}
        preserveAspectRatio="none"
        style={{ left: baseStyle.left, top: baseStyle.top, overflow: 'visible' }}
      >
        <path
          d={render.d}
          fill="transparent"
          stroke={stroke}
          strokeWidth={1}
          strokeLinejoin="round"
          vectorEffect="non-scaling-stroke"
        />
        {render.line ? (
          <path d={render.line} fill="none" stroke={stroke} strokeWidth={1} vectorEffect="non-scaling-stroke" />
        ) : null}
      </svg>
    )
  }
  return (
    <div
      className="pointer-events-none absolute border"
      style={{
        ...previewBoxStyle(isDark, preview),
        ...(isStickyPreview && preview.color
          ? {
              background: resolveCanvasColor(preview.color, {
                role: 'fill',
                isDark,
                palette: 'soft',
              }),
              borderColor: 'rgba(87, 83, 78, 0.18)',
            }
          : isTextEntity
            ? {
                background: 'rgba(254, 240, 138, 0.7)',
                borderColor: 'rgba(202, 138, 4, 0.4)',
              }
            : isFileEntity
              ? {
                  background: isDark ? 'rgba(214, 211, 209, 0.15)' : 'rgba(250, 250, 249, 0.7)',
                  borderColor: isDark ? 'rgba(168, 162, 158, 0.4)' : 'rgba(120, 113, 108, 0.4)',
                  borderRadius: 4,
                }
              : {}),
      }}
    />
  )
}

function DragCopyPreviewLayer({
  dragCopyPreview,
  isDark,
}: {
  dragCopyPreview: Array<{ id: string; left: number; top: number; width: number; height: number }>
  isDark: boolean
}) {
  return (
    <>
      {dragCopyPreview.map((preview) => (
        <div
          key={`drag-copy-preview-${preview.id}`}
          className="pointer-events-none absolute border"
          style={previewBoxStyle(isDark, preview)}
        />
      ))}
    </>
  )
}
