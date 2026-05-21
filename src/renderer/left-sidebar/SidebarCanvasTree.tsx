import { useState } from 'react'
import { Collapsible } from '@base-ui/react/collapsible'
import { ContextMenu } from '@base-ui/react/context-menu'
import { Menu } from '@base-ui/react/menu'
import {
  ChevronDown,
  ChevronRight,
  Circle,
  Diamond,
  Folder,
  FolderOpen,
  PenLine,
  Square,
  StickyNote,
} from 'lucide-react'
import type {
  LeftSidebarElectronAPI,
  SidebarCanvasItem,
  SidebarGroupItem,
  SidebarSectionKey,
} from '../../shared/types'
import { iconForFilePath } from '../shared/fileIcon'
import { PageListItem } from '../shared/pageListItem'
import { InlineEditLabel } from '../shared/InlineEditLabel'
import { useDragReorder } from './useDragReorder'

const RENAMABLE_FILE_PATTERN = /\.(md|wireframe\.json)$/i

const LIST_OUTER_LEFT_PADDING = 14
const LIST_OUTER_RIGHT_PADDING = 8
const LIST_ROW_INNER_X_PADDING = 8
const TREE_DEPTH_STEP = 14

function EntityListItem({
  icon,
  label,
  active,
  isDark,
  onClick,
  onRename,
  onDelete,
  deleteLabel = 'Delete',
  depth,
}: {
  icon: React.ReactNode
  label: string
  active: boolean
  isDark: boolean
  onClick: () => void
  onRename?: (name: string) => void
  onDelete: () => void
  deleteLabel?: string
  depth: number
}) {
  const [isEditing, setIsEditing] = useState(false)
  const rootClassName = `flex w-full items-center gap-1 py-1.5 text-left text-xs font-normal ${
    active
      ? isDark
        ? 'bg-[var(--surface-interactive)] text-zinc-100'
        : 'bg-[var(--surface-interactive)] text-zinc-900'
      : isDark
        ? 'text-zinc-200 hover:bg-[var(--surface-interactive-hover)]'
        : 'text-zinc-800 hover:bg-[var(--surface-interactive-hover)]'
  }`
  const rowStyle = {
    paddingLeft: LIST_OUTER_LEFT_PADDING + LIST_ROW_INNER_X_PADDING + depth * TREE_DEPTH_STEP,
    paddingRight: LIST_OUTER_RIGHT_PADDING + LIST_ROW_INNER_X_PADDING,
  }
  function startRename() {
    if (!onRename) return
    setIsEditing(true)
  }

  function commitRename(next: string) {
    setIsEditing(false)
    if (onRename && next && next !== label) onRename(next)
  }

  const row = isEditing ? (
    <div className={rootClassName} style={rowStyle}>
      {icon}
      <InlineEditLabel
        value={label}
        isEditing
        onCommit={commitRename}
        onCancel={() => setIsEditing(false)}
        variant="sidebar-row"
        isDark={isDark}
      />
    </div>
  ) : (
    <button
      type="button"
      className={rootClassName}
      style={rowStyle}
      onClick={onClick}
      onDoubleClick={onRename ? startRename : undefined}
      title={label}
    >
      {icon}
      <span className="min-w-0 flex-1 truncate">{label}</span>
    </button>
  )

  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger className="block w-full">{row}</ContextMenu.Trigger>
      <Menu.Portal>
        <Menu.Positioner sideOffset={6}>
          <Menu.Popup
            className={`z-50 min-w-40 rounded-[10px] border p-1 shadow-xl outline-none ${
              isDark
                ? 'border-[var(--surface-popover-border)] bg-[var(--surface-popover-subtle)] text-zinc-100'
                : 'border-[var(--surface-popover-border)] bg-[var(--surface-popover-subtle)] text-zinc-900'
            }`}
          >
            {onRename ? (
              <Menu.Item
                className={`flex cursor-default items-center gap-2 rounded-[7px] px-2.5 py-1.5 text-xs outline-none ${
                  isDark
                    ? 'text-zinc-100 data-[highlighted]:bg-[var(--surface-popover)]'
                    : 'text-zinc-900 data-[highlighted]:bg-[var(--surface-popover)]'
                }`}
                onClick={startRename}
              >
                <span>Rename</span>
              </Menu.Item>
            ) : null}
            <Menu.Item
              className={`flex cursor-default items-center gap-2 rounded-[7px] px-2.5 py-1.5 text-xs outline-none ${
                isDark
                  ? 'text-zinc-100 data-[highlighted]:bg-[var(--surface-popover)]'
                  : 'text-zinc-900 data-[highlighted]:bg-[var(--surface-popover)]'
              }`}
              onClick={onDelete}
            >
              <span>{deleteLabel}</span>
            </Menu.Item>
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Portal>
    </ContextMenu.Root>
  )
}

