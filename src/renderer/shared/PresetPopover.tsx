import type { ComponentProps, ReactElement, ReactNode } from 'react'
import { Popover } from '@base-ui/react/popover'
import { TOOLBAR_HEIGHT } from '../../shared/constants'
import { POPUP_SURFACE_CLASS, popupSurfaceStyle } from './popupSurface'

// Anchors to the toolbar strip's bottom edge (not the inset trigger button) so
// the gap matches the add-page popup; triggerSelector aligns it horizontally.
export function toolbarStripAnchor(triggerSelector: string) {
  return {
    getBoundingClientRect: () => {
      const strip = document.querySelector('.toolbar-bar')?.getBoundingClientRect()
      const trigger = document.querySelector(triggerSelector)?.getBoundingClientRect()
      const bottom = strip?.bottom ?? TOOLBAR_HEIGHT
      const left = trigger?.left ?? 0
      const width = trigger?.width ?? 0
      return new DOMRect(left, bottom, width, 0)
    },
  }
}

// Shared popover shell for the toolbar/page pickers: the styled popup frame +
// portal + positioner. Callers own their open state and render their own body.
// The box matches the add-page tool popup (CanvasItemPopup.Frame) via popupSurface.
export function PresetPopover({
  isDark,
  open,
  onOpenChange,
  side = 'bottom',
  align = 'center',
  sideOffset = 4,
  anchor,
  trigger,
  children,
}: {
  isDark: boolean
  open: boolean
  onOpenChange: (open: boolean) => void
  side?: 'top' | 'bottom' | 'left' | 'right'
  align?: 'start' | 'center' | 'end'
  sideOffset?: number
  // Override the default trigger anchor (e.g. pin below the toolbar strip).
  anchor?: ComponentProps<typeof Popover.Positioner>['anchor']
  trigger: ReactElement
  children: ReactNode
}) {
  return (
    <Popover.Root open={open} onOpenChange={onOpenChange}>
      <Popover.Trigger render={trigger} />
      <Popover.Portal>
        <Popover.Positioner anchor={anchor} side={side} align={align} sideOffset={sideOffset} collisionAvoidance={{ side: 'none', align: 'none' }} style={{ zIndex: 100 }}>
          <Popover.Popup
            data-overlay-ui
            className={`${POPUP_SURFACE_CLASS} text-[var(--surface-panel-foreground)]`}
            style={popupSurfaceStyle(isDark)}
          >
            {children}
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  )
}
