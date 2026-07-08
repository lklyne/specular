import { useEffect, useRef, useState } from 'react'
import { ContextMenu } from '@base-ui/react/context-menu'
import { Menu } from '@base-ui/react/menu'
import { Check, ChevronDown, ChevronRight, File, Plus } from 'lucide-react'
import type { LeftSidebarData, SidebarCanvasItem, ThemeData } from '../../shared/types'
import type { LeftSidebarElectronAPI } from '../../shared/electron-api/left-sidebar'
import { InlineEditLabel } from '../shared/InlineEditLabel'
import { SidebarCanvasTree } from './SidebarCanvasTree'
import { useReportTextEditing } from '../shared/hooks/useReportTextEditing'
import { useTheme } from '../shared/hooks/useTheme'
import { useDragReorder } from './useDragReorder'

const LIST_OUTER_LEFT_PADDING = 14
const LIST_OUTER_RIGHT_PADDING = 8
const LIST_ROW_INNER_X_PADDING = 8

const api = (window as unknown as { electronAPI: LeftSidebarElectronAPI }).electronAPI

function findSidebarItemById(items: SidebarCanvasItem[], targetId: string): SidebarCanvasItem | null {
  for (const item of items) {
    if (item.id === targetId) return item
    if (item.kind !== 'group') continue
    const childMatch = findSidebarItemById(item.children, targetId)
    if (childMatch) return childMatch
  }
  return null
}

function allSidebarItems(data: LeftSidebarData): SidebarCanvasItem[] {
  if (data.sections) return [...data.sections.notes, ...data.sections.pages]
  return data.items
}