function GroupTreeItem({
  group,
  depth,
  selectedEntityIds,
  selectedGroupId,
  isDark,
  api,
  section,
  parentId,
}: {
  group: SidebarGroupItem
  depth: number
  selectedEntityIds: string[]
  selectedGroupId: string | null
  isDark: boolean
  api: LeftSidebarElectronAPI
  section: SidebarSectionKey
  parentId: string | null
}) {
  const [expanded, setExpanded] = useState(false)
  const [isEditing, setIsEditing] = useState(false)
  const isSelected = selectedGroupId === group.id

  function startRename() {
    setIsEditing(true)
  }

  function commitRename(next: string) {
    setIsEditing(false)
    if (next && next !== group.label) api.renameGroup(group.id, next)
  }

  const rowPaddingLeft = LIST_OUTER_LEFT_PADDING + LIST_ROW_INNER_X_PADDING + depth * TREE_DEPTH_STEP
  const rowPaddingRight = LIST_OUTER_RIGHT_PADDING + LIST_ROW_INNER_X_PADDING
  const chevronLeft = rowPaddingLeft - 16
  const rowClassName = `flex w-full items-center gap-1 py-1.5 text-left text-xs font-normal ${
    isSelected
      ? isDark
        ? 'bg-[var(--surface-interactive)] text-zinc-100'
        : 'bg-[var(--surface-interactive)] text-zinc-900'
      : isDark
        ? 'text-zinc-200 hover:bg-[var(--surface-interactive-hover)]'
        : 'text-zinc-800 hover:bg-[var(--surface-interactive-hover)]'
  }`
  const rowStyle = { paddingLeft: rowPaddingLeft, paddingRight: rowPaddingRight }

  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger className="block w-full">
        <Collapsible.Root open={expanded} onOpenChange={setExpanded}>
          <div className="relative">
            {isEditing ? (
              <div className={rowClassName} style={rowStyle}>
                {expanded ? (
                  <FolderOpen size={14} className="shrink-0 text-zinc-500" />
                ) : (
                  <Folder size={14} className="shrink-0 text-zinc-500" />
                )}
                <InlineEditLabel
                  value={group.label}
                  isEditing
                  onCommit={commitRename}
                  onCancel={() => setIsEditing(false)}
                  variant="sidebar-row"
                  isDark={isDark}
                />
                <span className="ml-auto shrink-0 text-xs text-zinc-400">{group.entityCount}</span>
              </div>
            ) : (
              <button
                type="button"
                className={rowClassName}
                style={rowStyle}
                onClick={() => api.revealGroup(group.id)}
                onDoubleClick={startRename}
                title={group.label}
              >
                {expanded ? (
                  <FolderOpen size={14} className="shrink-0 text-zinc-500" />
                ) : (
                  <Folder size={14} className="shrink-0 text-zinc-500" />
                )}
                <span className="min-w-0 truncate">{group.label}</span>
                <span className="ml-auto shrink-0 text-xs text-zinc-400">{group.entityCount}</span>
              </button>
            )}
            <Collapsible.Trigger
              className="absolute top-1/2 flex -translate-y-1/2 items-center justify-center text-zinc-500"
              style={{ left: chevronLeft }}
              onClick={(event) => event.stopPropagation()}
            >
              {expanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
            </Collapsible.Trigger>
          </div>
          <Collapsible.Panel>
            <SidebarCanvasTreeList
              items={group.children}
              depth={depth + 1}
              selectedEntityIds={selectedEntityIds}
              selectedGroupId={selectedGroupId}
              isDark={isDark}
              api={api}
              section={section}
              parentId={group.id}
            />
          </Collapsible.Panel>
        </Collapsible.Root>
      </ContextMenu.Trigger>
      <Menu.Portal>
        <Menu.Positioner sideOffset={6}>
          <Menu.Popup
            className={`z-50 min-w-40 rounded-[10px] border p-1 shadow-xl outline-none ${
              isDark
                ? 'border-[var(--surface-popover-border)] bg-[var(--surface-popover-subtle)] text-zinc-100'
                : 'border-[var(--surface-popover-border)] bg-[var(--surface-popover-subtle)] text-zinc-900'
            }`}
          >
            <Menu.Item
              className={`flex cursor-default items-center gap-2 rounded-[7px] px-2.5 py-1.5 text-xs outline-none ${
                isDark
                  ? 'text-zinc-100 data-[highlighted]:bg-[var(--surface-popover)]'
                  : 'text-zinc-900 data-[highlighted]:bg-[var(--surface-popover)]'
              }`}
              onClick={startRename}
            >
              <span>Rename</span>
            </Menu.Item>
            <Menu.Item
              className={`flex cursor-default items-center gap-2 rounded-[7px] px-2.5 py-1.5 text-xs outline-none ${
                isDark
                  ? 'text-zinc-100 data-[highlighted]:bg-[var(--surface-popover)]'
                  : 'text-zinc-900 data-[highlighted]:bg-[var(--surface-popover)]'
              }`}
              onClick={() => api.ungroupGroup(group.id)}
            >
              <span>Ungroup</span>
            </Menu.Item>
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Portal>
    </ContextMenu.Root>
  )
}

