import { useEffect, useRef, useState } from 'react'
import { ContextMenu } from '@base-ui/react/context-menu'
import { Menu } from '@base-ui/react/menu'
import { Check, ChevronDown, ChevronRight, File, Plus } from 'lucide-react'
import type { CanvasEntityKind, LeftSidebarData, ThemeData } from '../../shared/types'
import type { LeftSidebarElectronAPI } from '../../shared/electron-api/left-sidebar'
import { InlineEditLabel } from '../shared/InlineEditLabel'
import { SidebarCanvasTree } from './SidebarCanvasTree'
import { sidebarSelectionIntent } from './sidebar-selection'
import { useReportTextEditing } from '../shared/hooks/useReportTextEditing'
import { useTheme } from '../shared/hooks/useTheme'
import { useDragReorder } from './useDragReorder'

const LIST_OUTER_LEFT_PADDING = 14
const LIST_OUTER_RIGHT_PADDING = 8
const LIST_ROW_INNER_X_PADDING = 8

const api = (window as unknown as { electronAPI: LeftSidebarElectronAPI }).electronAPI

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
  const lastClickedEntityIdRef = useRef<string | null>(null)
  const { isDark } = useTheme(initialTheme, api.onThemeChanged)
  useReportTextEditing(api.setTextEditing)

  const drag = useDragReorder(sidebarData.tabs.length, (tabId, toIndex) =>
    api.reorderTab(tabId, toIndex),
  )

  useEffect(() => api.onSidebarData((data) => setSidebarData(data)), [])

  useEffect(() => {
    if (!editingTabId) return
    if (!sidebarData.tabs.some((tab) => tab.id === editingTabId)) {
      setEditingTabId(null)
    }
  }, [editingTabId, sidebarData.tabs])

  const activeTab = sidebarData.tabs.find((tab) => tab.id === sidebarData.activeTabId) ?? null
  const canvasesHeaderLabel = pagesExpanded ? 'Canvases' : activeTab?.name ?? 'Canvases'

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

  async function commitRenameTab(tabId: string, currentName: string, nextName: string) {
    if (nextName !== currentName && await api.renameTab(tabId, nextName)) cancelRenameTab()
  }

  function handleSidebarSelect(
    event: React.MouseEvent<HTMLButtonElement>,
    id: string,
    kind: Exclude<CanvasEntityKind, 'group' | 'edge'>,
  ) {
    const orderedVisibleIds = Array.from(
      document.querySelectorAll<HTMLElement>('[data-sidebar-selectable-id]'),
      (element) => element.dataset.sidebarSelectableId,
    ).filter((candidate): candidate is string => Boolean(candidate))
    const intent = sidebarSelectionIntent({
      clickedId: id,
      orderedVisibleIds,
      lastClickedId: lastClickedEntityIdRef.current,
      shiftKey: event.shiftKey,
      toggleKey: event.metaKey || event.ctrlKey,
    })
    lastClickedEntityIdRef.current = intent.nextAnchorId
    if (kind === 'page') {
      api.revealPage(id, intent.ids, intent.mode)
    } else {
      api.revealEntity(id, kind, intent.ids, intent.mode)
    }
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
            title={canvasesHeaderLabel}
          >
            <span className="truncate text-[12px] font-medium">{canvasesHeaderLabel}</span>
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
            title="Add canvas"
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
                          onRequestFocus={() => api.setTextEditing(true)}
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
                        onPointerDown={(event) => {
                          if (event.detail !== 2) return
                          event.preventDefault()
                          event.stopPropagation()
                          startRenameTab(tab.id)
                        }}
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
                          <span>Rename canvas</span>
                        </Menu.Item>
                        <Menu.Item
                          className={`flex cursor-default items-center gap-2 rounded-[7px] px-2.5 py-1.5 text-xs outline-none ${
                            isDark
                              ? 'text-zinc-100 data-[highlighted]:bg-[var(--surface-popover)]'
                              : 'text-zinc-900 data-[highlighted]:bg-[var(--surface-popover)]'
                          }`}
                          onClick={() => api.deleteTab(tab.id)}
                        >
                          <span>Delete canvas</span>
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
                onSelect={handleSidebarSelect}
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
                onSelect={handleSidebarSelect}
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
