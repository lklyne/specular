import { useEffect, useState } from 'react'
import type { CanvasScenePageEntity, LayoutUpdateData } from '../../shared/types'
import { agentOverlayClipPath } from '../../shared/agent-overlay-clip'
import { runtimeStore } from '../shared/runtime-store'
import { AgentCursorLayer } from '../canvas-bg/AgentCursorLayer'
import { InspectPopoverLayer } from './InspectPopoverLayer'

export default function App({
  initialLayoutData,
}: {
  initialLayoutData: LayoutUpdateData
}) {
  const [layoutData, setLayoutData] = useState<LayoutUpdateData>(initialLayoutData)

  useEffect(() => {
    const unsubscribe = runtimeStore.subscribe(() => setLayoutData(runtimeStore.readLayoutData()))
    return () => {
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
