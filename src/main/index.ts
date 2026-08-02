import { app, crashReporter, dialog, net, nativeTheme, protocol } from 'electron'
import { basename } from 'path'
import { DEFAULT_PAGES, DEFAULT_REMOTE_DEBUGGING_PORT } from '../shared/constants'
import { logCrash } from './crash-log'
import {
  flushSpaceAutosaveSync,
  loadSpace,
} from './runtime/space-autosave'
import { restorePersistedSpace } from './runtime/space-restore'
import { createPage, pages, setMcpConnectionStatus } from './runtime/page-runtime'
import { setOpenLinkInNewFrameHandler } from './runtime/link-open-policy'
import { duplicatePageFromSource } from './workspace-pages'
import { requestLayout } from './runtime/viewport-control'
import { toggleDevTools } from './runtime/ui-actions'
import { broadcastTheme, initWindow, isDark, win } from './runtime/window-shell'
import {
  getMcpConnectionStatus,
  onMcpConnectionStatusChanged,
  onPresenceCursorsChanged,
  startAppControlServer,
  stopAppControlServer,
} from './app-control-server'
import { markDirty } from './runtime/layout-dirty'
import { registerIpcHandlers } from './ipc-handlers'
import { refreshAppMenu, setupAppMenu } from './runtime/app-menu'
import { getSpacePath, loadOnboardingState, saveOnboardingState, setSpacePath } from './runtime/preferences'
import { isSpaceAvailable } from './runtime/space-dir'
import { showOnboardingWindow, focusOnboardingWindow, isOnboardingWindowOpen } from './onboarding-window'
import { focusSettingsWindow, isSettingsWindowOpen } from './settings-window'
import { configureBundledAgentBrowser, hasUserOwnedAgentBrowserBinary } from './agent-browser-install'
import { autoUpdateSkillsIfSafe } from './skill-auto-update'
import { runAgentBrowserSkillRemovalMigration } from './skill-migrations'
import {
  bundledAgentBrowserSkillHash,
  installedAgentBrowserSkillDir,
  installedAgentBrowserSkillHash,
} from './skill-install'
import { rmSync } from 'node:fs'
import { registerBuiltInPlugins } from './plugins'
import { registerBuiltInEntityKinds } from './entities'
import {
  initDevServerManager,
  shutdownDevServerManager,
} from './runtime/dev-server-manager'
import { spawn as nodeSpawn } from 'node:child_process'
import { initializeDocObservers } from './runtime/space-observers'
import { cancelActive as cancelActiveInteraction } from './runtime/interaction-controller'
import { sendInteractiveState } from './runtime/overlay-manager'
import { createCanvasUndoManager, setUndoSelectionHooks, clearUndoHistory } from './runtime/space-undo'
import { getActiveDoc } from './runtime/space-doc'
import { zoom, pan } from './runtime/runtime-context'
import { workspaceGroups, workspaceEdges, workspaceAnnotations, spaceTabs, activeSpaceTabId, setActiveSpaceTabId } from './runtime/space-model'
import { getUiState, setSelection } from './ui-state'
import { destroyActivePages } from './runtime/runtime-core'
import { initAutoUpdater } from './auto-updater'
import { initSentry } from './sentry'
import { initFileWatcher, teardownAllFileWatchers } from './runtime/local-file-watcher'
import {
  breadcrumb,
  identifyInstall,
  setTag,
  setWorkspaceSource,
} from './sentry-context'
import { subscribe as subscribeInteraction } from './runtime/interaction-controller'
import * as Sentry from '@sentry/electron/main'

app.setName('Specular')

// Sentry sets up its own crashReporter when a DSN is configured, so it must
// run before the local crashReporter.start() call below.
initSentry()

crashReporter.start({ submitURL: '', uploadToServer: false, ignoreSystemCrashHandler: false })

process.on('uncaughtException', (err) => logCrash('uncaughtException', err))
process.on('unhandledRejection', (reason) => logCrash('unhandledRejection', reason))
app.on('render-process-gone', (_e, wc, details) => {
  let host: string | undefined
  try { host = new URL(wc.getURL()).host } catch {}
  logCrash('render-process-gone', { url: wc.getURL(), ...details })
  Sentry.withScope((scope) => {
    scope.setTag('webview_host', host ?? 'unknown')
    scope.setTag('reason', details.reason)
    scope.setExtra('exitCode', details.exitCode)
    Sentry.captureMessage(`render-process-gone: ${details.reason}`, 'error')
  })
})
app.on('child-process-gone', (_e, details) => logCrash('child-process-gone', details))

