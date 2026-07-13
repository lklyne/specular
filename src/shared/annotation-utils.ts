import type { Annotation, AnnotationStatus } from './types'
import { canonicalPageUrl } from './page-anchor'

export function truncate(value: string, max: number): string {
  if (value.length <= max) return value
  return value.slice(0, max - 1) + '…'
}

export function isUnresolved(status: AnnotationStatus): boolean {
  return status === 'pending' || status === 'acknowledged'
}

/** Origin of the document a page-bound annotation was placed on, or null for
 *  canvas-bound annotations (no `pageAnchor`) and non-URL anchors. */
export function annotationOrigin(annotation: Annotation): string | null {
  const pageUrl = annotation.pageAnchor?.pageUrl
  if (!pageUrl) return null
  try {
    return new URL(pageUrl).origin
  } catch {
    return null
  }
}

/** Annotation-flavored alias for the shared page-anchor canonicalization. */
export function canonicalAnnotationUrl(value: string | undefined | null): string | undefined {
  return canonicalPageUrl(value)
}
