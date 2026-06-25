// ADR 0008 — unified canvas-item popup compound component.

import { type ReactNode } from 'react'
import { AlignHorizontalDistributeCenter, Copy, Maximize2, Trash2 } from 'lucide-react'
import {
  paletteSlots,
  resolveCanvasColor,
  type CanvasColorRole,
  type CanvasColorSlot,
  type CanvasPalette,
} from '../../shared/canvas-colors'
import type { CanvasBgElectronAPI, LayoutUpdateData } from '../../shared/types'
import {
  useAnchoredPosition,
  useMultiAnchoredPosition,
  type AnchorSlot,
} from './useAnchoredPosition'

type Placement = 'above' | 'below' | 'overlay'
type Align = 'stretch' | 'center'

type RootProps = {
  layout: LayoutUpdateData
  open: boolean
  slot?: AnchorSlot
  placement?: Placement
  align?: Align
  offset?: number
  children: ReactNode
} & (
  | { entityId: string; entityIds?: undefined }
  | { entityIds: readonly string[]; entityId?: undefined }
)

function Root(props: RootProps) {
  const {
    layout,
    open,
    slot = 'body',
    placement = 'below',
    align = 'center',
    offset = 8,
    children,
  } = props
  const singleRect = useAnchoredPosition(
    layout,
    props.entityId ?? '',
    slot,
  )
  const multiRect = useMultiAnchoredPosition(layout, props.entityIds ?? [], slot)
  const rect = props.entityIds !== undefined ? multiRect : singleRect
  if (!open || !rect) return null
  const style = popupStyle(rect, placement, align, offset)
  return (
    <div
      data-overlay-ui
      className="pointer-events-auto absolute"
      style={style}
    >
      {children}
    </div>
  )
}

function popupStyle(
  rect: { x: number; y: number; width: number; height: number },
  placement: Placement,
  align: Align,
  offset: number,
): React.CSSProperties {
  if (placement === 'overlay') {
    return { left: rect.x, top: rect.y, width: rect.width, height: rect.height }
  }
  const isAbove = placement === 'above'
  const top = isAbove ? rect.y - offset : rect.y + rect.height + offset
  const verticalTransform = isAbove ? 'translateY(-100%)' : ''
  if (align === 'stretch') {
    return {
      left: rect.x + rect.width / 2,
      top,
      minWidth: rect.width,
      transform: `translateX(-50%) ${verticalTransform}`.trim(),
    }
  }
  return {
    left: rect.x + rect.width / 2,
    top,
    transform: `translateX(-50%) ${verticalTransform}`.trim(),
  }
}

// ADR 0008 §1 — viewport-anchored (tool mode) strip below toolbar.
// Uses `layout.toolbarCenterX` (window-coord px from main) verbatim; no
// devtools-panel compensation needed because toolbar layout drives placement.
function ViewportAnchor({
  layout,
  open,
  offset = 8,
  children,
}: {
  layout: LayoutUpdateData
  open: boolean
  offset?: number
  children: ReactNode
}) {
  if (!open) return null
  return (
    <>
      {/* Bridge across the gap between the toolbar and the popup. Marked as
          overlay UI so the placement-preview ghost clears while the cursor
          is in this strip, instead of stamping through the gap. */}
      <div
        data-overlay-ui
        aria-hidden
        className="pointer-events-auto absolute left-0 right-0"
        style={{ top: 0, height: offset }}
      />
      <div
        data-overlay-ui
        className="pointer-events-auto absolute"
        style={{
          top: offset,
          left: layout.toolbarCenterX,
          transform: 'translateX(-50%)',
        }}
      >
        {children}
      </div>
    </>
  )
}

// Stops mousedown so clicks inside don't fall through to canvas gestures.
function Frame({
  isDark,
  className = '',
  children,
}: {
  isDark: boolean
  className?: string
  children: ReactNode
}) {
  return (
    <div
      className={`flex items-center gap-1 rounded-[10px] border p-1 ${
        isDark ? 'text-zinc-100' : 'text-zinc-900'
      } ${className}`.trim()}
      style={{
        background: isDark ? '#3a3836' : '#ece9e7',
        borderColor: isDark ? '#414141' : '#dcdcda',
        boxShadow: isDark
          ? '0 10px 8px -6px rgba(0,0,0,.58), 0 4px 16px 0 rgba(0,0,0,.5)'
          : '0 10px 8px -6px rgba(0,0,0,.18), 0 4px 16px 0 rgba(199,193,188,.5)',
      }}
      onPointerDown={(event) => event.stopPropagation()}
    >
      {children}
    </div>
  )
}

function Section({ children, grow = false }: { children: ReactNode; grow?: boolean }) {
  return (
    <div className={`flex items-center gap-1${grow ? ' min-w-0 flex-1' : ''}`}>
      {children}
    </div>
  )
}

