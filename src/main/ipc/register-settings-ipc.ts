import { ipcChannels } from '../../shared/ipc-contract'
import { app, BrowserWindow, ipcMain, shell } from 'electron'
import type {
  FixModel,
  FixPermissions,
  OnboardingComponentId,
  OnboardingProgressEvent,
  OnboardingStatusSnapshot,
  SettingsBootstrapData,
} from '../../shared/types'
import {
  getFixConfig,
  getSpacePath,
  getThemeMode,
  isDark,
  setFixConfig,
} from '../runtime/preferences'
import { changeSpaceViaPicker } from '../runtime/space-change'
import { spaceDir } from '../runtime/space-dir'
import { listRepos, removeBindingByOrigin } from '../runtime/dev-server-manager'
import {
  getOnboardingStatus,
  onOnboardingStatusChanged,
  refreshOnboardingStatus,
} from '../onboarding-status'
import {
  runComponentToggle,
  runSkillInstallSelections,
} from '../skill-install-runner'
import { refreshAppMenu } from '../runtime/app-menu'
import { checkForUpdatesManually } from '../auto-updater'
import { notifyDevtoolsPanelData } from '../runtime/inspect-session'
import {
  closeSettingsWindow,
  getSettingsWebContents,
} from '../settings-window'

function sendToSettings(channel: string, payload: unknown): void {
  const wc = getSettingsWebContents()
  if (!wc || wc.isDestroyed()) return
  wc.send(channel, payload)
}

function broadcastProgress(event: OnboardingProgressEvent): void {
  sendToSettings(ipcChannels.settingsSkillProgress, event)
}

function broadcastFixConfig(): void {
  sendToSettings(ipcChannels.settingsFixConfigChanged, getFixConfig())
}

function broadcastSpaceChanged(path: string): void {
  sendToSettings(ipcChannels.spaceChanged, { path, isDefault: getSpacePath() === undefined })
}

export function registerSettingsIpc(): void {
  onOnboardingStatusChanged((status) => broadcastProgress({ kind: 'done', status }))

  ipcMain.handle(
    ipcChannels.settingsGetInitialData,
    (): SettingsBootstrapData => ({
      theme: { isDark: isDark(), themeMode: getThemeMode() },
      version: app.getVersion(),
      status: getOnboardingStatus(),
      fixConfig: getFixConfig(),
      connectedRepos: listRepos(),
      space: { path: spaceDir(), isDefault: getSpacePath() === undefined },
    }),
  )

  ipcMain.handle(ipcChannels.spaceChangeViaPicker, async (event): Promise<string | null> => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win) return null
    const newPath = await changeSpaceViaPicker(win)
    if (newPath) broadcastSpaceChanged(newPath)
    return newPath
  })

  ipcMain.on(ipcChannels.settingsCheckForUpdates, () => {
    void checkForUpdatesManually()
  })

  ipcMain.on(ipcChannels.spaceRevealInFinder, () => {
    void shell.openPath(spaceDir())
  })

  ipcMain.handle(ipcChannels.settingsRefreshStatus, async (): Promise<OnboardingStatusSnapshot> => {
    return await refreshOnboardingStatus()
  })

  ipcMain.handle(
    ipcChannels.settingsInstallSkills,
    async (
      _event,
      selections: Record<OnboardingComponentId, boolean>,
    ): Promise<OnboardingStatusSnapshot> => {
      const status = await runSkillInstallSelections(selections, broadcastProgress)
      refreshAppMenu()
      return status
    },
  )

  ipcMain.handle(
    ipcChannels.settingsSetComponentInstalled,
    async (
      _event,
      payload: { component: OnboardingComponentId; installed: boolean },
    ): Promise<OnboardingStatusSnapshot> => {
      const status = await runComponentToggle(
        payload.component,
        payload.installed,
        broadcastProgress,
      )
      refreshAppMenu()
      return status
    },
  )

  ipcMain.on(
    ipcChannels.settingsSetFixConfig,
    (_event, payload: { model?: FixModel; permissions?: FixPermissions } | undefined) => {
      if (!payload) return
      setFixConfig(payload)
      broadcastFixConfig()
      notifyDevtoolsPanelData()
    },
  )

  ipcMain.on(
    ipcChannels.settingsRemoveOriginBinding,
    (_event, origin: unknown) => {
      const trimmed = typeof origin === 'string' ? origin.trim() : ''
      if (!trimmed) return
      if (removeBindingByOrigin(trimmed)) notifyDevtoolsPanelData()
    },
  )

  ipcMain.on(ipcChannels.settingsClose, () => {
    closeSettingsWindow()
  })
}