const remoteDebuggingPort = process.env.SPECULAR_REMOTE_DEBUGGING_PORT ?? String(DEFAULT_REMOTE_DEBUGGING_PORT)
app.commandLine.appendSwitch('remote-debugging-port', remoteDebuggingPort)
app.commandLine.appendSwitch('remote-debugging-address', '127.0.0.1')
app.commandLine.appendSwitch('enable-unsafe-webgpu')

if (process.platform === 'darwin' && process.env.SPECULAR_BACKGROUND === '1') {
  app.setActivationPolicy('accessory')
}

// Allow smoke tests to isolate workspace data in a temp directory
const userDataDirArg = process.argv.find((a) => a.startsWith('--user-data-dir='))
if (userDataDirArg) {
  app.setPath('userData', userDataDirArg.slice('--user-data-dir='.length))
}

let quitRequested = false

/**
 * A missing space at boot prompts; it never falls back (ADR 0033 §4). Loops
 * a windowless dialog until the user locates the folder, opts into the
 * default space, or quits — silently falling back would autosave into a
 * different root than the one the user thinks they're working in.
 * Returns false when the app is quitting and boot should stop.
 */
async function resolveSpaceAtBoot(): Promise<boolean> {
  for (;;) {
    const configured = getSpacePath()
    if (!configured || isSpaceAvailable(configured)) return true

    const { response } = await dialog.showMessageBox({
      type: 'warning',
      buttons: ['Locate…', 'Open the default space', 'Quit'],
      defaultId: 0,
      cancelId: 2,
      title: "Specular can't find your space.",
      message: "Specular can't find your space.",
      detail: `The folder "${basename(configured)}" may be on a drive that isn't connected, or it was moved or renamed.`,
    })

    if (response === 2) {
      app.quit()
      return false
    }
    if (response === 1) {
      console.log(`Your space was at "${configured}" — now using the default space.`)
      setSpacePath(undefined)
      return true
    }
    const located = await dialog.showOpenDialog({ properties: ['openDirectory'] })
    const candidate = !located.canceled ? located.filePaths[0] : undefined
    if (candidate && isSpaceAvailable(candidate)) {
      setSpacePath(candidate)
      return true
    }
    // Canceled or still unusable — loop back to the message box.
  }
}

const hasSingleInstanceLock = app.requestSingleInstanceLock()
if (!hasSingleInstanceLock) {
  app.quit()
}

app.on('second-instance', () => {
  if (isOnboardingWindowOpen()) {
    focusOnboardingWindow()
    return
  }
  if (isSettingsWindowOpen()) {
    focusSettingsWindow()
    return
  }
  if (!win || win.isDestroyed()) return
  win.focus()
})

protocol.registerSchemesAsPrivileged([
  {
    scheme: 'local-file',
    privileges: { bypassCSP: true, supportFetchAPI: true, stream: true },
  },
])

