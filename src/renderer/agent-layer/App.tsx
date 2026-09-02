import type { ProjectedPageEntity } from '../../shared/scene-projection'
import { agentOverlayClipPath } from '../../shared/agent-overlay-clip'
import { useProjectedLayoutData } from '../shared/hooks/useProjectedLayoutData'
import { AgentCursorLayer } from '../canvas-bg/AgentCursorLayer'
import { InspectPopoverLayer } from './InspectPopoverLayer'

export default function App() {
  const layoutData = useProjectedLayoutData()

  return (
    <div
      className="pointer-events-none fixed inset-0 overflow-hidden bg-transparent"
      style={{ clipPath: agentOverlayClipPath(layoutData) }}
    >
      <AgentCursorLayer
        cursors={layoutData.presenceCursors}
        pages={layoutData.entities.filter(
          (entity): entity is ProjectedPageEntity => entity.kind === 'page',
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
