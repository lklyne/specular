import { Tooltip as Base } from '@base-ui/react/tooltip'
import type { ReactElement, ReactNode } from 'react'

/**
 * Shared styled tooltip. Wraps a single trigger element (usually a button) via
 * base-ui's `render` merge, so the trigger keeps its own props and handlers.
 *
 * Always opens below the trigger (no flipping) — the toolbar has nothing above
 * it, and consistent placement reads calmer. An optional `shortcut` renders
 * muted to the right of the label.
 *
 * The tip only adds a visual hint — it does not provide an accessible name, so
 * icon-only triggers must still set their own `aria-label`.
 *
 * `onOpenChange` lets a host react to open/close. The toolbar uses it to grow
 * its clipped WebContentsView (see ToolbarTooltip) so the tip can paint below
 * the 44px strip; canvas popups live in the full-window aboveView and ignore it.
 */
export function Tooltip({
  label,
  shortcut,
  children,
  sideOffset = 6,
  onOpenChange,
}: {
  label: ReactNode
  /** Muted keyboard hint shown to the right, e.g. `V` or `⌘⇧R`. */
  shortcut?: string
  /** The trigger — a single focusable element. */
  children: ReactElement
  sideOffset?: number
  onOpenChange?: (open: boolean) => void
}) {
  if (label == null || label === '') return children
  return (
    <Base.Root onOpenChange={(open) => onOpenChange?.(open)}>
      <Base.Trigger render={children} />
      <Base.Portal>
        <Base.Positioner
          side="bottom"
          sideOffset={sideOffset}
          collisionAvoidance={{ side: 'none' }}
        >
          <Base.Popup
            style={{
              // Theme-independent dark chip (Figma-style) so it reads in both
              // light and dark without wiring theme vars across bundles.
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              background: 'rgba(38,38,40,0.97)',
              color: '#f4f4f5',
              border: '1px solid rgba(255,255,255,0.08)',
              borderRadius: 6,
              padding: '3px 7px',
              fontSize: 12,
              lineHeight: 1.3,
              fontWeight: 500,
              whiteSpace: 'nowrap',
              boxShadow: '0 4px 14px rgba(0,0,0,0.28)',
              // The tip must never capture input — it renders inside the
              // (briefly expanded, transparent) toolbar view over the canvas.
              pointerEvents: 'none',
              userSelect: 'none',
              zIndex: 100,
            }}
          >
            <span>{label}</span>
            {shortcut ? (
              <span style={{ color: 'rgba(244,244,245,0.5)' }}>{shortcut}</span>
            ) : null}
          </Base.Popup>
        </Base.Positioner>
      </Base.Portal>
    </Base.Root>
  )
}

/**
 * Shared open delay for a group of tooltips. Mount once near a surface root;
 * adjacent triggers then skip the delay while one was recently open.
 */
export function TooltipProvider({ children }: { children: ReactNode }) {
  return (
    <Base.Provider delay={450} closeDelay={0}>
      {children}
    </Base.Provider>
  )
}
