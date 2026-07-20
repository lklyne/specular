import { useEffect, useMemo, useRef, useState } from 'react'
import type { CanvasSceneFileEntity, CanvasScenePageEntity, LayoutUpdateData, ThemeData, ZoomSnapshotState } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { focusContext } from '../../shared/focus-context'
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
import { ZoomSnapshotLayer } from './ZoomSnapshotLayer'

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
  const { layoutData, layoutRef, layoutTick } = useCanvasLayoutState({ api, initialLayoutData })
  const [zoomSnapshot, setZoomSnapshot] = useState<ZoomSnapshotState>({
    revision: 0,
    active: false,
    frames: [],
  })
  useEffect(() => api.onZoomSnapshotState(setZoomSnapshot), [])
  useEffect(() => {
    if (zoomSnapshot.frames.length === 0) return
    let cancelled = false
    const images = zoomSnapshot.frames.map((frame) => {
      const image = new Image()
      image.src = frame.dataUrl
      return image
    })
    void Promise.allSettled(
      images.map((image) =>
        typeof image.decode === 'function'
          ? image.decode()
          : Promise.resolve(),
      ),
    ).then(() => {
      if (!cancelled) api.zoomSnapshotReady(zoomSnapshot.revision)
    })
    return () => {
      cancelled = true
    }
  }, [zoomSnapshot.revision])
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

  const pageEntities = useMemo(
    () => layoutData.entities.filter((e): e is CanvasScenePageEntity => e.kind === 'page'),
    [layoutData.entities],
  )
  const fileEntities = useMemo(
    () => layoutData.entities.filter((e): e is CanvasSceneFileEntity => e.kind === 'file'),
    [layoutData.entities],
  )
  const focus = focusContext(layoutData)
  // 'fill' focus is the browser-like mode: edge-to-edge, no border, no bezel.
  const fillPageId = focus.mode === 'fill' ? focus.pageId : null
  // Eye off during focus: only the focused page's chrome survives; all other
  // context (other pages, file frames, groups) is hidden, never dimmed (ADR 0021).
  const hideContext = focus.active && !focus.showsContext
  const chromePages = useMemo(() => {
    if (fillPageId) return pageEntities.filter((p) => p.id !== fillPageId)
    if (hideContext) return pageEntities.filter((p) => p.id === focus.pageId)
    return pageEntities
  }, [pageEntities, fillPageId, hideContext, focus.pageId])
  const chromeFiles = useMemo(
    () => (hideContext ? [] : fileEntities),
    [fileEntities, hideContext],
  )
  // Hoist per-layer slices so the memoized layers receive stable array refs and
  // skip re-rendering on every pan/zoom nudge (props only change on a real
  // layout-update). Inline .filter() in JSX would defeat React.memo (#265).
  const svgDeviceShellPages = useMemo(
    () => chromePages.filter((f) => f.useSvgDeviceShell),
    [chromePages],
  )
  const chromeGroups = useMemo(
    () => (hideContext ? [] : (layoutData.groups ?? [])),
    [hideContext, layoutData.groups],
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
          <ZoomSnapshotLayer pages={pageEntities} snapshot={zoomSnapshot} />
        </div>
      </div>
      {/* Borders and device shells draw in screen space from the live camera,
          outside the scene transform, so their strokes stay crisp mid-zoom
          instead of being bitmap-scaled with the settled layout baseline. */}
      <ChromeCanvasSurface
        pages={chromePages}
        fileEntities={chromeFiles}
        transform={t}
        isDark={isDark}
      />

      {/* Group selection popup migrated to above-view (ADR 0008 §1, step 5).
          Selected page menu lives in the floating-ui view. */}
    </div>
  )
}
