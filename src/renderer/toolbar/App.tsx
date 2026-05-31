import { useEffect } from 'react'
import type { ThemeData } from '../../shared/types'
import { DRAWING_FEATURE_ENABLED } from '../../shared/featureFlags'
import { isPlainShortcutKey } from '../../shared/gesture-utils'
import { useReportTextEditing } from '../shared/hooks/useReportTextEditing'
import { useTheme } from '../shared/hooks/useTheme'
import { toolbarApi } from './toolbarApi'
import {
  CenterAddressBar,
  CenterActions,
  LeftActions,
  RightPanelToggle,
  ToolbarDivider,
  ToolbarStatusActions,
} from './toolbarSections'
import { useToolbarState } from './useToolbarState'

export default function App({ initialTheme }: { initialTheme: ThemeData }) {
  const {
    zoomPercent,
    leftSidebarOpen,
    devtoolsOpen,
    activeTool,
    drawBrushType,
    drawColor,
    stickyColor,
    selection,
    addressValue,
    setAddressValue,
    addressBarRef,
    currentPresetValue,
    hasSelection,
    hasPages,
    isBrowserMode,
    agentCursors,
  } = useToolbarState()

  const isDark = useTheme(initialTheme, toolbarApi.onThemeChanged)

  useReportTextEditing(toolbarApi.setTextEditing)

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!document.hasFocus()) return
      if (
        isPlainShortcutKey(event, 'escape') &&
        (activeTool.kind !== 'select' || selection.viewMode === 'browser')
      ) {
        event.preventDefault()
        if (document.activeElement instanceof HTMLElement) {
          document.activeElement.blur()
        }
        if (activeTool.kind !== 'select') {
          toolbarApi.setTool({ kind: 'select' })
        }
        if (selection.viewMode === 'browser') {
          toolbarApi.toggleBrowserMode()
        }
        return
      }

      if (event.key.toLowerCase() !== 'r' || !event.shiftKey) return
      if (!event.metaKey && !event.ctrlKey) return
      event.preventDefault()
      toolbarApi.reloadApp()
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [activeTool, selection.viewMode])
  const isMac = navigator.userAgent.includes('Mac')
  const showMultiPageAddressBar = selection.selectionCount > 1
  const showTabsModeAddressBar = isBrowserMode && hasSelection
  const showCenterActionsOnly = !showMultiPageAddressBar && !showTabsModeAddressBar

  return (
    <>
      <style>{`
        html, body, #root {
          background: transparent !important;
          margin: 0;
          padding: 0;
          overflow: visible !important;
        }
        html:not(.dark) .toolbar-bar {
          background: var(--surface-toolbar);
          color: var(--surface-toolbar-foreground);
          border-bottom-color: var(--surface-toolbar-border);
        }
        html.dark .toolbar-bar {
          background: var(--surface-toolbar);
          color: var(--surface-toolbar-foreground);
          border-bottom-color: var(--surface-toolbar-border);
        }
        html:not(.dark) [data-popup-open] {
          background: var(--surface-popover);
          border-color: var(--surface-popover-border);
          color: var(--surface-toolbar-foreground);
        }
        html.dark [data-popup-open] {
          background: color-mix(in srgb, var(--surface-popover) 70%, transparent);
          border-color: transparent;
          color: #f4f4f5;
        }
        .nav-squircle-btn {
          border-radius: 16px;
          -electron-corner-smoothing: system-ui;
        }
        .toolbar-squircle-btn {
          border-radius: 8px;
          -electron-corner-smoothing: system-ui;
        }
      `}</style>

      {/* NOTE: padding values (`pl-[86px] pr-4` mac, `px-4` other) are mirrored
          in `runtime-constants.ts` (TOOLBAR_PAD_*) so main can compute the
          tool-center x for popup alignment. Keep in sync. */}
      <div
        className={`toolbar-bar fixed top-0 left-0 right-0 grid h-[44px] ${
          showCenterActionsOnly ? 'grid-cols-[1fr_auto_1fr]' : 'grid-cols-[auto_1fr_auto]'
        } items-center gap-1 ${
          isMac ? 'pl-[86px] pr-4' : 'px-4'
        } select-none [-webkit-app-region:drag] border-b border-[var(--surface-toolbar-border)] bg-[var(--surface-toolbar)] text-[var(--surface-toolbar-foreground)]`}
      >
        <LeftActions
          isDark={isDark}
          leftSidebarOpen={leftSidebarOpen}
          onToggleLeftSidebar={toolbarApi.toggleLeftSidebar}
        />

        {showTabsModeAddressBar ? (
          <div className="flex min-w-0 flex-1 items-center gap-3">
            <ToolbarDivider isDark={isDark} />
            <div className="min-w-0 flex-1">
              <CenterAddressBar
                isDark={isDark}
                hasSelection={hasSelection}
                selection={selection}
                addressValue={addressValue}
                setAddressValue={setAddressValue}
                addressBarRef={addressBarRef}
                align="left"
                onGoBackSelection={toolbarApi.goBackSelection}
                onGoForwardSelection={toolbarApi.goForwardSelection}
                onReloadSelection={toolbarApi.reloadSelection}
                onNavigateSelection={toolbarApi.navigateSelection}
              />
            </div>
            <ToolbarDivider isDark={isDark} />
            <div className="flex shrink-0 items-center gap-2 [-webkit-app-region:no-drag]">
              <CenterActions
                isDark={isDark}
                isBrowserMode={isBrowserMode}
                activeTool={activeTool}
                drawBrushType={drawBrushType}
                drawColor={drawColor}
                stickyColor={stickyColor}
                drawingEnabled={DRAWING_FEATURE_ENABLED}
                hasSelection={hasSelection}
                zoomPercent={zoomPercent}
                currentPresetValue={currentPresetValue}
                onSetTool={toolbarApi.setTool}
                onDropdownOpenChange={(open) => {
                  if (open) toolbarApi.dropdownOpen()
                  else toolbarApi.dropdownClose()
                }}
                onToggleTheme={toolbarApi.toggleTheme}
                onZoomSet={(value) => toolbarApi.zoomSet(value / 100)}
              />
            </div>
          </div>
        ) : showMultiPageAddressBar ? (
          <div className="flex min-w-0 flex-1 items-center gap-3">
            <ToolbarDivider isDark={isDark} />
            <div className="min-w-0 flex-1">
              <CenterAddressBar
                isDark={isDark}
                hasSelection={hasSelection}
                selection={selection}
                addressValue={addressValue}
                setAddressValue={setAddressValue}
                onGoBackSelection={toolbarApi.goBackSelection}
                onGoForwardSelection={toolbarApi.goForwardSelection}
                onReloadSelection={toolbarApi.reloadSelection}
                onNavigateSelection={toolbarApi.navigateSelection}
              />
            </div>
          </div>
        ) : showCenterActionsOnly ? (
          <div className="flex items-center justify-center">
            <div className="flex min-w-0 max-w-full items-center gap-2 [-webkit-app-region:no-drag]">
              <CenterActions
                isDark={isDark}
                isBrowserMode={isBrowserMode}
                activeTool={activeTool}
                drawBrushType={drawBrushType}
                drawColor={drawColor}
                stickyColor={stickyColor}
                drawingEnabled={DRAWING_FEATURE_ENABLED}
                hasSelection={hasSelection}
                zoomPercent={zoomPercent}
                currentPresetValue={currentPresetValue}
                onSetTool={toolbarApi.setTool}
                onDropdownOpenChange={(open) => {
                  if (open) toolbarApi.dropdownOpen()
                  else toolbarApi.dropdownClose()
                }}
                onToggleTheme={toolbarApi.toggleTheme}
                onZoomSet={(value) => toolbarApi.zoomSet(value / 100)}
              />
            </div>
          </div>
        ) : null}

        <div className="flex min-w-0 items-center justify-end gap-1">
          <ToolbarStatusActions
            isDark={isDark}
            agentCursors={agentCursors}
          />
          {!showCenterActionsOnly && <ToolbarDivider isDark={isDark} />}
          <RightPanelToggle
            isDark={isDark}
            devtoolsOpen={devtoolsOpen}
            isBrowserMode={isBrowserMode}
            hasPages={hasPages}
            onToggleDevTools={toolbarApi.toggleDevTools}
            onToggleBrowserMode={toolbarApi.toggleBrowserMode}
          />
        </div>
      </div>
    </>
  )
}
