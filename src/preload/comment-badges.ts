import { ipcRenderer } from 'electron'
import type { Annotation } from '../shared/types'
import { isUnresolved } from '../shared/annotation-utils'

const COMMENT_BADGE_DEBUG = process.env.CANVAS_DEBUG_COMMENT_BADGES === '1'
const PAGE_COMMENT_BADGES_ENABLED = false

let pageAnnotations: Annotation[] = []
let commentBadgesLayerEl: HTMLDivElement | null = null
let commentHoverHighlightEl: HTMLDivElement | null = null
let commentHoverSummaryEl: HTMLDivElement | null = null
let commentHoverActive = false
let commentBadgesRenderRaf = 0
let commentHoverBadgeKey: string | null = null
let commentHoverHideRaf = 0
let commentBadgesRenderKey = ''
let commentBadgeDebugSeq = 0

interface PageCommentBadge {
  key: string
  annotationId: string
  count: number
  summary: string
  x: number
  y: number
  transform: string
  highlightRect?: { left: number; top: number; width: number; height: number }
}

function commentBadgeDebug(event: string, details?: Record<string, unknown>): void {
  if (!COMMENT_BADGE_DEBUG) return
  commentBadgeDebugSeq += 1
  console.log('[comment-badge-debug]', {
    seq: commentBadgeDebugSeq,
    ts: Date.now(),
    event,
    ...details,
  })
}

export function setPageAnnotations(annotations: Annotation[]): void {
  pageAnnotations = annotations
  commentBadgeDebug('page-annotations-update', {
    annotations: pageAnnotations.length,
    ids: pageAnnotations.map((annotation) => annotation.id),
  })
}

export function isCommentHoverActive(): boolean {
  return commentHoverActive
}

function ensureCommentBadgesOverlay(): void {
  if (commentBadgesLayerEl && commentHoverHighlightEl && commentHoverSummaryEl) return

  const layer = document.createElement('div')
  layer.id = '__canvas-comment-badges-layer'
  Object.assign(layer.style, {
    position: 'fixed',
    inset: '0',
    zIndex: '2147483647',
    pointerEvents: 'none',
  })

  const hoverHighlight = document.createElement('div')
  hoverHighlight.id = '__canvas-comment-hover-highlight'
  Object.assign(hoverHighlight.style, {
    position: 'fixed',
    zIndex: '2147483645',
    pointerEvents: 'none',
    border: '1px solid rgba(59, 130, 246, 0.95)',
    borderStyle: 'dashed',
    background: 'rgba(59, 130, 246, 0.14)',
    boxShadow: '0 0 0 1px rgba(255,255,255,0.22) inset',
    display: 'none',
  })

  const hoverSummary = document.createElement('div')
  hoverSummary.id = '__canvas-comment-hover-summary'
  Object.assign(hoverSummary.style, {
    position: 'fixed',
    zIndex: '2147483647',
    pointerEvents: 'none',
    width: '260px',
    borderRadius: '14px',
    border: '1px solid rgba(161, 161, 170, 0.8)',
    background: '#ffffff',
    color: '#111827',
    padding: '8px 10px',
    boxShadow: '0 8px 16px rgba(0, 0, 0, 0.15)',
    font: '11px/1.4 ui-sans-serif, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif',
    display: 'none',
    whiteSpace: 'pre-wrap',
  })

  document.documentElement.appendChild(layer)
  document.documentElement.appendChild(hoverHighlight)
  document.documentElement.appendChild(hoverSummary)
  commentBadgesLayerEl = layer
  commentHoverHighlightEl = hoverHighlight
  commentHoverSummaryEl = hoverSummary
}

export function hideCommentBadgeHover(): void {
  if (commentHoverHideRaf) {
    window.cancelAnimationFrame(commentHoverHideRaf)
    commentHoverHideRaf = 0
  }
  commentBadgeDebug('hide-hover', {
    prevHoverBadgeKey: commentHoverBadgeKey,
    activeBeforeHide: commentHoverActive,
  })
  commentHoverActive = false
  commentHoverBadgeKey = null
  if (commentHoverHighlightEl) commentHoverHighlightEl.style.display = 'none'
  if (commentHoverSummaryEl) commentHoverSummaryEl.style.display = 'none'
}

