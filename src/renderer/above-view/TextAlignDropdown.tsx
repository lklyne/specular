import { Menu } from '@base-ui/react/menu'
import {
  AlignCenter,
  AlignLeft,
  AlignRight,
  ChevronDown,
  type LucideIcon,
} from 'lucide-react'
import type { ShapeTextAlign } from '../../shared/types'
import {
  POPUP_SURFACE_CLASS,
  dropdownTriggerClass,
  popupSurfaceStyle,
} from '../shared/popupSurface'

const ALIGNMENTS: Array<{
  alignment: ShapeTextAlign
  label: string
  Icon: LucideIcon
}> = [
  { alignment: 'left', label: 'Align text left', Icon: AlignLeft },
  { alignment: 'center', label: 'Align text center', Icon: AlignCenter },
  { alignment: 'right', label: 'Align text right', Icon: AlignRight },
]

export function TextAlignDropdown({
  isDark,
  alignment,
  onPick,
}: {
  isDark: boolean
  alignment: ShapeTextAlign | null
  onPick: (alignment: ShapeTextAlign) => void
}) {
  const ActiveIcon =
    ALIGNMENTS.find((option) => option.alignment === alignment)?.Icon ??
    AlignCenter

  return (
    <Menu.Root>
      <Menu.Trigger
        className={dropdownTriggerClass(isDark, 'pl-1.5 pr-1')}
        aria-label="Text alignment"
        title="Text alignment"
      >
        <ActiveIcon size={14} />
        <ChevronDown size={12} />
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Positioner
          align="center"
          side="bottom"
          sideOffset={8}
          style={{ zIndex: 50 }}
        >
          <Menu.Popup
            data-overlay-ui
            className={`flex items-center gap-1 ${POPUP_SURFACE_CLASS}`}
            style={popupSurfaceStyle(isDark)}
            onPointerDown={(event) => event.stopPropagation()}
          >
            {ALIGNMENTS.map(({ alignment: option, label, Icon }) => (
              <Menu.Item
                key={option}
                aria-label={label}
                title={label}
                onClick={() => onPick(option)}
                className={`flex h-7 w-7 cursor-default items-center justify-center rounded-[6px] outline-none transition-colors ${
                  alignment === option
                    ? isDark
                      ? 'bg-[rgba(253,248,245,0.1)] text-[var(--surface-foreground)]'
                      : 'bg-[var(--color-stone-200)] text-[var(--surface-foreground)]'
                    : isDark
                      ? 'text-[var(--surface-foreground-muted)] hover:bg-[rgba(253,248,245,0.08)] hover:text-[var(--surface-foreground)]'
                      : 'text-[var(--surface-foreground-muted)] hover:bg-[var(--color-stone-100)] hover:text-[var(--surface-foreground)]'
                }`}
              >
                <Icon size={15} />
              </Menu.Item>
            ))}
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Portal>
    </Menu.Root>
  )
}