app.whenReady().then(async () => {
  protocol.handle('local-file', (request) => {
    // Strip any ?v= cache-buster / #hash before resolving to a real file —
    // renderers append ?v=<fileReloadVersion> to force a fresh fetch on disk change.
    const raw = request.url.slice('local-file://'.length).split(/[?#]/)[0]
    const filePath = decodeURIComponent(raw)
    return net.fetch(`file://${filePath}`)
  })

  identifyInstall()
  configureBundledAgentBrowser()
  registerBuiltInPlugins()
  registerBuiltInEntityKinds()
  // New-tab links from a page open as a duplicate frame on the canvas rather
  // than a native popup; page-factory routes through this seam to avoid an
  // import cycle into workspace-pages.
  setOpenLinkInNewFrameHandler(({ sourcePageId, url, focus }) =>
    duplicatePageFromSource({ sourcePageId, url, focus }),
  )
  initFileWatcher((_entityIds) => {
    markDirty('canvas')
    requestLayout()
  })

  initDevServerManager({
    userDataDir: app.getPath('userData'),
    spawn: (command, args, options) =>
      nodeSpawn(command, args as string[], { ...options, shell: process.platform === 'win32' }),
  })

  setupAppMenu()
  registerIpcHandlers()
  await startAppControlServer()

  // Silently update skills the user hasn't hand-edited; surfaces drift via the
  // app menu label (refreshed below).
  autoUpdateSkillsIfSafe()
  void runAgentBrowserSkillRemovalMigration({
    loadState: loadOnboardingState,
    saveState: saveOnboardingState,
    installedHash: installedAgentBrowserSkillHash,
    bundledHash: bundledAgentBrowserSkillHash,
    installedDir: installedAgentBrowserSkillDir,
    hasUserBinary: hasUserOwnedAgentBrowserBinary,
    removeDir: (dir) => rmSync(dir, { recursive: true, force: true }),
  })
  refreshAppMenu()

  const skipOnboarding = process.env.SPECULAR_SKIP_ONBOARDING === '1'
  if (!skipOnboarding && !loadOnboardingState().completed) {
    breadcrumb('onboarding', 'shown')
    const reason = await showOnboardingWindow('welcome')
    breadcrumb('onboarding', reason)
    if (quitRequested) return
  }

  if (!(await resolveSpaceAtBoot())) return

  initWindow()
  setMcpConnectionStatus(getMcpConnectionStatus())
  onMcpConnectionStatusChanged((status) => {
    setTag('has_mcp_connection', status.healthy)
    breadcrumb('mcp', status.healthy ? 'connected' : 'disconnected', {
      clients: status.activeClientCount,
    })
    setMcpConnectionStatus(status)
  })
  subscribeInteraction((mode) => {
    breadcrumb('interaction', mode.kind)
  })
  onPresenceCursorsChanged(() => {
    markDirty('canvas', 'toolbar')
    requestLayout()
  })
  setInterval(() => {
    setMcpConnectionStatus(getMcpConnectionStatus())
  }, 5_000)

  // Load workspace from .canvas files (primary), falling back to legacy workspace-store.json
  const persistedWorkspace = loadSpace()
  const restoredPersistedWorkspace = persistedWorkspace
    ? restorePersistedSpace(persistedWorkspace)
    : false

  if (!restoredPersistedWorkspace) {
    for (const cfg of DEFAULT_PAGES) {
      createPage(cfg)
    }
  }

  setWorkspaceSource(restoredPersistedWorkspace ? 'restored' : 'new')
  breadcrumb('workspace', 'loaded', {
    source: restoredPersistedWorkspace ? 'restored' : 'new',
  })

  if (!restoredPersistedWorkspace) {
    toggleDevTools()
  }

  const doc = getActiveDoc()
  createCanvasUndoManager(doc)
  setUndoSelectionHooks(
    () => getUiState().selection,
    (selection) => setSelection(selection as any),
  )
  initializeDocObservers({
    pages,
    workspaceGroups,
    workspaceEdges,
    workspaceAnnotations,
    getZoom: () => zoom,
    getPan: () => pan,
    cancelActiveInteraction: () => cancelActiveInteraction('undo'),
    sendInteractiveState,
    destroyActivePages,
    getActiveTabId: () => activeSpaceTabId,
    setActiveTabId: setActiveSpaceTabId,
    spaceTabs,
  })
  // Clear any undo entries created by the initial doc sync
  clearUndoHistory()

  requestLayout()

  // Theme detection
  nativeTheme.on('updated', () => {
    win!.contentView.setBackgroundColor(isDark() ? '#18181b' : '#f5f5f4')
    broadcastTheme()
  })

  initAutoUpdater()

  console.log('\n=== Specular ===')
  console.log('Cmd+scroll to zoom, trackpad scroll to pan.')
  console.log('Chrome headers: drag to reposition, arrows to cycle presets.\n')
})

app.on('window-all-closed', () => {
  stopAppControlServer()
  app.quit()
})

app.on('before-quit', () => {
  quitRequested = true
  flushSpaceAutosaveSync()
  teardownAllFileWatchers()
  void shutdownDevServerManager()
})