function scheduleCommentBadgeHoverHide(reason: string, event?: MouseEvent): void {
  if (commentHoverHideRaf) return
  const pointerX = event?.clientX ?? null
  const pointerY = event?.clientY ?? null
  commentBadgeDebug('schedule-hide', {
    reason,
    hoverBadgeKey: commentHoverBadgeKey,
    pointerX,
    pointerY,
  })
  commentHoverHideRaf = window.requestAnimationFrame(() => {
    commentHoverHideRaf = 0
    const stillHoveringBadge = Boolean(document.querySelector('[data-comment-badge]:hover'))
    let topElements: string[] = []
    if (pointerX !== null && pointerY !== null) {
      topElements = document
        .elementsFromPoint(pointerX, pointerY)
        .slice(0, 4)
        .map((el) => {
          const tag = el.tagName.toLowerCase()
          const id = (el as HTMLElement).id ? `#${(el as HTMLElement).id}` : ''
          const marker = (el as HTMLElement).getAttribute('data-comment-badge') ? '[data-comment-badge]' : ''
          return `${tag}${id}${marker}`
        })
    }
    commentBadgeDebug('run-hide-check', {
      reason,
      hoverBadgeKey: commentHoverBadgeKey,
      stillHoveringBadge,
      topElements,
    })
    if (stillHoveringBadge) return
    hideCommentBadgeHover()
  })
}

function updateCommentBadgeHover(badge: PageCommentBadge): void {
  if (commentHoverHideRaf) {
    window.cancelAnimationFrame(commentHoverHideRaf)
    commentHoverHideRaf = 0
  }
  commentHoverActive = true
  commentHoverBadgeKey = badge.key
  commentBadgeDebug('show-hover', {
    badgeKey: badge.key,
    annotationId: badge.annotationId,
    count: badge.count,
  })
  if (!commentHoverHighlightEl || !commentHoverSummaryEl) return
  if (badge.highlightRect) {
    commentHoverHighlightEl.style.display = 'block'
    commentHoverHighlightEl.style.left = `${Math.round(badge.highlightRect.left)}px`
    commentHoverHighlightEl.style.top = `${Math.round(badge.highlightRect.top)}px`
    commentHoverHighlightEl.style.width = `${Math.max(1, Math.round(badge.highlightRect.width))}px`
    commentHoverHighlightEl.style.height = `${Math.max(1, Math.round(badge.highlightRect.height))}px`
  } else {
    commentHoverHighlightEl.style.display = 'none'
  }

  commentHoverSummaryEl.style.display = 'block'
  const summaryX = Math.max(
    8,
    Math.min(badge.x - 240, window.innerWidth - 268),
  )
  const summaryY = Math.max(
    8,
    Math.min(badge.y + 22, window.innerHeight - 108),
  )
  commentHoverSummaryEl.style.left = `${Math.round(summaryX)}px`
  commentHoverSummaryEl.style.top = `${Math.round(summaryY)}px`
  commentHoverSummaryEl.replaceChildren()
  const title = document.createElement('div')
  title.style.fontWeight = '600'
  title.textContent = `${badge.count} message${badge.count === 1 ? '' : 's'}`
  const body = document.createElement('div')
  body.style.marginTop = '4px'
  body.textContent = badge.summary || ''
  commentHoverSummaryEl.appendChild(title)
  commentHoverSummaryEl.appendChild(body)
}

function annotationViewportRect(annotation: Annotation): { left: number; top: number; width: number; height: number } | null {
  const anchor = annotation.anchor
  if (anchor.type !== 'element' || !anchor.boundingBox) return null
  const inspectPosition = annotation.metadata?.inspectContext?.position
  if (inspectPosition?.documentY !== undefined) {
    return {
      left: anchor.boundingBox.x,
      top: inspectPosition.documentY - window.scrollY,
      width: anchor.boundingBox.width,
      height: anchor.boundingBox.height,
    }
  }
  return {
    left: anchor.boundingBox.x,
    top: anchor.boundingBox.y,
    width: anchor.boundingBox.width,
    height: anchor.boundingBox.height,
  }
}

