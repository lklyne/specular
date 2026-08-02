import { Menu } from '@base-ui/react/menu'
import { ChevronDown } from 'lucide-react'
import type { EdgeLineStyle } from '../../shared/types'
import {
  POPUP_SURFACE_CLASS,
  dropdownTriggerClass,
  popupSurfaceStyle,
} from '../shared/popupSurface'
import { BorderGlyph } from './BorderDropdown'
import { StrokeWidthSwatch } from './StrokeWidthSwatch'

const EDGE_STROKE_WIDTHS = [1.5, 3] as const

function LineGlyph({
  dashed,
  width,
}: {
  dashed: boolean
  width: number
}) {
  return (
    <svg width={14} height={14} viewBox="0 0 14 14" aria-hidden>
      {dashed ? (
        <>
          <rect x={0.5} y={6} width={3} height={2} rx={1} fill="currentColor" />
          <rect x={5.5} y={6} width={3} height={2} rx={1} fill="currentColor" />
          <rect x={10.5} y={6} width={3} height={2} rx={1} fill="currentColor" />
        </>
      ) : (
        <line
          x1={1}
          y1={7}
          x2={13}
          y2={7}
          stroke="currentColor"
          strokeWidth={width}
          strokeLinecap="round"
        />
      )}
    </svg>
  )
}

function optionClass(isDark: boolean, active: boolean): string {
  const state = active
    ? isDark
      ? 'bg-[rgba(253,248,245,0.1)] text-[var(--surface-panel-foreground)]'
      : 'bg-[var(--color-stone-200)] text-[var(--surface-panel-foreground)]'
    : isDark
      ? 'text-[var(--surface-panel-foreground-muted)] hover:bg-[rgba(253,248,245,0.08)] hover:text-[var(--surface-panel-foreground)]'
      : 'text-[var(--surface-panel-foreground-muted)] hover:bg-[var(--color-stone-100)] hover:text-[var(--surface-panel-foreground)]'
  return `flex h-6 w-6 items-center justify-center rounded-[6px] outline-none transition-colors ${state}`
}

export function EdgeStrokeDropdown({
  isDark,
  lineStyle,
  strokeWidth,
  onSetStyle,
  onSetWidth,
}: {
  isDark: boolean
  lineStyle: EdgeLineStyle
  strokeWidth: number
  onSetStyle: (style: EdgeLineStyle) => void
  onSetWidth: (width: number) => void
}) {
  return (
    <Menu.Root>
      <Menu.Trigger
        className={dropdownTriggerClass(isDark, 'pl-1.5 pr-1')}
        aria-label="Edge stroke"
        title="Edge stroke"
      >
        <BorderGlyph size={14} />
        <ChevronDown size={12} />
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Positioner align="center" side="bottom" sideOffset={8} style={{ zIndex: 50 }}>
          <Menu.Popup
            data-overlay-ui
            className={`flex items-center gap-1 ${POPUP_SURFACE_CLASS}`}
            style={popupSurfaceStyle(isDark)}
            onPointerDown={(event) => event.stopPropagation()}
          >
            {EDGE_STROKE_WIDTHS.map((width, index) => (
              <StrokeWidthSwatch
                key={width}
                isDark={isDark}
                active={strokeWidth === width}
                variant={index === 0 ? 'thin' : 'thick'}
                ariaLabel={index === 0 ? 'Thin edge' : 'Thick edge'}
                onClick={() => onSetWidth(width)}
              />
            ))}
            <div className={`mx-0.5 h-5 w-px ${isDark ? 'bg-zinc-700' : 'bg-zinc-200'}`} />
            {(['solid', 'dashed'] as const).map((style) => (
              <Menu.Item
                key={style}
                aria-label={style === 'solid' ? 'Regular edge' : 'Dashed edge'}
                title={style === 'solid' ? 'Regular' : 'Dashed'}
                closeOnClick={false}
                className={optionClass(isDark, lineStyle === style)}
                onClick={() => onSetStyle(style)}
              >
                <LineGlyph dashed={style === 'dashed'} width={2} />
              </Menu.Item>
            ))}
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Portal>
    </Menu.Root>
  )
}