function Divider({ isDark }: { isDark: boolean }) {
  return (
    <div
      aria-hidden
      className={`mx-1 h-4 w-px shrink-0 ${isDark ? 'bg-white/20' : 'bg-zinc-900/20'}`}
    />
  )
}

function popupIconButtonClass(isDark: boolean, active = false): string {
  const base =
    'flex h-6 w-6 items-center justify-center rounded-[6px] border-0 transition-colors'
  if (active) {
    return isDark
      ? `${base} bg-[rgba(253,248,245,0.1)] text-zinc-100`
      : `${base} bg-[#fdf8f5] text-zinc-900`
  }
  return isDark
    ? `${base} text-zinc-300 hover:bg-[rgba(253,248,245,0.1)] hover:text-zinc-100`
    : `${base} text-zinc-600 hover:bg-[#fdf8f5] hover:text-zinc-900`
}

function IconButton({
  isDark,
  active = false,
  title,
  ariaLabel,
  onClick,
  children,
}: {
  isDark: boolean
  active?: boolean
  title: string
  ariaLabel: string
  onClick: () => void
  children: ReactNode
}) {
  return (
    <button
      type="button"
      className={popupIconButtonClass(isDark, active)}
      onClick={onClick}
      title={title}
      aria-label={ariaLabel}
      aria-pressed={active}
    >
      {children}
    </button>
  )
}

function ColorSwatch({
  isDark,
  active,
  color,
  ariaLabel,
  onClick,
}: {
  isDark: boolean
  active: boolean
  color: string
  ariaLabel: string
  onClick: () => void
}) {
  return (
    <button
      type="button"
      aria-label={ariaLabel}
      className={`flex h-5 w-5 items-center justify-center rounded-full border transition-colors ${
        isDark ? 'bg-[#3a3836]' : 'bg-[#ece9e7]'
      }`}
      style={{ borderColor: active ? color : 'transparent' }}
      onClick={onClick}
    >
      <span
        className="block h-3 w-3 rounded-full"
        style={{ background: color }}
      />
    </button>
  )
}

function EntityActions({
  isDark,
  noun,
  count,
  onDuplicate,
  onDelete,
  api,
}: {
  isDark: boolean
  noun: string
  count: number
  onDuplicate: () => void
  onDelete: () => void
  api?: Pick<CanvasBgElectronAPI, 'focusSelection'> &
    Partial<Pick<CanvasBgElectronAPI, 'distributeSelection'>>
}) {
  return (
    <Section>
      {api && (
        <IconButton
          isDark={isDark}
          title={`Focus ${noun}`}
          ariaLabel={`Focus ${noun}`}
          onClick={() => api.focusSelection()}
        >
          <Maximize2 size={14} />
        </IconButton>
      )}
      {api?.distributeSelection && count >= 3 && (
        <IconButton
          isDark={isDark}
          title="Distribute spacing"
          ariaLabel="Distribute spacing"
          onClick={() => api.distributeSelection?.()}
        >
          <AlignHorizontalDistributeCenter size={14} />
        </IconButton>
      )}
      <IconButton
        isDark={isDark}
        title={`Duplicate ${noun}`}
        ariaLabel={`Duplicate ${noun}`}
        onClick={onDuplicate}
      >
        <Copy size={14} />
      </IconButton>
      <IconButton
        isDark={isDark}
        title={`Delete ${noun}`}
        ariaLabel={`Delete ${noun}`}
        onClick={onDelete}
      >
        <Trash2 size={14} />
      </IconButton>
    </Section>
  )
}

function PaletteSection({
  isDark,
  palette,
  activeSlot,
  role,
  noun,
  onPick,
}: {
  isDark: boolean
  palette: CanvasPalette
  activeSlot: CanvasColorSlot | null
  role: CanvasColorRole
  noun?: string
  onPick: (storage: string) => void
}) {
  return (
    <Section>
      {paletteSlots(palette).map((slot) => {
        const swatch = slot.hex ?? resolveCanvasColor(slot.storage, { role, isDark })
        const ariaLabel = noun
          ? `Set ${noun} color to ${slot.label}`
          : `Set color to ${slot.label}`
        return (
          <ColorSwatch
            key={slot.id}
            isDark={isDark}
            active={activeSlot === slot.id}
            color={swatch}
            ariaLabel={ariaLabel}
            onClick={() => onPick(slot.storage)}
          />
        )
      })}
    </Section>
  )
}

export const CanvasItemPopup = {
  Root,
  ViewportAnchor,
  Frame,
  Section,
  Divider,
  IconButton,
  ColorSwatch,
  EntityActions,
  PaletteSection,
}
