import { useMemo, useState } from 'react'
import { MessageSquare } from 'lucide-react'
import type {
  Annotation,
  CanvasScenePageEntity,
  LayoutUpdateData,
} from '../../shared/types'
import { isUnresolved } from '../../shared/annotation-utils'
import type { AnnotationLiveBboxLookup } from './annotationMath'

interface CommentBadge {
  key: string
  annotationId: string
  count: number
  summary: string
  x: number
  y: number
  transform: string
  opacity: number
  highlightRect?: { left: number; top: number; width: number; height: number }
}

// How far (screen px) an element-anchored badge may travel outside its page's
// content band before it fully fades. Keeps badges from drifting onto the page
// chrome above the frame or floating far off-page after scroll.
const BADGE_FADE_MARGIN = 48

export function CommentBadgesLayer({
  annotations,
  layoutData,
  liveBboxes,
  onOpenThread,
}: {
  annotations: Annotation[]
  layoutData: LayoutUpdateData
  liveBboxes: AnnotationLiveBboxLookup
  onOpenThread: (annotationId: string) => void
}) {
  const [hoveredKey, setHoveredKey] = useState<string | null>(null)
  const badges = useMemo(
    () => commentBadgesForLayout(annotations, layoutData, liveBboxes),
    [annotations, layoutData, liveBboxes],
  )
  const hoveredBadge = hoveredKey
    ? badges.find((badge) => badge.key === hoveredKey) ?? null
    : null

  if (badges.length === 0) return null

  return (
    <>
      {hoveredBadge?.highlightRect ? (
        <div
          className="pointer-events-none absolute z-[14]"
          style={{
            left: hoveredBadge.highlightRect.left,
            top: hoveredBadge.highlightRect.top,
            width: Math.max(1, hoveredBadge.highlightRect.width),
            height: Math.max(1, hoveredBadge.highlightRect.height),
            border: '1px dashed rgba(59, 130, 246, 0.95)',
            background: 'rgba(59, 130, 246, 0.14)',
            boxShadow: '0 0 0 1px rgba(255, 255, 255, 0.22) inset',
            boxSizing: 'border-box',
          }}
        />
      ) : null}
      {badges.map((badge) => (
        <button
          key={badge.key}
          type="button"
          data-overlay-ui="comment-badge"
          aria-label={`${badge.count} unresolved messages`}
          className="pointer-events-auto absolute z-[15] inline-flex items-center gap-1.5 rounded-full border border-blue-300/90 bg-blue-500 px-2 py-1.5 text-[10px] font-semibold leading-none text-white shadow-[0_2px_6px_rgba(0,0,0,0.25)]"
          style={{
            left: badge.x,
            top: badge.y,
            transform: badge.transform,
            opacity: badge.opacity,
            pointerEvents: badge.opacity < 0.4 ? 'none' : undefined,
          }}
          onClick={(event) => {
            event.preventDefault()
            event.stopPropagation()
            setHoveredKey(null)
            onOpenThread(badge.annotationId)
          }}
          onPointerEnter={() => setHoveredKey(badge.key)}
          onPointerLeave={() => setHoveredKey((current) => (current === badge.key ? null : current))}
        >
          <MessageSquare size={12} strokeWidth={1.8} />
          <span>{badge.count}</span>
        </button>
      ))}
      {hoveredBadge ? (
        <div
          className="pointer-events-none absolute z-[19] w-[260px] whitespace-pre-wrap rounded-[14px] border border-zinc-400/80 bg-white px-2.5 py-2 text-[11px] leading-[1.4] text-zinc-900 shadow-[0_8px_16px_rgba(0,0,0,0.15)] dark:border-zinc-600 dark:bg-zinc-900 dark:text-zinc-100"
          style={{
            left: Math.max(8, Math.min(hoveredBadge.x - 240, window.innerWidth - 268)),
            top: Math.max(8, Math.min(hoveredBadge.y + 22, window.innerHeight - 108)),
          }}
        >
          <div className="font-semibold">
            {hoveredBadge.count} message{hoveredBadge.count === 1 ? '' : 's'}
          </div>
          <div className="mt-1">{hoveredBadge.summary}</div>
        </div>
      ) : null}
    </>
  )
}

