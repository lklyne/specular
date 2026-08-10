import { useEffect } from 'react'
import type { ThemeData } from '../../shared/types'
import { isPlainShortcutKey } from '../../shared/gesture-utils'
import { useReportTextEditing } from '../shared/hooks/useReportTextEditing'
import { useTheme } from '../shared/hooks/useTheme'
import { TooltipProvider } from '../shared/Tooltip'
import { toolbarApi } from './toolbarApi'
import {
  CenterActions,
  LeftActions,
  RightPanelToggle,
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
    shapeColor,
    currentPresetValue,
    hasSelection,
    agentCursors,
  } = useToolbarState()

  const { isDark, themeMode } = useTheme(initialTheme, toolbarApi.onThemeChanged)

  useReportTextEditing(toolbarApi.setTextEditing)

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!document.hasFocus()) return
      if (
        isPlainShortcutKey(event, 'escape') &&
        activeTool.kind !== 'select'
      ) {
        event.preventDefault()
        if (document.activeElement instanceof HTMLElement) {
          document.activeElement.blur()
        }
        toolbarApi.setTool({ kind: 'select' })
        return
      }

      if (event.key.toLowerCase() !== 'r' || !event.shiftKey) return
      if (!event.metaKey && !event.ctrlKey) return
      event.preventDefault()
      toolbarApi.reloadApp()
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [activeTool])
  const isMac = navigator.userAgent.includes('Mac')

  return (
    <>
      <style>{`
        html, body, #root {
          background: transparent !important;
          margin: 0;
          padding: 0;
          overflow: visible !important;
          scrollbar-width: none;
        }
        /* The toolbar WebContentsView grows a band below the 44px strip to paint
           tooltips; never show its scrollbar when content overflows the view. */
        ::-webkit-scrollbar { display: none; }
        /* Bridge the horizontal gaps between buttons so the tooltip fires
           continuously. The transparent ::before extends each button's hover
           hit-area into the gap without moving the glyph or growing the visible
           hover fill (which stays on the real button box). */
        .tb-hit { position: relative; }
        .tb-hit::before {
          content: '';
          position: absolute;
          top: 0;
          bottom: 0;
          left: -4px;
          right: -4px;
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
      <TooltipProvider>
      <div
        className={`toolbar-bar fixed top-0 left-0 right-0 grid h-[44px] grid-cols-[1fr_auto_1fr] items-center gap-1 ${
          isMac ? 'pl-[86px] pr-4' : 'px-4'
        } select-none [-webkit-app-region:drag] border-b border-[var(--surface-toolbar-border)] bg-[var(--surface-toolbar)] text-[var(--surface-toolbar-foreground)]`}
      >
        {/* Bridge calls are wrapped in arrows so the React click event never
            crosses the contextBridge: serializing it walks the whole window
            object, and the first touch of window.speechSynthesis blocks the
            browser process ~800ms enumerating macOS voices. */}
        <LeftActions
          isDark={isDark}
          leftSidebarOpen={leftSidebarOpen}
          onToggleLeftSidebar={() => toolbarApi.toggleLeftSidebar()}
        />

        <div className="flex items-center justify-center">
          <div className="flex min-w-0 max-w-full items-center gap-2 [-webkit-app-region:no-drag]">
            <CenterActions
              isDark={isDark}
              activeTool={activeTool}
              drawBrushType={drawBrushType}
              drawColor={drawColor}
              stickyColor={stickyColor}
              shapeColor={shapeColor}
              hasSelection={hasSelection}
              zoomPercent={zoomPercent}
              currentPresetValue={currentPresetValue}
              themeMode={themeMode}
              onSetTool={(tool) => toolbarApi.setTool(tool)}
              onDropdownOpenChange={(open) => {
                if (open) {
                  toolbarApi.dropdownOpen()
                  // A tool popup (e.g. add-page) lives in the above-view overlay
                  // and can't see this dropdown; disarm the tool so the two
                  // toolbar popups can't sit open at once.
                  if (activeTool.kind !== 'select') toolbarApi.setTool({ kind: 'select' })
                } else toolbarApi.dropdownClose()
              }}
              onThemeModeSelect={(mode) => toolbarApi.setThemeMode(mode)}
              onZoomSet={(value) => toolbarApi.zoomSet(value / 100)}
            />
          </div>
        </div>

        <div className="flex min-w-0 items-center justify-end gap-1">
          <ToolbarStatusActions
            isDark={isDark}
            agentCursors={agentCursors}
          />
          <RightPanelToggle
            isDark={isDark}
            devtoolsOpen={devtoolsOpen}
            onToggleDevTools={() => toolbarApi.toggleDevTools()}
          />
        </div>
      </div>
      </TooltipProvider>
    </>
  )
}
