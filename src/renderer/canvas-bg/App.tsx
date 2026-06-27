import { useMemo, useRef } from 'react'
import type {
  CanvasBgElectronAPI,
  CanvasSceneFileEntity,
  CanvasScenePageEntity,
  LayoutUpdateData,
  ThemeData,
} from '../../shared/types'
import { useReportTextEditing } from '../shared/hooks/useReportTextEditing'
import { useTheme } from '../shared/hooks/useTheme'
import { DRAW_CURSOR } from './canvasBgConstants'
import { CanvasDebugBadge, CanvasGridSurface } from './CanvasGridSurface'
import { DeviceShellLayer } from './DeviceShellLayer'
import { GroupBackgroundLayer } from './GroupBackgroundLayer'
import { PageBorderLayer } from './PageBorderLayer'
import { SvgDeviceShellLayer } from './SvgDeviceShellLayer'
import { useCanvasLayoutState } from './useCanvasLayoutState'
import { useCanvasViewportGestures } from './useCanvasViewportGestures'

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
      <CanvasGridSurface
        bgRef={bgRef}
        isDark={isDark}
        canvasOrigin={layoutData.canvasOrigin}
        pan={layoutData.pan}
        zoom={layoutData.zoom}
      />
      <GroupBackgroundLayer
        groups={layoutData.groups ?? []}
        isDark={isDark}
        dimmed={layoutData.focusPresentation !== null}
      />
      <div className="pointer-events-none absolute inset-0">
        <PageBorderLayer
          pages={pageEntities}
          fileEntities={fileEntities}
        />
        <DeviceShellLayer
          pages={pageEntities.filter((f) => !f.useSvgDeviceShell)}
          fileEntities={fileEntities}
          isDark={isDark}
        />
        <SvgDeviceShellLayer
          pages={pageEntities.filter((f) => f.useSvgDeviceShell)}
          isDark={isDark}
        />
      </div>

      {/* Group selection popup migrated to above-view (ADR 0008 §1, step 5).
          Selected page menu lives in the floating-ui view. */}
    </div>
  )
}