export function commentBadgesForLayout(
  annotations: Annotation[],
  layoutData: LayoutUpdateData,
  liveBboxes: AnnotationLiveBboxLookup,
): CommentBadge[] {
  const pagesById = new Map(
    layoutData.entities
      .filter((entity): entity is CanvasScenePageEntity => entity.kind === 'page')
      .map((page) => [page.id, page]),
  )
  const grouped = new Map<string, { representative: Annotation; count: number }>()

  for (const annotation of annotations
    .filter((candidate) => isUnresolved(candidate.status))
    .sort((a, b) => Date.parse(b.createdAt) - Date.parse(a.createdAt))) {
    const anchor = annotation.anchor
    if (anchor.type !== 'element' && anchor.type !== 'page') continue
    const key =
      anchor.type === 'page'
        ? `page:${anchor.pageId}:${anchor.offsetX}:${anchor.offsetY}`
        : `element:${anchor.pageId}:${anchor.elementPath ?? anchor.selector}:${anchor.boundingBox?.x ?? ''}:${anchor.boundingBox?.y ?? ''}:${anchor.boundingBox?.width ?? ''}:${anchor.boundingBox?.height ?? ''}`
    const existing = grouped.get(key)
    if (existing) {
      existing.count += 1 + annotation.replies.length
    } else {
      grouped.set(key, {
        representative: annotation,
        count: 1 + annotation.replies.length,
      })
    }
  }

  const badges: CommentBadge[] = []
  for (const [key, value] of grouped) {
    const annotation = value.representative
    const anchor = annotation.anchor
    if (anchor.type === 'element') {
      const page = pagesById.get(anchor.pageId)
      const rect = page ? elementAnnotationRect(annotation, page, layoutData, liveBboxes) : null
      if (!rect) continue
      const opacity = page ? badgeOpacity(rect.top + 8, page, layoutData) : 1
      if (opacity <= 0) continue
      badges.push({
        key,
        annotationId: annotation.id,
        count: value.count,
        summary: annotation.text,
        x: rect.left + rect.width - 8,
        y: rect.top + 8,
        transform: 'translate(-100%, 0)',
        opacity,
        highlightRect: rect,
      })
      continue
    }
    if (anchor.type === 'page') {
      const page = pagesById.get(anchor.pageId)
      if (!page) continue
      const rightX = page.screenX + page.screenWidth - 8
      const y = Math.min(
        Math.max(page.screenY + anchor.offsetY * page.screenHeight, page.screenY + 10),
        page.screenY + page.screenHeight - 10,
      )
      const top = y - layoutData.canvasOrigin.y
      const opacity = badgeOpacity(top, page, layoutData)
      if (opacity <= 0) continue
      badges.push({
        key,
        annotationId: annotation.id,
        count: value.count,
        summary: annotation.text,
        x: rightX,
        y: top,
        transform: 'translate(-100%, -50%)',
        opacity,
      })
    }
  }
  return badges
}

// Fade a badge as its anchor point leaves the page's vertical content band.
// Keyed on the badge's own y (the element's top edge / page offset), not the
// element's full height — a tall element (e.g. #main) straddles the band even
// when its anchored top has scrolled far out of view. Returns 1 inside the
// band, ramps to 0 across BADGE_FADE_MARGIN of overflow, 0 beyond.
function badgeOpacity(
  anchorY: number,
  page: CanvasScenePageEntity,
  layoutData: LayoutUpdateData,
): number {
  const contentScreenY = page.contentScreenY ?? page.screenY
  const contentScreenHeight = page.contentScreenHeight ?? page.screenHeight
  const top = contentScreenY - layoutData.canvasOrigin.y
  const bottom = top + contentScreenHeight
  const overflow = Math.max(top - anchorY, anchorY - bottom, 0)
  if (overflow <= 0) return 1
  return Math.max(0, 1 - overflow / BADGE_FADE_MARGIN)
}

function elementAnnotationRect(
  annotation: Annotation,
  page: CanvasScenePageEntity,
  layoutData: LayoutUpdateData,
  liveBboxes: AnnotationLiveBboxLookup,
): { left: number; top: number; width: number; height: number } | null {
  if (annotation.anchor.type !== 'element') return null
  const bbox = liveBboxes.get(annotation.id) ?? annotation.anchor.boundingBox
  if (!bbox) return null

  const contentScreenX = page.contentScreenX ?? page.screenX
  const contentScreenY = page.contentScreenY ?? page.screenY
  const contentScreenWidth = page.contentScreenWidth ?? page.screenWidth
  const contentScreenHeight = page.contentScreenHeight ?? page.screenHeight
  const scaleX = contentScreenWidth / page.width
  const scaleY = contentScreenHeight / page.height

  return {
    left: contentScreenX + bbox.x * scaleX,
    top: contentScreenY + bbox.y * scaleY - layoutData.canvasOrigin.y,
    width: bbox.width * scaleX,
    height: bbox.height * scaleY,
  }
}