export default function App({
  initialSidebarData,
  initialTheme,
}: {
  initialSidebarData: LeftSidebarData
  initialTheme: ThemeData
}) {
  const [sidebarData, setSidebarData] = useState<LeftSidebarData>(initialSidebarData)
  const [pagesExpanded, setPagesExpanded] = useState(true)
  const [notesExpanded, setNotesExpanded] = useState(true)
  const [pagesSectionExpanded, setPagesSectionExpanded] = useState(true)
  const [editingTabId, setEditingTabId] = useState<string | null>(null)
  const previousActivePageCountRef = useRef<number | null>(null)
  const { isDark } = useTheme(initialTheme, api.onThemeChanged)
  useReportTextEditing(api.setTextEditing)

  const drag = useDragReorder(sidebarData.tabs.length, (tabId, toIndex) =>
    api.reorderTab(tabId, toIndex),
  )

  useEffect(() => api.onSidebarData((data) => setSidebarData(data)), [])

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!document.hasFocus()) return
      const target = event.target
      if (
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        (target instanceof HTMLElement && target.isContentEditable)
      ) {
        return
      }

      if (event.key !== 'Delete' && event.key !== 'Backspace') return
      if (!sidebarData.selectedEntityIds.length) return

      let deletedAny = false
      const items = allSidebarItems(sidebarData)
      for (const entityId of sidebarData.selectedEntityIds) {
        const item = findSidebarItemById(items, entityId)
        if (!item || item.kind === 'group') continue
        if (item.kind === 'page') {
          api.deletePage(item.id)
        } else {
          api.deleteEntity(item.id, item.kind)
        }
        deletedAny = true
      }

      if (!deletedAny) return
      event.preventDefault()
    }

    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [sidebarData, sidebarData.selectedEntityIds])

  useEffect(() => {
    if (!editingTabId) return
    if (!sidebarData.tabs.some((tab) => tab.id === editingTabId)) {
      setEditingTabId(null)
    }
  }, [editingTabId, sidebarData.tabs])

  const activeTab = sidebarData.tabs.find((tab) => tab.id === sidebarData.activeTabId) ?? null
  const pagesHeaderLabel = pagesExpanded ? 'Workspaces' : activeTab?.name ?? 'Workspaces'

  useEffect(() => {
    const nextCount = activeTab?.pages.length ?? 0
    const previousCount = previousActivePageCountRef.current
    if (previousCount !== null && nextCount > previousCount) {
      setPagesExpanded(true)
    }
    previousActivePageCountRef.current = nextCount
  }, [activeTab?.id, activeTab?.pages.length])

  function startRenameTab(tabId: string) {
    setEditingTabId(tabId)
  }

  function cancelRenameTab() {
    setEditingTabId(null)
  }

  function commitRenameTab(tabId: string, currentName: string, nextName: string) {
    if (nextName && nextName !== currentName) api.renameTab(tabId, nextName)
    cancelRenameTab()
  }

  return (
    <aside
      className={`flex h-screen w-screen flex-col overflow-hidden ${
        isDark
          ? 'border-r border-[var(--surface-chrome-border)] bg-[var(--surface-panel)] text-zinc-100'
          : 'border-r border-[var(--surface-chrome-border)] bg-[var(--surface-panel)] text-zinc-900'
      }`}
    >
      <div
        className={
          pagesExpanded
            ? 'flex h-9 items-center px-3'
            : 'flex h-9 items-center border-b border-[var(--surface-panel-border)] px-3'
        }
      >
        <div className="flex w-full items-center gap-1">
          <button
            type="button"
            className="flex min-w-0 flex-1 items-center gap-1.5 text-left"
            onClick={() => setPagesExpanded((value) => !value)}
            title={pagesHeaderLabel}
          >
            <span className="truncate text-[12px] font-medium">{pagesHeaderLabel}</span>
            {pagesExpanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
          </button>
          <button
            type="button"
            className={`rounded-[8px] border border-transparent p-1.5 ${
              isDark
                ? 'bg-transparent text-zinc-300 hover:bg-zinc-700/70 hover:text-zinc-100'
                : 'bg-transparent text-zinc-600 hover:bg-zinc-100 hover:text-zinc-900 active:bg-zinc-200'
            }`}
            onClick={() => api.createTab()}
            title="Add workspace"
          >
            <Plus size={14} />
          </button>
        </div>
      </div>

        <div className="thin-scrollbar min-h-0 flex-1 overflow-auto">
        {pagesExpanded ? (
          <div className="pt-0.5 pb-2" {...drag.containerProps}>
            {sidebarData.tabs.map((tab, tabIndex) => (
              <div
                key={tab.id}
                {...drag.itemProps(tab.id, tabIndex, editingTabId === tab.id)}
              >
                <ContextMenu.Root>
                  <ContextMenu.Trigger className="block w-full">
                    {editingTabId === tab.id ? (
                      <div
                        className={`flex w-full items-center gap-1 py-1.5 text-xs font-normal ${
                          isDark
                            ? 'text-zinc-100 hover:bg-[var(--surface-interactive-hover)]'
                            : 'text-zinc-900 hover:bg-[var(--surface-interactive-hover)]'
                        } ${tab.isActive ? '' : isDark ? 'text-zinc-200' : 'text-zinc-800'}`}
                        style={{
                          paddingLeft: LIST_OUTER_LEFT_PADDING + LIST_ROW_INNER_X_PADDING,
                          paddingRight: LIST_OUTER_RIGHT_PADDING + LIST_ROW_INNER_X_PADDING,
                        }}
                      >
                        <File size={14} className="shrink-0 text-zinc-500" />
                        <InlineEditLabel
                          value={tab.name}
                          isEditing
                          onCommit={(nextName) => commitRenameTab(tab.id, tab.name, nextName)}
                          onCancel={cancelRenameTab}
                          variant="sidebar-row"
                          isDark={isDark}
                        />
                        {tab.isActive ? <Check size={14} className="ml-auto shrink-0" /> : null}
                      </div>
                    ) : (
                      <button
                        type="button"
                        className={`flex w-full items-center gap-1 py-1.5 text-left text-xs font-normal ${
                          isDark
                            ? 'text-zinc-100 hover:bg-[var(--surface-interactive-hover)]'
                            : 'text-zinc-900 hover:bg-[var(--surface-interactive-hover)]'
                        } ${tab.isActive ? '' : isDark ? 'text-zinc-200' : 'text-zinc-800'}`}
                        style={{
                          paddingLeft: LIST_OUTER_LEFT_PADDING + LIST_ROW_INNER_X_PADDING,
                          paddingRight: LIST_OUTER_RIGHT_PADDING + LIST_ROW_INNER_X_PADDING,
                        }}
                        onClick={() => api.selectTab(tab.id)}
                        onDoubleClick={() => startRenameTab(tab.id)}
                        title={tab.name}
                      >
                        <File size={14} className="shrink-0 text-zinc-500" />
                        <span className="truncate">{tab.name}</span>
                        {tab.isActive ? <Check size={14} className="ml-auto shrink-0" /> : null}
                      </button>
                    )}
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
                          onClick={() => startRenameTab(tab.id)}
                        >
                          <span>Rename workspace</span>
                        </Menu.Item>
                        <Menu.Item
                          className={`flex cursor-default items-center gap-2 rounded-[7px] px-2.5 py-1.5 text-xs outline-none ${
                            isDark
                              ? 'text-zinc-100 data-[highlighted]:bg-[var(--surface-popover)]'
                              : 'text-zinc-900 data-[highlighted]:bg-[var(--surface-popover)]'
                          }`}
                          onClick={() => api.deleteTab(tab.id)}
                        >
                          <span>Delete workspace</span>
                        </Menu.Item>
                      </Menu.Popup>
                    </Menu.Positioner>
                  </Menu.Portal>
                </ContextMenu.Root>
              </div>
            ))}
          </div>
        ) : null}

        <div className={isDark ? 'border-t border-zinc-700/50' : 'border-t border-gray-200/80'} />

        <div className="py-2">
          <div>
            <div className="flex h-9 items-center px-3">
              <button
                type="button"
                className="flex min-w-0 flex-1 items-center gap-1.5 text-left"
                onClick={() => setNotesExpanded((value) => !value)}
                title="Notes"
              >
                <span className="truncate text-[12px] font-medium">Notes</span>
                {notesExpanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
              </button>
            </div>
            {notesExpanded ? (
              <SidebarCanvasTree
                items={sidebarData.sections.notes}
                selectedEntityIds={sidebarData.selectedEntityIds}
                selectedGroupId={sidebarData.selectedGroupId ?? null}
                isDark={isDark}
                api={api}
                section="notes"
              />
            ) : null}
          </div>

          <div>
            <div className="flex h-9 items-center px-3">
              <button
                type="button"
                className="flex min-w-0 flex-1 items-center gap-1.5 text-left"
                onClick={() => setPagesSectionExpanded((value) => !value)}
                title="Pages"
              >
                <span className="truncate text-[12px] font-medium">Pages</span>
                {pagesSectionExpanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
              </button>
            </div>
            {pagesSectionExpanded ? (
              <SidebarCanvasTree
                items={sidebarData.sections.pages}
                selectedEntityIds={sidebarData.selectedEntityIds}
                selectedGroupId={sidebarData.selectedGroupId ?? null}
                isDark={isDark}
                api={api}
                section="pages"
              />
            ) : null}
          </div>

          {!sidebarData.items.length ? (
            <div
              className="py-1 text-[11px] text-zinc-500"
              style={{
                paddingLeft: LIST_OUTER_LEFT_PADDING + LIST_ROW_INNER_X_PADDING,
                paddingRight: LIST_OUTER_RIGHT_PADDING + LIST_ROW_INNER_X_PADDING,
              }}
            >
              No items
            </div>
          ) : null}
        </div>
      </div>
    </aside>
  )
}