function pageCommentBadgeData(): PageCommentBadge[] {
  // Resting comment badges render in aboveView so they can sit above focused
  // device frames and neighboring page WCVs. Keep this preload path quiet; the
  // page still owns comment hover previews and live-bbox tracking.
  const unresolved = pageAnnotations
    .filter((annotation) => isUnresolved(annotation.status))
    .filter(() => PAGE_COMMENT_BADGES_ENABLED)
    .sort((a, b) => Date.parse(b.createdAt) - Date.parse(a.createdAt))
  const grouped = new Map<
    string,
    {
      representative: Annotation
      count: number
    }
  >()

  const anchorKey = (annotation: Annotation): string => {
    const anchor = annotation.anchor
    if (anchor.type === 'canvas') return `canvas:${anchor.canvasX}:${anchor.canvasY}`
    if (anchor.type === 'page') return `page:${anchor.pageId}:${anchor.offsetX}:${anchor.offsetY}`
    if (anchor.type === 'region') return `region:${anchor.canvasRect.x}:${anchor.canvasRect.y}:${anchor.canvasRect.width}:${anchor.canvasRect.height}`
    const bb = anchor.boundingBox
    return `element:${anchor.pageId}:${anchor.elementPath ?? anchor.selector}:${bb?.x ?? ''}:${bb?.y ?? ''}:${bb?.width ?? ''}:${bb?.height ?? ''}`
  }

  for (const annotation of unresolved) {
    const key = anchorKey(annotation)
    const existing = grouped.get(key)
    if (!existing) {
      grouped.set(key, {
        representative: annotation,
        count: 1 + annotation.replies.length,
      })
      continue
    }
    existing.count += 1 + annotation.replies.length
  }

  const badges: PageCommentBadge[] = []
  for (const [key, value] of grouped.entries()) {
    const anchor = value.representative.anchor
    if (anchor.type === 'canvas') continue
    if (anchor.type === 'element' && anchor.boundingBox) {
      const rect = annotationViewportRect(value.representative)
      if (!rect) continue
      badges.push({
        key,
        annotationId: value.representative.id,
        count: value.count,
        summary: value.representative.text,
        x: rect.left + rect.width - 8,
        y: rect.top + 8,
        transform: 'translate(-100%, 0)',
        highlightRect: {
          left: rect.left,
          top: rect.top,
          width: rect.width,
          height: rect.height,
        },
      })
      continue
    }
    if (anchor.type === 'page') {
      badges.push({
        key,
        annotationId: value.representative.id,
        count: value.count,
        summary: value.representative.text,
        x: window.innerWidth - 8,
        y: Math.max(10, Math.min(window.innerHeight - 10, anchor.offsetY * window.innerHeight)),
        transform: 'translate(-100%, -50%)',
      })
    }
  }
  return badges
}

function commentBadgesKey(badges: PageCommentBadge[]): string {
  return badges
    .map((badge) =>
      [
        badge.key,
        badge.annotationId,
        badge.count,
        Math.round(badge.x),
        Math.round(badge.y),
        badge.transform,
        badge.summary,
        badge.highlightRect
          ? [
              Math.round(badge.highlightRect.left),
              Math.round(badge.highlightRect.top),
              Math.round(badge.highlightRect.width),
              Math.round(badge.highlightRect.height),
            ].join(',')
          : '',
      ].join('|'),
    )
    .join('||')
}

