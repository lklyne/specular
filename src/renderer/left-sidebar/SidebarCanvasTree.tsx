import { useState } from 'react'
import { Collapsible } from '@base-ui/react/collapsible'
import { ContextMenu } from '@base-ui/react/context-menu'
import { Menu } from '@base-ui/react/menu'
import {
  ChevronDown,
  ChevronRight,
  Folder,
  FolderOpen,
  MessageSquare,
  PenLine,
  StickyNote,
} from 'lucide-react'
import { ShapeGlyph } from '../shared/ShapeGlyph'
import type {
  SidebarAnnotationItem,
  SidebarCanvasItem,
  SidebarGroupItem,
  SidebarPageItem,
  SidebarSectionKey,
} from '../../shared/types'
import type { LeftSidebarElectronAPI } from '../../shared/electron-api/left-sidebar'
import { iconForFilePath } from '../shared/fileIcon'
import { PageListItem } from '../shared/pageListItem'
import { InlineEditLabel } from '../shared/InlineEditLabel'
import { useDragReorder } from './useDragReorder'

const RENAMABLE_FILE_PATTERN = /\.md$/i
type SidebarSelectHandler = (
  event: React.MouseEvent<HTMLButtonElement>,
  id: string,
  kind: Exclude<SidebarCanvasItem['kind'], 'group'>,
) => void

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
  onRequestEditFocus,
  deleteLabel = 'Delete',
  depth,
  selectableId,
}: {
  icon: React.ReactNode
  label: string
  active: boolean
  isDark: boolean
  onClick: React.MouseEventHandler<HTMLButtonElement>
  onRename?: (name: string) => Promise<boolean>
  onDelete: () => void
  onRequestEditFocus?: () => void
  deleteLabel?: string
  depth: number
  selectableId: string
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

  async function commitRename(next: string) {
    if (onRename && next !== label && await onRename(next)) setIsEditing(false)
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
        onRequestFocus={onRequestEditFocus}
      />
    </div>
  ) : (
    <button
      type="button"
      className={rootClassName}
      style={rowStyle}
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

function AnnotationListItem({
  annotation,
  depth,
  isDark,
  api,
}: {
  annotation: SidebarAnnotationItem
  depth: number
  isDark: boolean
  api: LeftSidebarElectronAPI
}) {
  return (
    <button
      type="button"
      className={`flex w-full items-center gap-1 py-1.5 text-left text-xs font-normal ${
        isDark
          ? 'text-zinc-200 hover:bg-[var(--surface-interactive-hover)]'
          : 'text-zinc-800 hover:bg-[var(--surface-interactive-hover)]'
      } ${annotation.onCurrentPage ? '' : 'opacity-50'}`}
      style={{
        paddingLeft: LIST_OUTER_LEFT_PADDING + LIST_ROW_INNER_X_PADDING + depth * TREE_DEPTH_STEP,
        paddingRight: LIST_OUTER_RIGHT_PADDING + LIST_ROW_INNER_X_PADDING,
      }}
      onClick={() => api.openAnnotationThread(annotation.id)}
      title={
        annotation.onCurrentPage
          ? annotation.label
          : `${annotation.label} — page navigated away from this comment's URL`
      }
    >
      <MessageSquare size={13} className="shrink-0 text-zinc-900 dark:text-zinc-500" />
      <span className="min-w-0 flex-1 truncate">{annotation.label}</span>
      {annotation.messageCount > 1 ? (
        <span className="ml-auto shrink-0 text-xs text-zinc-400">{annotation.messageCount}</span>
      ) : null}
    </button>
  )
}

/**
 * A page row that acts as a folder for content anchored to it: anchored
 * canvas entities and annotations render as indented children behind a
 * chevron, mirroring the group tree. Anchored rows dim when the page has
 * navigated away from their anchor URL (their canvas visuals are hidden).
 */
function PageTreeItem({
  page,
  depth,
  selectedEntityIds,
  selectedGroupId,
  isDark,
  api,
  section,
  onSelect,
}: {
  page: SidebarPageItem
  depth: number
  selectedEntityIds: string[]
  selectedGroupId: string | null
  isDark: boolean
  api: LeftSidebarElectronAPI
  section: SidebarSectionKey
  onSelect: SidebarSelectHandler
}) {
  const [expanded, setExpanded] = useState(true)
  const children = page.children ?? []
  const contentPaddingLeft =
    LIST_OUTER_LEFT_PADDING + LIST_ROW_INNER_X_PADDING + depth * TREE_DEPTH_STEP
  const row = (
    <PageListItem
      page={page}
      active={selectedEntityIds.includes(page.id)}
      isDark={isDark}
      contentPaddingLeft={contentPaddingLeft}
      contentPaddingRight={LIST_OUTER_RIGHT_PADDING + LIST_ROW_INNER_X_PADDING}
      onClick={(event) => onSelect(event, page.id, 'page')}
      selectableId={page.id}
      onRename={(name) => api.renamePage(page.id, name)}
      onRequestEditFocus={() => api.setTextEditing(true)}
      onDelete={() =>
        selectedEntityIds.includes(page.id) ? api.deleteSelection() : api.deletePage(page.id)
      }
    />
  )
  if (children.length === 0) return row

  return (
    <Collapsible.Root open={expanded} onOpenChange={setExpanded}>
      <div className="relative">
        {row}
        <Collapsible.Trigger
          className="absolute top-1/2 flex -translate-y-1/2 items-center justify-center text-zinc-900 dark:text-zinc-500"
          style={{ left: contentPaddingLeft - 16 }}
          onClick={(event) => event.stopPropagation()}
        >
          {expanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
        </Collapsible.Trigger>
      </div>
      <Collapsible.Panel>
        {children.map((item) =>
          item.kind === 'annotation' ? (
            <AnnotationListItem
              key={item.id}
              annotation={item}
              depth={depth + 1}
              isDark={isDark}
              api={api}
            />
          ) : (
            <div
              key={item.id}
              className={item.onCurrentPage ? undefined : 'opacity-50'}
              title={item.onCurrentPage ? undefined : 'Page navigated away from this item’s URL'}
            >
              <SidebarCanvasTreeItem
                item={item}
                depth={depth + 1}
                selectedEntityIds={selectedEntityIds}
                selectedGroupId={selectedGroupId}
                isDark={isDark}
                api={api}
                section={section}
                onSelect={onSelect}
              />
            </div>
          ),
        )}
      </Collapsible.Panel>
    </Collapsible.Root>
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
  onSelect,
}: {
  group: SidebarGroupItem
  depth: number
  selectedEntityIds: string[]
  selectedGroupId: string | null
  isDark: boolean
  api: LeftSidebarElectronAPI
  section: SidebarSectionKey
  onSelect: SidebarSelectHandler
}) {
  const [expanded, setExpanded] = useState(false)
  const [isEditing, setIsEditing] = useState(false)
  const isSelected = selectedGroupId === group.id

  function startRename() {
    setIsEditing(true)
  }

  async function commitRename(next: string) {
    if (next !== group.label && await api.renameGroup(group.id, next)) setIsEditing(false)
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
                  <FolderOpen size={14} className="shrink-0 text-zinc-900 dark:text-zinc-500" />
                ) : (
                  <Folder size={14} className="shrink-0 text-zinc-900 dark:text-zinc-500" />
                )}
                <InlineEditLabel
                  value={group.label}
                  isEditing
                  onCommit={commitRename}
                  onCancel={() => setIsEditing(false)}
                  variant="sidebar-row"
                  isDark={isDark}
                  onRequestFocus={() => api.setTextEditing(true)}
                />
                <span className="ml-auto shrink-0 text-xs text-zinc-400">{group.entityCount}</span>
              </div>
            ) : (
              <button
                type="button"
                className={rowClassName}
                style={rowStyle}
                onClick={() => api.revealGroup(group.id)}
                onPointerDown={(event) => {
                  if (event.detail !== 2) return
                  event.preventDefault()
                  event.stopPropagation()
                  startRename()
                }}
                onDoubleClick={startRename}
                title={group.label}
              >
                {expanded ? (
                  <FolderOpen size={14} className="shrink-0 text-zinc-900 dark:text-zinc-500" />
                ) : (
                  <Folder size={14} className="shrink-0 text-zinc-900 dark:text-zinc-500" />
                )}
                <span className="min-w-0 truncate">{group.label}</span>
                <span className="ml-auto shrink-0 text-xs text-zinc-400">{group.entityCount}</span>
              </button>
            )}
            <Collapsible.Trigger
              className="absolute top-1/2 flex -translate-y-1/2 items-center justify-center text-zinc-900 dark:text-zinc-500"
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
              onSelect={onSelect}
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
  onSelect,
}: {
  item: SidebarCanvasItem
  depth: number
  selectedEntityIds: string[]
  selectedGroupId: string | null
  isDark: boolean
  api: LeftSidebarElectronAPI
  section: SidebarSectionKey
  onSelect: SidebarSelectHandler
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
        onSelect={onSelect}
      />
    )
  }

  const isSelected = selectedEntityIds.includes(item.id)
  if (item.kind === 'page') {
    return (
      <PageTreeItem
        page={item}
        depth={depth}
        selectedEntityIds={selectedEntityIds}
        selectedGroupId={selectedGroupId}
        isDark={isDark}
        api={api}
        section={section}
        onSelect={onSelect}
      />
    )
  }

  if (item.kind === 'text') {
    return (
      <div>
        <EntityListItem
          icon={<StickyNote size={14} className="shrink-0 text-zinc-900 dark:text-zinc-500" />}
          label={item.label}
          active={isSelected}
          isDark={isDark}
          depth={depth}
          onClick={(event) => onSelect(event, item.id, 'text')}
          selectableId={item.id}
          onRename={(name) => api.renameTextEntity(item.id, name)}
          onRequestEditFocus={() => api.setTextEditing(true)}
          onDelete={() =>
            isSelected ? api.deleteSelection() : api.deleteEntity(item.id, 'text')
          }
        />
      </div>
    )
  }

  if (item.kind === 'drawing') {
    return (
      <div>
        <EntityListItem
          icon={<PenLine size={14} className="shrink-0 text-zinc-900 dark:text-zinc-500" />}
          label={item.label}
          active={isSelected}
          isDark={isDark}
          depth={depth}
          onClick={(event) => onSelect(event, item.id, 'drawing')}
          selectableId={item.id}
          onRename={(name) => api.renameDrawingEntity(item.id, name)}
          onRequestEditFocus={() => api.setTextEditing(true)}
          onDelete={() =>
            isSelected ? api.deleteSelection() : api.deleteEntity(item.id, 'drawing')
          }
          deleteLabel="Delete Drawing"
        />
      </div>
    )
  }

  if (item.kind === 'shape') {
    return (
      <div>
        <EntityListItem
          icon={<span className="shrink-0 text-zinc-900 dark:text-zinc-500"><ShapeGlyph kind={item.shapeKind} size={14} /></span>}
          label={item.label}
          active={isSelected}
          isDark={isDark}
          depth={depth}
          onClick={(event) => onSelect(event, item.id, 'shape')}
          selectableId={item.id}
          onDelete={() =>
            isSelected ? api.deleteSelection() : api.deleteEntity(item.id, 'shape')
          }
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
        icon={<FileIcon size={14} className="shrink-0 text-zinc-900 dark:text-zinc-500" />}
        label={item.label}
        active={isSelected}
        isDark={isDark}
        depth={depth}
        onClick={(event) => onSelect(event, item.id, 'file')}
        selectableId={item.id}
        onRename={canRenameFile ? (name) => api.renameFileEntity(item.id, name) : undefined}
        onRequestEditFocus={() => api.setTextEditing(true)}
        onDelete={() =>
          isSelected ? api.deleteSelection() : api.deleteEntity(item.id, 'file')
        }
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
  onSelect,
}: {
  items: SidebarCanvasItem[]
  depth: number
  selectedEntityIds: string[]
  selectedGroupId: string | null
  isDark: boolean
  api: LeftSidebarElectronAPI
  section: SidebarSectionKey
  parentId: string | null
  onSelect: SidebarSelectHandler
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
            onSelect={onSelect}
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
  onSelect: SidebarSelectHandler
}) {
  return <SidebarCanvasTreeList {...props} depth={0} parentId={null} />
}
