import type { ComponentProps, ReactElement, ReactNode } from 'react'
import { Popover } from '@base-ui/react/popover'
import { POPUP_SURFACE_CLASS, popupSurfaceStyle } from './popupSurface'

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
            className={`${POPUP_SURFACE_CLASS} ${isDark ? 'text-zinc-100' : 'text-zinc-900'}`}
            style={popupSurfaceStyle(isDark)}
          >
            {children}
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  )
}
