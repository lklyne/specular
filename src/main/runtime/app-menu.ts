import { app, dialog, Menu, webContents, type WebContents } from 'electron'
import { pages, selectedPageId } from './runtime-context'
import { selectedEntityIds } from '../ui-state'
import { getComponentView } from './component-page-factory'
import { acceleratorFor } from './binding-accelerator'
import { currentKeyboardTargetPageId } from './selection-controller'
import { mainHandlers } from './binding-handlers'
import { buildBindingContext } from './binding-dispatcher'
import { checkForUpdatesManually } from '../auto-updater'
import { showOnboardingWindow } from '../onboarding-window'
import { showSettingsWindow } from '../settings-window'
import { showDebugWindow } from '../debug-window'
import {
  aboveView,
  bgView,
  cursorOverlayWindow,
  devtoolsHeaderView,
  leftSidebarView,
  toolbarView,
} from './view-refs'
import {
  bundledSkillHash,
  installedSkillHash,
  type SkillId,
} from '../skill-install'
import {
  getPerfTraceOwner,
  isPerfTraceRecording,
  setPerfTraceStateListener,
  togglePerfTrace,
} from '../perf-trace'
import { stopPanZoomPerfTest } from '../pan-zoom-perf-test'
import { getHideAgentCursors, setHideAgentCursors } from './preferences'
import { requestLayout } from './layout-engine'

const SKILL_IDS: SkillId[] = ['specular']

function pendingSkillUpdates(): number {
  let count = 0
  for (const id of SKILL_IDS) {
    const installed = installedSkillHash(id)
    const bundled = bundledSkillHash(id)
    if (installed !== null && bundled !== null && installed !== bundled) {
      count++
    }
  }
  return count
}

function setupLabel(): string {
  const pending = pendingSkillUpdates()
  if (pending === 0) return 'Setup Specular\u2026'
  if (pending === 1) return 'Setup Specular\u2026 (1 update)'
  return `Setup Specular\u2026 (${pending} updates)`
}

/**
 * The webContents an Edit-menu clipboard command acts on: the page that owns
 * the keyboard, or whatever holds OS focus when no page does.
 */
function editingTarget(): WebContents | null {
  const pageId = currentKeyboardTargetPageId()
  const page = pageId ? pages.find((candidate) => candidate.id === pageId) : null
  if (page && !page.host.webContents.isDestroyed()) return page.host.webContents
  const focused = webContents.getFocusedWebContents()
  return focused && !focused.isDestroyed() ? focused : null
}

function editingItem(
  label: string,
  accelerator: string,
  run: (wc: WebContents) => void,
): Electron.MenuItemConstructorOptions {
  return {
    label,
    accelerator,
    click: () => {
      const target = editingTarget()
      if (target) run(target)
    },
  }
}

