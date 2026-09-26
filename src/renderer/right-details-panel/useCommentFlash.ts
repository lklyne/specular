import { useEffect, useState, type RefObject } from 'react'
import { rightDetailsPanelApi } from './rightDetailsPanelApi'

const FLASH_MS = 200
/** How long a flash waits for its comment to render after a thread switch. */
const PENDING_MS = 1000

/**
 * Flashes the comment the user just focused on the canvas, so a click on a pin
 * visibly lands in the panel. The focus event can beat the thread switch that
 * renders the comment, so the request waits for panel data to catch up.
 */
export function useCommentFlash(rootRef: RefObject<HTMLElement | null>, dataVersion: unknown) {
  const [pending, setPending] = useState<{ annotationId: string; at: number } | null>(null)

  useEffect(
    () =>
      rightDetailsPanelApi.onAnnotationThreadOpen(({ annotationId }) => {
        setPending(annotationId ? { annotationId, at: performance.now() } : null)
      }),
    [],
  )

  useEffect(() => {
    if (!pending) return
    const root = rootRef.current
    const targets = root
      ? root.querySelectorAll<HTMLElement>(`[data-annotation-id="${CSS.escape(pending.annotationId)}"]`)
      : []
    if (targets.length > 0) {
      targets[0].scrollIntoView({ block: 'nearest' })
      for (const target of targets) flash(target)
      setPending(null)
      return
    }
    const timer = window.setTimeout(
      () => setPending(null),
      Math.max(0, PENDING_MS - (performance.now() - pending.at)),
    )
    return () => window.clearTimeout(timer)
  }, [pending, dataVersion, rootRef])
}

function flash(element: HTMLElement): void {
  const style = getComputedStyle(element)
  // One notch toward the text color reads as "darker" in light and dark themes.
  const from = `color-mix(in srgb, ${style.backgroundColor}, ${style.color} 12%)`
  element.animate([{ backgroundColor: from }, { backgroundColor: style.backgroundColor }], {
    duration: FLASH_MS,
    easing: 'ease-out',
  })
}
