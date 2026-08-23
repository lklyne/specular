import { useMemo, useRef } from 'react'
import type { LayoutUpdateData, ThemeData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { useReportTextEditing } from '../shared/hooks/useReportTextEditing'
import { useTheme } from '../shared/hooks/useTheme'
import { DRAW_CURSOR } from './canvasBgConstants'
import { CanvasDebugBadge, CanvasGridSurface } from './CanvasGridSurface'
import { ChromeCanvasSurface } from './ChromeCanvasSurface'
import { GroupBackgroundLayer } from './GroupBackgroundLayer'
import { PerfHudOverlay } from './PerfHudOverlay'
import { SvgDeviceShellLayer } from './SvgDeviceShellLayer'
import { useCanvasLayoutState } from './useCanvasLayoutState'
import { useCanvasViewportGestures } from './useCanvasViewportGestures'
import { useSceneCameraTransform } from '../shared/hooks/useScenePanOffset'
import { cameraAfterSceneTransform } from '../../shared/scene-camera-transform'
import { useFrozenPageBitmaps } from '../shared/useFrozenPageBitmaps'
import { useFrozenPagesState } from './useFrozenPagesState'
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
  const { isDark } = useTheme(initialTheme, api.onThemeChanged)
  useReportTextEditing(api.setTextEditing)
  const { layoutData, layoutRef, layoutTick } = useCanvasLayoutState({ initialLayoutData })
  const { frozenPages, dragFrozenPageIds } = useFrozenPagesState(api)
  const frozenPageBitmaps = useFrozenPageBitmaps(frozenPages, (revision) =>
    api.frozenPagesReady('bg', revision),
  )
  const t = useSceneCameraTransform(
    api.onViewportNudge,
    layoutData,
    layoutData.canvasOrigin,
  )
  const liveCamera = useMemo(
    () => cameraAfterSceneTransform(layoutData, t, layoutData.canvasOrigin),
    [layoutData, t],
  )

  useCanvasViewportGestures({
    api,
    bgRef,
    layoutRef,
  })

  const { chromePages, chromeFiles, svgDeviceShellPages, chromeGroups } = useChromeSlices(
    layoutData,
    dragFrozenPageIds,
  )
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
        pan={liveCamera.pan}
        zoom={liveCamera.zoom}
      />
      {/* Translate+scale the page chrome live with the pan/zoom gesture so
          borders and device shells track the natively-positioned page views
          instead of trailing until the next layout-update rebuild lands
          (#257). */}
      <div
        className="pointer-events-none absolute inset-0"
        style={{ transform: `translate3d(${t.x}px, ${t.y}px, 0) scale(${t.scale})`, transformOrigin: '0 0' }}
      >
        <GroupBackgroundLayer groups={chromeGroups} isDark={isDark} />
        <div className="pointer-events-none absolute inset-0">
          <SvgDeviceShellLayer
            pages={svgDeviceShellPages}
            isDark={isDark}
          />
        </div>
      </div>
      {/* Borders, device shells, and frozen-page rasters draw in screen space
          from the live camera, outside the scene transform, so strokes stay
          crisp mid-zoom and the raster shares the chrome's exact geometry. */}
      <ChromeCanvasSurface
        pages={chromePages}
        fileEntities={chromeFiles}
        snapshots={frozenPageBitmaps}
        transform={t}
        isDark={isDark}
        dragFrozenPageIds={dragFrozenPageIds}
      />

      {/* Group selection popup migrated to above-view (ADR 0008 §1, step 5).
          Selected page menu lives in the floating-ui view. */}
    </div>
  )
}
