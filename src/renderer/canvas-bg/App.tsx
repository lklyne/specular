import { useRef, useState } from 'react'
import type { LayoutUpdateData, ThemeData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { useReportTextEditing } from '../shared/hooks/useReportTextEditing'
import { useTheme } from '../shared/hooks/useTheme'
import { DRAW_CURSOR } from './canvasBgConstants'
import { CanvasDebugBadge, CanvasGridSurface } from './CanvasGridSurface'
import { CanvasItemSurface } from './CanvasItemSurface'
import { GroupBackgroundLayer } from './GroupBackgroundLayer'
import { PerfHudOverlay } from './PerfHudOverlay'
import { readPageSurfaceArm } from './spike/pageSurfaceArm'
import { SpikePageSurface } from './spike/SpikePageSurface'
import { useCanvasLayoutState } from './useCanvasLayoutState'
import { useCanvasViewportGestures } from './useCanvasViewportGestures'
import { useChromeSlices } from './useChromeSlices'

const api = (window as unknown as { electronAPI: CanvasBgElectronAPI }).electronAPI

export default function App({
  initialLayoutData,
  initialTheme,
}: {
  initialLayoutData: LayoutUpdateData
  initialTheme: ThemeData
}) {
  const isDev =
    ((import.meta as unknown as { env?: { DEV?: boolean } }).env?.DEV ??
      false) === true
  const bgRef = useRef<HTMLDivElement>(null)
  // Read once at mount: the spike arm is fixed for a session's lifetime
  // (`src/shared/page-surface-spike.ts`).
  const [pageSurfaceArm] = useState(readPageSurfaceArm)
  const { isDark } = useTheme(initialTheme, api.onThemeChanged)
  useReportTextEditing(api.setTextEditing)
  const { layoutData, layoutRef, layoutTick } = useCanvasLayoutState({ initialLayoutData })

  useCanvasViewportGestures({
    api,
    bgRef,
    layoutRef,
  })

  const { canvasItemDraws, chromeGroups } = useChromeSlices(layoutData)
  return (
    <div
      className="relative h-screen w-screen overflow-hidden"
      style={{
        cursor: layoutData.activeTool.kind === 'draw' ? DRAW_CURSOR : undefined,
      }}
    >
      <CanvasDebugBadge
        annotationCount={layoutData.annotations.length}
        activeTool={layoutData.activeTool}
        isDev={isDev}
        layoutTick={layoutTick}
      />
      <PerfHudOverlay isDev={isDev} layoutData={layoutData} />
      <CanvasGridSurface
        bgRef={bgRef}
        isDark={isDark}
        canvasOrigin={layoutData.canvasOrigin}
        pan={layoutData.pan}
        zoom={layoutData.zoom}
      />
      {/* Every layer sits at the window origin and is placed by projection
          from the camera slice, so the scene container needs no transform. */}
      <div className="pointer-events-none absolute inset-0">
        <GroupBackgroundLayer groups={chromeGroups} isDark={isDark} />
      </div>

      {/* Pages and device-framed files, each painted whole (shell, border,
          live texture) in z-order on one canvas, topmost within canvas-bg.
          Drawn rather than DOM so strokes stay crisp mid-zoom (ADR 0038). */}
      <CanvasItemSurface api={api} draws={canvasItemDraws} isDark={isDark} />

      {/* WebGPU/three measurement spike (throwaway; ADR 0038's territory):
          draws page content over CanvasItemSurface's shells/borders when a
          non-2d arm is active. */}
      {pageSurfaceArm !== '2d' && <SpikePageSurface arm={pageSurfaceArm} draws={canvasItemDraws} />}

      {/* Group selection popup migrated to above-view (ADR 0008 §1, step 5).
          Selected page menu lives in the floating-ui view. */}
    </div>
  )
}