export function renderCommentBadges(): void {
  ensureCommentBadgesOverlay()
  if (!commentBadgesLayerEl) return
  const badges = pageCommentBadgeData()
  const nextRenderKey = commentBadgesKey(badges)
  const shouldRebuild = nextRenderKey !== commentBadgesRenderKey
  commentBadgesRenderKey = nextRenderKey
  commentBadgeDebug('render-start', {
    badges: badges.length,
    hoverBadgeKey: commentHoverBadgeKey,
    hoverActive: commentHoverActive,
    shouldRebuild,
  })
  let hoveredBadge: PageCommentBadge | null = null

  if (!shouldRebuild) {
    if (commentHoverBadgeKey) {
      hoveredBadge = badges.find((badge) => badge.key === commentHoverBadgeKey) ?? null
    }
    if (hoveredBadge) {
      commentBadgeDebug('render-preserve-hover', { badgeKey: hoveredBadge.key })
      updateCommentBadgeHover(hoveredBadge)
    } else if (commentHoverActive) {
      commentBadgeDebug('render-clear-hover-no-match', { hoverBadgeKey: commentHoverBadgeKey })
      hideCommentBadgeHover()
    }
    return
  }

  commentBadgesLayerEl.replaceChildren()

  for (const badge of badges) {
    if (commentHoverBadgeKey && badge.key === commentHoverBadgeKey) {
      hoveredBadge = badge
    }
    const button = document.createElement('button')
    button.type = 'button'
    button.setAttribute('data-overlay-ui', 'comment-badge')
    button.setAttribute('data-comment-badge', '1')
    button.setAttribute('aria-label', `${badge.count} unresolved messages`)
    Object.assign(button.style, {
      position: 'fixed',
      left: `${Math.round(badge.x)}px`,
      top: `${Math.round(badge.y)}px`,
      transform: badge.transform,
      pointerEvents: 'auto',
      display: 'inline-flex',
      alignItems: 'center',
      gap: '6px',
      borderRadius: '999px',
      border: '1px solid rgba(147, 197, 253, 0.9)',
      background: '#3b82f6',
      color: '#ffffff',
      font: '600 10px/1.1 ui-sans-serif, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif',
      padding: '6px 8px',
      boxShadow: '0 2px 6px rgba(0, 0, 0, 0.25)',
      cursor: 'pointer',
    })
    button.innerHTML =
      '<svg viewBox="0 0 16 16" aria-hidden="true" style="width:12px;height:12px;display:block;"><path d="M3 3.5h10v7H8.2L5 13.5v-3H3z" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>' +
      `<span>${badge.count}</span>`
    button.addEventListener('mouseenter', (event) => {
      const mouseEvent = event as MouseEvent
      commentBadgeDebug('badge-mouseenter', {
        badgeKey: badge.key,
        pointerX: mouseEvent.clientX,
        pointerY: mouseEvent.clientY,
      })
      updateCommentBadgeHover(badge)
    })
    button.addEventListener('mouseleave', (event) => {
      const mouseEvent = event as MouseEvent
      const related = mouseEvent.relatedTarget as HTMLElement | null
      commentBadgeDebug('badge-mouseleave', {
        badgeKey: badge.key,
        pointerX: mouseEvent.clientX,
        pointerY: mouseEvent.clientY,
        relatedTag: related?.tagName?.toLowerCase() ?? null,
        relatedId: related?.id ?? null,
        relatedOverlayUi: related?.getAttribute('data-overlay-ui') ?? null,
      })
      scheduleCommentBadgeHoverHide('badge-mouseleave', mouseEvent)
    })
    button.addEventListener('click', (event) => {
      event.preventDefault()
      event.stopPropagation()
      event.stopImmediatePropagation()
      ipcRenderer.send('annotation-open-thread', { annotationId: badge.annotationId })
    })
    commentBadgesLayerEl.appendChild(button)
  }

  if (hoveredBadge) {
    commentBadgeDebug('render-preserve-hover', { badgeKey: hoveredBadge.key })
    updateCommentBadgeHover(hoveredBadge)
  } else if (commentHoverActive) {
    commentBadgeDebug('render-clear-hover-no-match', { hoverBadgeKey: commentHoverBadgeKey })
    hideCommentBadgeHover()
  }
}

export function queueRenderCommentBadges(): void {
  if (commentBadgesRenderRaf) return
  commentBadgesRenderRaf = window.requestAnimationFrame(() => {
    commentBadgesRenderRaf = 0
    renderCommentBadges()
  })
}
