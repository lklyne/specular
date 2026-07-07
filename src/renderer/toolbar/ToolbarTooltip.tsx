import { useEffect, useId, type ComponentProps } from 'react'
import { Tooltip } from '../shared/Tooltip'
import { toolbarApi } from './toolbarApi'

// The toolbar renders inside a 44px-tall WebContentsView that clips overflow.
// Like dropdowns, a tooltip has to grow the view to paint below the strip —
// but only a shallow band (main bounds the expansion), so the transparent
// click-swallow zone over the canvas stays small.
//
// ponytail: bounded-band expand; upgrade to an input-transparent overlay
// (cursorOverlayWindow pattern) if the swallow band ever bites.
//
// Track the *set* of currently-open triggers, not a +1/-1 count: base-ui fires
// onOpenChange unconditionally (setOpen never dedupes against current state), so
// an A→B hover closes A twice — once via its own mouseleave, once via the delay
// group's forced sibling-close — while opening B once. A counter drifts to 0 and
// collapses the view under B's still-visible tip; a Set makes redundant closes
// no-ops.
//
// Grow immediately, but defer the collapse: moving between adjacent triggers can
// fire close(A) *before* open(B), so a synchronous collapse would drop the view
// to 44px for a frame and clip B's tip before its regrown bounds land. The
// pending collapse is cancelled by the next open, so the view stays grown across
// the transition.
const openTriggers = new Set<string>()
let collapseTimer: ReturnType<typeof setTimeout> | null = null

function setOpen(id: string, open: boolean): void {
  const had = openTriggers.size > 0
  if (open) openTriggers.add(id)
  else openTriggers.delete(id)
  const has = openTriggers.size > 0
  if (!had && has) {
    if (collapseTimer) {
      clearTimeout(collapseTimer)
      collapseTimer = null
    }
    toolbarApi.tooltipOpen()
  } else if (had && !has && !collapseTimer) {
    collapseTimer = setTimeout(() => {
      collapseTimer = null
      if (openTriggers.size === 0) toolbarApi.tooltipClose()
    }, 120)
  }
}

export function ToolbarTooltip(
  props: Omit<ComponentProps<typeof Tooltip>, 'onOpenChange'>,
) {
  const id = useId()
  // Unmounting while open would otherwise strand the id in the set.
  useEffect(() => () => setOpen(id, false), [id])
  return <Tooltip {...props} onOpenChange={(open) => setOpen(id, open)} />
}
