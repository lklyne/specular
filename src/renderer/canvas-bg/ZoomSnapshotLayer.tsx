import type {
  CanvasScenePageEntity,
  ZoomSnapshotState,
} from '../../shared/types'

export function ZoomSnapshotLayer({
  pages,
  snapshot,
}: {
  pages: CanvasScenePageEntity[]
  snapshot: ZoomSnapshotState
}) {
  if (snapshot.frames.length === 0) return null

  const frameByPageId = new Map(
    snapshot.frames.map((frame) => [frame.pageId, frame]),
  )

  return (
    <div
      className="pointer-events-none absolute inset-0"
      data-zoom-snapshot-active={snapshot.active ? 'true' : 'false'}
    >
      {pages.flatMap((page) => {
        const frame = frameByPageId.get(page.id)
        if (!frame) return []
        return (
          <img
            key={page.id}
            src={frame.dataUrl}
            alt=""
            draggable={false}
            data-zoom-snapshot-page-id={page.id}
            className="pointer-events-none absolute select-none"
            style={{
              left: page.contentScreenX ?? page.screenX,
              top: page.contentScreenY ?? page.screenY,
              width: page.contentScreenWidth ?? page.screenWidth,
              height: page.contentScreenHeight ?? page.screenHeight,
              borderRadius: 4,
            }}
          />
        )
      })}
    </div>
  )
}
