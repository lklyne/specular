import { useEffect, useState } from 'react'
import type { CanvasScenePageEntity, LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { agentOverlayClipPath } from '../../shared/agent-overlay-clip'
import { runtimeStore } from '../shared/runtime-store'
import { AgentCursorLayer } from '../canvas-bg/AgentCursorLayer'
import { InspectPopoverLayer } from './InspectPopoverLayer'

const api = (window as unknown as { electronAPI: CanvasBgElectronAPI }).electronAPI

export default function App({
  initialLayoutData,
}: {
  initialLayoutData: LayoutUpdateData
}) {
  const [layoutData, setLayoutData] = useState<LayoutUpdateData>(initialLayoutData)

  useEffect(() => {
    const offSnapshot = api.onLayoutUpdate((data) => runtimeStore.applySnapshot(data))
    const offPatches = api.onRuntimePatch((batch) => runtimeStore.applyPatches(batch))
    const unsubscribe = runtimeStore.subscribe(() => setLayoutData(runtimeStore.readLayoutData()))
    return () => {
      offSnapshot()
      offPatches()
      unsubscribe()
    }
  }, [])

  return (
    <div
      className="pointer-events-none fixed inset-0 overflow-hidden bg-transparent"
      style={{ clipPath: agentOverlayClipPath(layoutData) }}
    >
      <AgentCursorLayer
        cursors={layoutData.presenceCursors}
        pages={layoutData.entities.filter(
          (entity): entity is CanvasScenePageEntity => entity.kind === 'page',
        )}
        canvasOrigin={layoutData.canvasOrigin}
        pan={layoutData.pan}
        zoom={layoutData.zoom}
        overlayOffsetY={layoutData.canvasOrigin.y}
      />
      <InspectPopoverLayer layoutData={layoutData} />
    </div>
  )
}
