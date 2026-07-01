import { useEffect, useState } from 'react'
import type {
  CanvasBgElectronAPI,
  CanvasScenePageEntity,
  LayoutUpdateData,
} from '../../shared/types'
import { agentOverlayClipPath } from '../../shared/agent-overlay-clip'
import { AgentCursorLayer } from '../canvas-bg/AgentCursorLayer'
import { useSceneCamera, sceneReprojectTransform } from '../shared/hooks/useSceneCamera'
import { InspectPopoverLayer } from './InspectPopoverLayer'

const api = (window as unknown as { electronAPI: CanvasBgElectronAPI }).electronAPI

export default function App({
  initialLayoutData,
}: {
  initialLayoutData: LayoutUpdateData
}) {
  const [layoutData, setLayoutData] = useState<LayoutUpdateData>(initialLayoutData)

  useEffect(() => api.onLayoutUpdate(setLayoutData), [])

  // Pan/zoom no longer rebuild the scene (ADR 0023 Phase 1), so the presence
  // cursors would freeze during a gesture. Reproject them live from the nudge,
  // same as the canvas-bg / above-view scenes. Cursor children subtract
  // canvasOrigin.y, so the origin's local y is 0.
  const camera = useSceneCamera(api.onViewportNudge, layoutData)
  const sceneTransform = sceneReprojectTransform(layoutData, camera, 0)

  return (
    <div
      className="pointer-events-none fixed inset-0 overflow-hidden bg-transparent"
      style={{ clipPath: agentOverlayClipPath(layoutData) }}
    >
      <div className="absolute inset-0 origin-top-left" style={{ transform: sceneTransform }}>
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
      </div>
      <InspectPopoverLayer layoutData={layoutData} />
    </div>
  )
}
