import { useEffect, useState } from 'react'
import type {
  CanvasBgElectronAPI,
  CanvasScenePageEntity,
  LayoutUpdateData,
} from '../../shared/types'
import { agentOverlayClipPath } from '../../shared/agent-overlay-clip'
import { AgentCursorLayer } from '../canvas-bg/AgentCursorLayer'
import { InspectPopoverLayer } from './InspectPopoverLayer'

const api = (window as unknown as { electronAPI: CanvasBgElectronAPI }).electronAPI

export default function App({
  initialLayoutData,
}: {
  initialLayoutData: LayoutUpdateData
}) {
  const [layoutData, setLayoutData] = useState<LayoutUpdateData>(initialLayoutData)

  useEffect(() => api.onLayoutUpdate(setLayoutData), [])

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
