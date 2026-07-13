import type { Annotation, AnnotationStatus } from './types'

export function truncate(value: string, max: number): string {
  if (value.length <= max) return value
  return value.slice(0, max - 1) + '…'
}

export function isUnresolved(status: AnnotationStatus): boolean {
  return status === 'pending' || status === 'acknowledged'
}

export function annotationOrigin(annotation: Annotation): string | null {
  const pageUrl = annotation.metadata?.pageUrl
  if (!pageUrl) return null
  try {
    return new URL(pageUrl).origin
  } catch {
    return null
  }
}

/**
 * Canonical form for page-URL comparison: hash stripped, otherwise the URL
 * as-is. Non-URL strings pass through so file:// fixtures and dev servers
 * with unusual schemes still compare by exact string.
 */
export function canonicalAnnotationUrl(value: string | undefined | null): string | undefined {
  if (!value) return undefined
  const trimmed = value.trim()
  if (!trimmed) return undefined
  try {
    const parsed = new URL(trimmed)
    parsed.hash = ''
    return parsed.toString()
  } catch {
    return trimmed
  }
}

/**
 * The page a page-anchored annotation lives on. Element and page anchors name
 * it directly; region anchors are canvas-space but record the primary page
 * they were drawn over in metadata. Canvas anchors have no page context.
 */
export function annotationContextPageId(annotation: Annotation): string | null {
  const anchor = annotation.anchor
  if (anchor.type === 'element' || anchor.type === 'page') return anchor.pageId
  if (anchor.type === 'region') {
    return (
      annotation.metadata?.regionComponents?.[0]?.pageId ??
      annotation.metadata?.regionElements?.[0]?.pageId ??
      null
    )
  }
  return null
}

/**
 * Whether an annotation still belongs on the page's current document. False
 * only when both sides carry a URL and they disagree (hash-insensitive) —
 * i.e. the page has navigated away from the URL the annotation was created
 * on. Annotations without a recorded pageUrl (older files, agent-created)
 * always match, as do pages with no URL yet (loading, about:blank).
 */
export function annotationMatchesPageUrl(
  annotation: Annotation,
  currentPageUrl: string | undefined | null,
): boolean {
  const annotationUrl = canonicalAnnotationUrl(annotation.metadata?.pageUrl)
  const pageUrl = canonicalAnnotationUrl(currentPageUrl)
  if (!annotationUrl || !pageUrl) return true
  return annotationUrl === pageUrl
}