function SidebarCanvasTreeItem({
  item,
  depth,
  selectedEntityIds,
  selectedGroupId,
  isDark,
  api,
  section,
  parentId,
}: {
  item: SidebarCanvasItem
  depth: number
  selectedEntityIds: string[]
  selectedGroupId: string | null
  isDark: boolean
  api: LeftSidebarElectronAPI
  section: SidebarSectionKey
  parentId: string | null
}) {
  if (item.kind === 'group') {
    return (
      <GroupTreeItem
        group={item}
        depth={depth}
        selectedEntityIds={selectedEntityIds}
        selectedGroupId={selectedGroupId}
        isDark={isDark}
        api={api}
        section={section}
        parentId={parentId}
      />
    )
  }

  const isSelected = selectedEntityIds.includes(item.id)
  if (item.kind === 'page') {
    return (
      <PageListItem
        page={item}
        active={isSelected}
        isDark={isDark}
        contentPaddingLeft={LIST_OUTER_LEFT_PADDING + LIST_ROW_INNER_X_PADDING + depth * TREE_DEPTH_STEP}
        contentPaddingRight={LIST_OUTER_RIGHT_PADDING + LIST_ROW_INNER_X_PADDING}
        onClick={() => api.revealPage(item.id)}
        onRename={(name) => api.renamePage(item.id, name)}
        onDelete={() => api.deletePage(item.id)}
      />
    )
  }

  if (item.kind === 'text') {
    return (
      <div>
        <EntityListItem
          icon={<StickyNote size={14} className="shrink-0 text-zinc-500" />}
          label={item.label}
          active={isSelected}
          isDark={isDark}
          depth={depth}
          onClick={() => api.revealEntity(item.id, 'text')}
          onRename={(name) => api.renameTextEntity(item.id, name)}
          onDelete={() => api.deleteEntity(item.id, 'text')}
        />
      </div>
    )
  }

  if (item.kind === 'drawing') {
    return (
      <div>
        <EntityListItem
          icon={<PenLine size={14} className="shrink-0 text-zinc-500" />}
          label={item.label}
          active={isSelected}
          isDark={isDark}
          depth={depth}
          onClick={() => api.revealEntity(item.id, 'drawing')}
          onRename={(name) => api.renameDrawingEntity(item.id, name)}
          onDelete={() => api.deleteEntity(item.id, 'drawing')}
          deleteLabel="Delete Drawing"
        />
      </div>
    )
  }

  if (item.kind === 'shape') {
    const ShapeIcon =
      item.shapeKind === 'ellipse' ? Circle : item.shapeKind === 'diamond' ? Diamond : Square
    return (
      <div>
        <EntityListItem
          icon={<ShapeIcon size={14} className="shrink-0 text-zinc-500" />}
          label={item.label}
          active={isSelected}
          isDark={isDark}
          depth={depth}
          onClick={() => api.revealEntity(item.id, 'shape')}
          onDelete={() => api.deleteEntity(item.id, 'shape')}
          deleteLabel="Delete Shape"
        />
      </div>
    )
  }

  const canRenameFile = RENAMABLE_FILE_PATTERN.test(item.file)
  const FileIcon = iconForFilePath(item.file)
  return (
    <div>
      <EntityListItem
        icon={<FileIcon size={14} className="shrink-0 text-zinc-500" />}
        label={item.label}
        active={isSelected}
        isDark={isDark}
        depth={depth}
        onClick={() => api.revealEntity(item.id, 'file')}
        onRename={canRenameFile ? (name) => api.renameFileEntity(item.id, name) : undefined}
        onDelete={() => api.deleteEntity(item.id, 'file')}
      />
    </div>
  )
}

function SidebarCanvasTreeList({
  items,
  depth,
  selectedEntityIds,
  selectedGroupId,
  isDark,
  api,
  section,
  parentId,
}: {
  items: SidebarCanvasItem[]
  depth: number
  selectedEntityIds: string[]
  selectedGroupId: string | null
  isDark: boolean
  api: LeftSidebarElectronAPI
  section: SidebarSectionKey
  parentId: string | null
}) {
  const drag = useDragReorder(items.length, (id, toIndex) => {
    const withoutDragged = items.filter((item) => item.id !== id)
    const anchor = withoutDragged[toIndex]
    // Sidebar renders top-of-stack first (front-to-back), but the backend
    // treats `position` as entityOrder-relative (back-to-front). Flip so
    // "visually above anchor" = "after anchor in entityOrder" = more frontward.
    api.reorderSidebarItem(section, id, anchor?.id ?? null, anchor ? 'after' : 'before', parentId)
  })

  return (
    <div {...drag.containerProps}>
      {items.map((item, index) => (
        <div key={item.id} {...drag.itemProps(item.id, index)}>
          <SidebarCanvasTreeItem
            item={item}
            depth={depth}
            selectedEntityIds={selectedEntityIds}
            selectedGroupId={selectedGroupId}
            isDark={isDark}
            api={api}
            section={section}
            parentId={parentId}
          />
        </div>
      ))}
    </div>
  )
}

export function SidebarCanvasTree(props: {
  items: SidebarCanvasItem[]
  selectedEntityIds: string[]
  selectedGroupId: string | null
  isDark: boolean
  api: LeftSidebarElectronAPI
  section: SidebarSectionKey
}) {
  return <SidebarCanvasTreeList {...props} depth={0} parentId={null} />
}
