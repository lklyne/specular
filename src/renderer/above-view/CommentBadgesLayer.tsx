import type { ProjectedLayoutData, ProjectedPageEntity } from '../../shared/scene-projection'
import { memo, useEffect, useMemo, useRef, useState } from 'react'
import { CircleCheck, MessageSquare, Trash2 } from 'lucide-react'
import type { Annotation } from '../../shared/types'
import { isUnresolved } from '../../shared/annotation-utils'
import { pageViewportToScreen } from '../../shared/page-space'
import type { AnnotationLiveBboxLookup } from './annotationMath'
import { PageOverlayBand } from './PageOverlayBand'

interface CommentBadge {
  key: string
  annotationId: string
  /** Every unresolved annotation grouped under this badge — what its
   *  popover's Resolve and Delete act on. */
  annotationIds: string[]
  pageId: string
  count: number
  summary: string
  x: number
  y: number
  transform: string
  highlightRect?: { left: number; top: number; width: number; height: number }
}

export const CommentBadgesLayer = memo(function CommentBadgesLayer({
  annotations,
  layoutData,
  liveBboxes,
  onOpenThread,
  onResolve,
  onDelete,
}: {
  annotations: Annotation[]
  layoutData: ProjectedLayoutData
  liveBboxes: AnnotationLiveBboxLookup
  onOpenThread: (annotationId: string) => void
  onResolve: (annotationIds: string[]) => void
  onDelete: (annotationIds: string[]) => void
}) {
  const [hoveredKey, setHoveredKey] = useState<string | null>(null)
  // Leaving the badge closes the popover after a beat, so the pointer can
  // cross the gap into it; entering the popover cancels the close.
  const closeTimer = useRef<number | null>(null)
  const cancelClose = () => {
    if (closeTimer.current !== null) window.clearTimeout(closeTimer.current)
    closeTimer.current = null
  }
  const hover = (key: string) => {
    cancelClose()
    setHoveredKey(key)
  }
  const scheduleClose = () => {
    cancelClose()
    closeTimer.current = window.setTimeout(() => setHoveredKey(null), 200)
  }
  useEffect(() => cancelClose, [])
  const badges = useMemo(
    () => commentBadgesForLayout(annotations, layoutData, liveBboxes),
    [annotations, layoutData, liveBboxes],
  )
  const hoveredBadge = hoveredKey
    ? badges.find((badge) => badge.key === hoveredKey) ?? null
    : null

  if (badges.length === 0) return null

  const pagesById = new Map(
    layoutData.entities
      .filter((entity): entity is ProjectedPageEntity => entity.kind === 'page')
      .map((page) => [page.id, page]),
  )
  const byPage = new Map<string, CommentBadge[]>()
  for (const badge of badges) {
    const group = byPage.get(badge.pageId)
    if (group) group.push(badge)
    else byPage.set(badge.pageId, [badge])
  }

  return (
    <>
      {[...byPage.entries()].map(([pageId, group]) => {
        const page = pagesById.get(pageId)
        if (!page) return null
        return (
          <PageOverlayBand key={pageId} page={page} originY={layoutData.canvasOrigin.y} zIndex={15}>
            {hoveredBadge?.pageId === pageId && hoveredBadge.highlightRect ? (
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
            {group.map((badge) => (
              <button
                key={badge.key}
                type="button"
                data-overlay-ui="comment-badge"
                aria-label={`${badge.count} open messages`}
                className="pointer-events-auto absolute z-[15] inline-flex outline-none items-center gap-1.5 rounded-full border border-blue-300/90 bg-blue-500 px-2 py-1.5 text-[10px] font-semibold leading-none text-white shadow-[0_2px_6px_rgba(0,0,0,0.25)]"
                style={{
                  left: badge.x,
                  top: badge.y,
                  transform: badge.transform,
                }}
                onClick={(event) => {
                  event.preventDefault()
                  event.stopPropagation()
                  setHoveredKey(null)
                  onOpenThread(badge.annotationId)
                }}
                onPointerEnter={() => hover(badge.key)}
                onPointerLeave={() => scheduleClose()}
              >
                <MessageSquare size={12} strokeWidth={1.8} />
                <span>{badge.count}</span>
              </button>
            ))}
          </PageOverlayBand>
        )
      })}
      {hoveredBadge ? (
        <div
          data-overlay-ui="comment-badge-popover"
          className="pointer-events-auto absolute z-[45] w-[260px] whitespace-pre-wrap rounded-[14px] border border-zinc-400/80 bg-white px-2.5 py-2 text-[11px] leading-[1.4] text-[var(--surface-foreground)] shadow-[0_8px_16px_rgba(0,0,0,0.15)] dark:border-zinc-600 dark:bg-zinc-900"
          style={{
            left: Math.max(8, Math.min(hoveredBadge.x - 240, window.innerWidth - 268)),
            top: Math.max(8, Math.min(hoveredBadge.y + 22, window.innerHeight - 108)),
          }}
          onPointerEnter={() => hover(hoveredBadge.key)}
          onPointerLeave={() => scheduleClose()}
        >
          <div className="flex items-center gap-1">
            <div className="flex-1 font-semibold">
              {hoveredBadge.count} message{hoveredBadge.count === 1 ? '' : 's'}
            </div>
            <PopoverAction
              label="Resolve"
              onClick={() => {
                setHoveredKey(null)
                onResolve(hoveredBadge.annotationIds)
              }}
            >
              <CircleCheck size={13} strokeWidth={1.8} />
            </PopoverAction>
            <PopoverAction
              label="Delete"
              onClick={() => {
                setHoveredKey(null)
                onDelete(hoveredBadge.annotationIds)
              }}
            >
              <Trash2 size={13} strokeWidth={1.8} />
            </PopoverAction>
          </div>
          <button
            type="button"
            className="mt-1 block w-full cursor-pointer text-left"
            onClick={() => {
              setHoveredKey(null)
              onOpenThread(hoveredBadge.annotationId)
            }}
          >
            {hoveredBadge.summary}
          </button>
        </div>
      ) : null}
    </>
  )
})

function PopoverAction({
  label,
  onClick,
  children,
}: {
  label: string
  onClick: () => void
  children: React.ReactNode
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      className="flex h-6 w-6 items-center justify-center rounded-md text-[var(--surface-foreground-muted)] hover:bg-black/5 hover:text-[var(--surface-foreground)] dark:hover:bg-white/10"
      onClick={onClick}
    >
      {children}
    </button>
  )
}

export function commentBadgesForLayout(
  annotations: Annotation[],
  layoutData: ProjectedLayoutData,
  liveBboxes: AnnotationLiveBboxLookup,
): CommentBadge[] {
  const pagesById = new Map(
    layoutData.entities
      .filter((entity): entity is ProjectedPageEntity => entity.kind === 'page')
      .map((page) => [page.id, page]),
  )
  const grouped = new Map<string, { representative: Annotation; count: number; ids: string[] }>()

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
      existing.ids.push(annotation.id)
    } else {
      grouped.set(key, {
        representative: annotation,
        count: 1 + annotation.replies.length,
        ids: [annotation.id],
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
      badges.push({
        key,
        annotationId: annotation.id,
        annotationIds: value.ids,
        pageId: anchor.pageId,
        count: value.count,
        summary: annotation.text,
        x: rect.left + rect.width - 8,
        y: rect.top + 8,
        transform: 'translate(-100%, 0)',
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
      badges.push({
        key,
        annotationId: annotation.id,
        annotationIds: value.ids,
        pageId: anchor.pageId,
        count: value.count,
        summary: annotation.text,
        x: rightX,
        y: y - layoutData.canvasOrigin.y,
        transform: 'translate(-100%, -50%)',
      })
    }
  }
  return badges
}

function elementAnnotationRect(
  annotation: Annotation,
  page: ProjectedPageEntity,
  layoutData: ProjectedLayoutData,
  liveBboxes: AnnotationLiveBboxLookup,
): { left: number; top: number; width: number; height: number } | null {
  if (annotation.anchor.type !== 'element') return null
  const bbox = liveBboxes.get(annotation.id) ?? annotation.anchor.boundingBox
  if (!bbox) return null
  return pageViewportToScreen(bbox, page, layoutData)
}
