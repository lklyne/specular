import { useMemo, useRef } from 'react'
import type {
  CanvasBgElectronAPI,
  CanvasSceneFileEntity,
  CanvasScenePageEntity,
  LayoutUpdateData,
  ThemeData,
} from '../../shared/types'
import { focusContext } from '../../shared/focus-context'
import { useReportTextEditing } from '../shared/hooks/useReportTextEditing'
import { useTheme } from '../shared/hooks/useTheme'
import { DRAW_CURSOR } from './canvasBgConstants'
import { CanvasDebugBadge, CanvasGridSurface } from './CanvasGridSurface'
import { DeviceShellLayer } from './DeviceShellLayer'
import { GroupBackgroundLayer } from './GroupBackgroundLayer'
import { PageBorderLayer } from './PageBorderLayer'
import { PerfHudOverlay } from './PerfHudOverlay'
import { SvgDeviceShellLayer } from './SvgDeviceShellLayer'
import { useCanvasLayoutState } from './useCanvasLayoutState'
import { useCanvasViewportGestures } from './useCanvasViewportGestures'
import { useScenePanOffset } from '../shared/hooks/useScenePanOffset'

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
  const isDark = useTheme(initialTheme, api.onThemeChanged)
  useReportTextEditing(api.setTextEditing)
  const { layoutData, layoutRef, layoutTick } = useCanvasLayoutState({ api, initialLayoutData })
  const panOffset = useScenePanOffset(api.onViewportNudge, layoutData)
  const livePan = useMemo(
    () => ({ x: layoutData.pan.x + panOffset.x, y: layoutData.pan.y + panOffset.y }),
    [layoutData.pan.x, layoutData.pan.y, panOffset.x, panOffset.y],
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
        pan={livePan}
        zoom={layoutData.zoom}
      />
      {/* Translate the page chrome live with the pan gesture so borders and
          device shells track the natively-positioned page views instead of
          trailing until the next layout-update rebuild lands (#257). */}
      <div
        className="pointer-events-none absolute inset-0"
        style={{ transform: `translate3d(${panOffset.x}px, ${panOffset.y}px, 0)` }}
      >
        <GroupBackgroundLayer
          groups={hideContext ? [] : (layoutData.groups ?? [])}
          isDark={isDark}
        />
        <div className="pointer-events-none absolute inset-0">
          <PageBorderLayer
            pages={chromePages}
            fileEntities={chromeFiles}
          />
          <DeviceShellLayer
            pages={chromePages.filter((f) => !f.useSvgDeviceShell)}
            fileEntities={chromeFiles}
            isDark={isDark}
          />
          <SvgDeviceShellLayer
            pages={chromePages.filter((f) => f.useSvgDeviceShell)}
            isDark={isDark}
          />
        </div>
      </div>

      {/* Group selection popup migrated to above-view (ADR 0008 §1, step 5).
          Selected page menu lives in the floating-ui view. */}
    </div>
  )
}
