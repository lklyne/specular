import { useEffect, useState, type ComponentType, type MouseEventHandler } from 'react'
import { ContextMenu } from '@base-ui/react/context-menu'
import { Menu } from '@base-ui/react/menu'
import { Laptop, Smartphone, Tablet } from 'lucide-react'
import { InlineEditLabel } from './InlineEditLabel'

type PageVisual = {
  id: string
  label: string
  name?: string
  faviconUrl?: string | null
  width?: number
  height?: number
}

interface PageListItemProps {
  page: PageVisual
  active: boolean
  compact?: boolean
  fullBleedCompact?: boolean
  showDimensions?: boolean
  contentPaddingLeft?: number
  contentPaddingRight?: number
  isDark: boolean
  onClick: MouseEventHandler<HTMLButtonElement>
  selectableId?: string
  onRename?: (name: string) => Promise<boolean>
  onDelete?: () => void
  onRequestEditFocus?: () => void
}

function viewportIcon(label: string, width?: number) {
  if (label.startsWith('iPhone')) return Smartphone
  if (label.startsWith('iPad')) return Tablet
  if (typeof width !== 'number') return Laptop
  if (width < 600) return Smartphone
  if (width < 1100) return Tablet
  return Laptop
}

function PageGlyph({
  faviconUrl,
  Icon,
}: {
  faviconUrl?: string | null
  Icon: ComponentType<{ size?: number; className?: string }>
}) {
  const [imageFailed, setImageFailed] = useState(false)

  useEffect(() => {
    setImageFailed(false)
  }, [faviconUrl])

  if (faviconUrl && !imageFailed) {
    return (
      <img
        alt=""
        aria-hidden="true"
        src={faviconUrl}
        className="h-[14px] w-[14px] shrink-0 rounded-[3px]"
        onError={() => setImageFailed(true)}
      />
    )
  }

  return <Icon size={14} className="shrink-0 text-[var(--surface-panel-foreground-muted)]" />
}

export function PageListItem({
  page,
  active,
  compact = false,
  fullBleedCompact = false,
  showDimensions = true,
  contentPaddingLeft,
  contentPaddingRight,
  isDark,
  onClick,
  selectableId,
  onRename,
  onDelete,
  onRequestEditFocus,
}: PageListItemProps) {
  const [isEditing, setIsEditing] = useState(false)
  const Icon = viewportIcon(page.label, page.width)
  const hasContextMenu = onRename !== undefined || onDelete !== undefined

  function startRename() {
    if (!onRename) return
    setIsEditing(true)
  }

  async function commitRename(next: string) {
    if (onRename && next !== page.label && await onRename(next)) setIsEditing(false)
  }

  const rootClassName = `flex items-center gap-1 text-left text-xs font-normal ${
    compact
      ? `w-full min-w-0 max-w-[240px] ${fullBleedCompact ? 'h-full py-0' : 'py-1.5'} ${
          active
            ? 'bg-[var(--surface-interactive)] text-[var(--surface-panel-foreground)]'
            : `text-[var(--surface-panel-foreground)] hover:bg-[var(--surface-interactive-hover)] dark:hover:bg-[var(--surface-interactive-hover)]`
        }`
      : `w-full py-1.5 ${
          active
            ? 'bg-[var(--surface-interactive)] text-[var(--surface-panel-foreground)]'
            : 'text-[var(--surface-panel-foreground)] hover:bg-[var(--surface-interactive-hover)] dark:hover:bg-[var(--surface-interactive-hover)]'
        }`
  }`
  const horizontalPaddingStyle = {
    paddingLeft: contentPaddingLeft ?? 8,
    paddingRight: contentPaddingRight ?? 8,
  }
  const dividerClassName = isDark ? 'bg-zinc-700' : 'bg-zinc-300'

  const content = isEditing ? (
    <div className={rootClassName} style={horizontalPaddingStyle}>
      <PageGlyph faviconUrl={page.faviconUrl} Icon={Icon} />
      <InlineEditLabel
        value={page.label}
        isEditing
        onCommit={commitRename}
        onCancel={() => setIsEditing(false)}
        variant="sidebar-row"
        isDark={isDark}
        onRequestFocus={onRequestEditFocus}
      />
      {showDimensions && page.width && page.height ? (
        <span className="ml-auto shrink-0 text-xs text-[var(--surface-panel-foreground-muted)]">
          {page.width}&times;{page.height}
        </span>
      ) : null}
    </div>
  ) : (
    <div className={fullBleedCompact ? 'relative flex h-full w-full min-w-0 max-w-[240px]' : undefined}>
      <button
        type="button"
        className={`${rootClassName} box-border appearance-none border-0`}
        style={horizontalPaddingStyle}
        onClick={onClick}
        data-sidebar-selectable-id={selectableId}
        onPointerDown={
          onRename
            ? (event) => {
                if (event.detail !== 2) return
                event.preventDefault()
                event.stopPropagation()
                startRename()
              }
            : undefined
        }
        onDoubleClick={startRename}
        title={page.label}
      >
        <PageGlyph faviconUrl={page.faviconUrl} Icon={Icon} />
        <span className="min-w-0 flex-1 truncate">{page.label}</span>
        {showDimensions && page.width && page.height ? (
          <span className="ml-auto shrink-0 text-xs text-[var(--surface-panel-foreground-muted)]">
            {page.width}&times;{page.height}
          </span>
        ) : null}
      </button>
      {compact && fullBleedCompact ? (
        <span
          aria-hidden="true"
          className={`pointer-events-none absolute right-0 top-0 h-full w-px ${dividerClassName}`}
        />
      ) : null}
    </div>
  )

  if (!hasContextMenu) return content

  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger className={compact ? 'block min-w-0 w-full' : 'block w-full'}>
        {content}
      </ContextMenu.Trigger>
      <Menu.Portal>
        <Menu.Positioner sideOffset={6}>
          <Menu.Popup
            className={`z-50 min-w-40 rounded-[10px] border p-1 shadow-xl outline-none ${
              isDark
                ? 'border-zinc-700 bg-zinc-900 text-[var(--surface-panel-foreground)]'
                : 'border-zinc-200 bg-white text-[var(--surface-panel-foreground)]'
            }`}
          >
            {onRename ? (
              <Menu.Item
                className={`flex cursor-default items-center gap-2 rounded-[7px] px-2.5 py-1.5 text-xs outline-none ${
                  isDark
                    ? 'text-[var(--surface-panel-foreground)] data-[highlighted]:bg-zinc-800'
                    : 'text-[var(--surface-panel-foreground)] data-[highlighted]:bg-zinc-100'
                }`}
                onClick={startRename}
              >
                <span>Rename</span>
              </Menu.Item>
            ) : null}
            {onDelete ? (
              <Menu.Item
                className={`flex cursor-default items-center gap-2 rounded-[7px] px-2.5 py-1.5 text-xs outline-none ${
                  isDark
                    ? 'text-[var(--surface-panel-foreground)] data-[highlighted]:bg-zinc-800'
                    : 'text-[var(--surface-panel-foreground)] data-[highlighted]:bg-zinc-100'
                }`}
                onClick={onDelete}
              >
                <span>Delete</span>
              </Menu.Item>
            ) : null}
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Portal>
    </ContextMenu.Root>
  )
}