function buildTemplate(): Electron.MenuItemConstructorOptions[] {
  const isMac = process.platform === 'darwin'

  return [
    // App menu (macOS only)
    ...(isMac
      ? [
          {
            label: app.name,
            submenu: [
              {
                label: 'About Specular',
                click: showAboutDialog,
              },
              {
                label: 'Check for Updates\u2026',
                click: () => checkForUpdatesManually(),
              },
              { type: 'separator' as const },
              {
                label: setupLabel(),
                click: () => {
                  void showOnboardingWindow('settings')
                },
              },
              {
                label: 'Settings…',
                accelerator: 'CmdOrCtrl+,',
                click: () => showSettingsWindow(),
              },
              { type: 'separator' as const },
              { role: 'services' as const },
              { type: 'separator' as const },
              { role: 'hide' as const },
              { role: 'hideOthers' as const },
              { role: 'unhide' as const },
              { type: 'separator' as const },
              { role: 'quit' as const },
            ],
          },
        ]
      : []),

    // File
    {
      label: 'File',
      submenu: [
        {
          label: 'Close Tab',
          accelerator: acceleratorFor('close-tab'),
          click: () => mainHandlers['close-tab'](buildBindingContext('canvasBg', false)),
        },
      ],
    },

    // Edit — the clipboard items dispatch by hand rather than through their
    // built-in roles, because a role acts on whatever webContents holds OS
    // focus and a page never does: aboveView holds focus on its behalf
    // (ADR 0038), so a role would cut, copy or paste against the hidden
    // keyboard sink. `editingTarget` names the page instead when one owns the
    // keyboard. Canvas entity copy/cut/paste still rides the `copy`/`cut`/
    // `paste` DOM events in the renderer, which fire either way.
    {
      label: 'Edit',
      submenu: [
        { role: 'undo' },
        { role: 'redo' },
        { type: 'separator' },
        editingItem('Cut', 'CmdOrCtrl+X', (wc) => wc.cut()),
        editingItem('Copy', 'CmdOrCtrl+C', (wc) => wc.copy()),
        editingItem('Paste', 'CmdOrCtrl+V', (wc) => wc.paste()),
        { type: 'separator' },
        editingItem('Select All', 'CmdOrCtrl+A', (wc) => wc.selectAll()),
      ],
    },

    // View — skip reload/zoom since the app manages those.
    // The built-in `toggleDevTools` role assumes a focused webContents on the
    // BrowserWindow; Specular has none (everything is a WebContentsView), so
    // it throws "Cannot read properties of undefined (reading 'toggleDevTools')".
    // We dispatch by hand to the named overlay instead.
    {
      label: 'View',
      submenu: [
        {
          label: 'Hide agent cursors',
          type: 'checkbox',
          checked: getHideAgentCursors(),
          // Rendering only — interaction sync keeps replaying clicks/hover on
          // peers, this just stops drawing the cursor glyph. Handy when a
          // screen recorder glitches on the presence-cursor overlay window.
          click: () => {
            setHideAgentCursors(!getHideAgentCursors())
            requestLayout()
            refreshAppMenu()
          },
        },
        { type: 'separator' },
        {
          label: 'Toggle DevTools (Canvas)',
          accelerator: 'CmdOrCtrl+Alt+I',
          click: () => toggleViewDevTools(bgView?.webContents),
        },
        {
          label: 'DevTools',
          submenu: [
            {
              label: 'Canvas (canvas-bg)',
              click: () => toggleViewDevTools(bgView?.webContents),
            },
            {
              label: 'Above-pages overlay (above-view)',
              click: () => toggleViewDevTools(aboveView?.webContents),
            },
            {
              label: 'Toolbar',
              click: () => toggleViewDevTools(toolbarView?.webContents),
            },
            {
              label: 'Left sidebar',
              click: () => toggleViewDevTools(leftSidebarView?.webContents),
            },
            {
              label: 'Right details panel',
              click: () => toggleViewDevTools(devtoolsHeaderView?.webContents),
            },
            {
              label: 'Agent cursor overlay',
              click: () => toggleViewDevTools(cursorOverlayWindow?.webContents),
            },
            { type: 'separator' as const },
            {
              label: 'Selected page',
              accelerator: 'CmdOrCtrl+Alt+Shift+I',
              click: () => toggleSelectedPageDevTools(),
            },
            {
              label: 'Selected component',
              click: () => toggleSelectedComponentDevTools(),
            },
          ],
        },
        ...(app.isPackaged
          ? []
          : [
              {
                label: 'Open Motion Debug Window',
                accelerator: 'CmdOrCtrl+Shift+D',
                click: () => showDebugWindow(),
              } as const,
            ]),
        { type: 'separator' },
        // All-process Chromium trace for pan/zoom jank attribution — works in
        // packaged builds (the optimized app is what's worth profiling).
        // Auto-stops after 30s; the saved file opens at ui.perfetto.dev.
        {
          label: getPerfTraceOwner() === 'pan-zoom-test'
            ? 'Stop Pan/Zoom Performance Test'
            : isPerfTraceRecording()
              ? 'Stop Performance Trace'
              : 'Record Performance Trace',
          accelerator: 'CmdOrCtrl+Alt+Shift+P',
          click: () => {
            if (getPerfTraceOwner() === 'pan-zoom-test') {
              void stopPanZoomPerfTest()
            } else {
              void togglePerfTrace()
            }
          },
        },
        { type: 'separator' },
        { role: 'togglefullscreen' },
      ],
    },

    // Window
    { role: 'windowMenu' },

    // Help
    {
      role: 'help',
      submenu: [
        // On non-mac, put About and Updates in Help menu
        ...(!isMac
          ? [
              {
                label: 'About Specular',
                click: showAboutDialog,
              },
              {
                label: 'Check for Updates\u2026',
                click: () => checkForUpdatesManually(),
              },
            ]
          : []),
      ],
    },
  ]
}

export function setupAppMenu(): void {
  // Keep the trace item's Start/Stop label live, including the auto-stop.
  setPerfTraceStateListener(refreshAppMenu)
  Menu.setApplicationMenu(Menu.buildFromTemplate(buildTemplate()))
}

/** Rebuild the application menu in place. Use after install/dismiss to
 * refresh the "(N updates)" suffix on the Setup item. */
export function refreshAppMenu(): void {
  setupAppMenu()
}

/** Toggle a detached DevTools window for the given UI overlay's webContents. */
function toggleViewDevTools(wc: WebContents | undefined): void {
  if (!wc || wc.isDestroyed()) {
    dialog.showMessageBox({
      type: 'info',
      title: 'DevTools',
      message: 'That view is not available right now.',
    })
    return
  }
  if (wc.isDevToolsOpened()) {
    wc.closeDevTools()
    return
  }
  wc.openDevTools({ mode: 'detach' })
}

function toggleSelectedPageDevTools(): void {
  const id = selectedPageId()
  const page = id ? pages.find((p) => p.id === id) : null
  if (!page) {
    dialog.showMessageBox({
      type: 'info',
      title: 'DevTools',
      message: 'Select a page first to open its DevTools.',
    })
    return
  }
  toggleViewDevTools(page.host.webContents)
}

function toggleSelectedComponentDevTools(): void {
  for (const entityId of selectedEntityIds()) {
    const cv = getComponentView(entityId)
    if (cv) {
      toggleViewDevTools(cv.view.webContents)
      return
    }
  }
  dialog.showMessageBox({
    type: 'info',
    title: 'DevTools',
    message: 'Select a component first to open its DevTools.',
  })
}

function showAboutDialog(): void {
  dialog.showMessageBox({
    type: 'info',
    title: 'About Specular',
    message: 'Specular',
    detail: [
      `Version ${app.getVersion()}`,
      '',
      'A spatial canvas for agent collaboration on the web.',
      '',
      '\u00A9 2026 Lyle Klyne',
      'Licensed under PolyForm Shield 1.0.0',
    ].join('\n'),
  })
}
