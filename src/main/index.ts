import { app, crashReporter, net, nativeTheme, protocol } from 'electron'
import { DEFAULT_PAGES } from '../shared/constants'
import { logCrash } from './crash-log'
import {
  flushWorkspaceAutosaveSync,
  loadWorkspace,
  restorePersistedWorkspace,
} from './runtime/workspace-session'
import { createPage, pages, removePageById, setMcpConnectionStatus } from './runtime/page-runtime'
import { requestLayout } from './runtime/surface-layout'
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
import { loadOnboardingState } from './runtime/preferences'
import { showOnboardingWindow, focusOnboardingWindow, isOnboardingWindowOpen } from './onboarding-window'
import { focusSettingsWindow, isSettingsWindowOpen } from './settings-window'
import { configureBundledAgentBrowser } from './agent-browser-install'
import { autoUpdateSkillsIfSafe } from './skill-auto-update'
import { registerBuiltInPlugins } from './plugins'
import {
  initDevServerManager,
  shutdownDevServerManager,
} from './runtime/dev-server-manager'
import { spawn as nodeSpawn } from 'node:child_process'
import { initializeDocObservers } from './runtime/workspace-observers'
import { cancelActive as cancelActiveInteraction } from './runtime/interaction-controller'
import { sendInteractiveState } from './runtime/overlay-manager'
import { createCanvasUndoManager, setUndoSelectionHooks, clearUndoHistory } from './runtime/workspace-undo'
import { getActiveDoc } from './runtime/workspace-doc'
import { zoom, pan } from './runtime/runtime-context'
import { workspaceGroups, workspaceEdges, workspaceAnnotations, workspaceTabs, activeWorkspaceTabId, setActiveWorkspaceTabId } from './runtime/workspace-model'
import { textEntities } from './runtime/text-entity-state'
import { fileEntities } from './runtime/file-entity-state'
import { drawingEntities } from './runtime/drawing-entity-state'
import { shapeEntities } from './runtime/shape-entity-state'
import { getUiState, setSelection } from './ui-state'
import { destroyActivePages } from './runtime/runtime-core'
import { initAutoUpdater } from './auto-updater'
import { initSentry } from './sentry'
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

const remoteDebuggingPort = process.env.SPECULAR_REMOTE_DEBUGGING_PORT ?? '9229'
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
    const filePath = decodeURIComponent(request.url.replace('local-file://', ''))
    return net.fetch(`file://${filePath}`)
  })

  identifyInstall()
  configureBundledAgentBrowser()
  registerBuiltInPlugins()
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
  refreshAppMenu()

  const skipOnboarding = process.env.SPECULAR_SKIP_ONBOARDING === '1'
  if (!skipOnboarding && !loadOnboardingState().completed) {
    breadcrumb('onboarding', 'shown')
    const reason = await showOnboardingWindow('welcome')
    breadcrumb('onboarding', reason)
    if (quitRequested) return
  }

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
  const persistedWorkspace = loadWorkspace()
  const restoredPersistedWorkspace = persistedWorkspace
    ? restorePersistedWorkspace(persistedWorkspace)
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
    textEntities,
    fileEntities,
    drawingEntities,
    shapeEntities,
    workspaceGroups,
    workspaceEdges,
    workspaceAnnotations,
    getZoom: () => zoom,
    getPan: () => pan,
    serializePage: (page) => ({
      id: page.id,
      name: page.name,
      url: page.url,
      presetIndex: page.presetIndex,
      canvasX: page.canvasX,
      canvasY: page.canvasY,
      linked: page.linked,
      source: (page as any).source,
      parentGroupId: page.parentGroupId ?? (page as any).groupId,
      metadata: page.metadata,
    }),
    cancelActiveInteraction: () => cancelActiveInteraction('undo'),
    sendInteractiveState,
    createPage: (data) => createPage(data as any),
    removePageById,
    destroyActivePages,
    getActiveTabId: () => activeWorkspaceTabId,
    setActiveTabId: setActiveWorkspaceTabId,
    workspaceTabs,
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
  flushWorkspaceAutosaveSync()
  void shutdownDevServerManager()
})
