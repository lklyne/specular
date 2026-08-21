import { ipcChannels } from '../../shared/ipc-contract'
import { app, BrowserWindow, dialog, ipcMain } from 'electron'
import { mkdirSync } from 'fs'
import { join } from 'path'
import type {
  OnboardingBootstrapData,
  OnboardingComponentId,
  OnboardingProgressEvent,
  OnboardingStatusSnapshot,
} from '../../shared/types'
import {
  getSpacePath,
  getThemeMode,
  isDark,
  loadOnboardingState,
  saveOnboardingState,
  setSpacePath,
} from '../runtime/preferences'
import {
  getOnboardingStatus,
  onOnboardingStatusChanged,
  refreshOnboardingStatus,
} from '../onboarding-status'
import { spacePickerDefaultPath } from '../runtime/picker-defaults'
import { runSkillInstallSelections } from '../skill-install-runner'
import { refreshAppMenu } from '../runtime/app-menu'
import {
  closeAndResolve,
  getOnboardingMode,
  getOnboardingWebContents,
} from '../onboarding-window'

function broadcast(event: OnboardingProgressEvent): void {
  const wc = getOnboardingWebContents()
  if (!wc || wc.isDestroyed()) return
  wc.send(ipcChannels.onboardingProgress, event)
}

export function registerOnboardingIpc(): void {
  onOnboardingStatusChanged((status) => broadcast({ kind: 'done', status }))

  ipcMain.handle(ipcChannels.onboardingGetInitialData, (): OnboardingBootstrapData => ({
    theme: { isDark: isDark(), themeMode: getThemeMode() },
    status: getOnboardingStatus(),
    mode: getOnboardingMode(),
    defaultSpacePath: join(app.getPath('home'), 'Specular'),
    spacePath: getSpacePath() ?? null,
  }))

  // Onboarding runs before any workspace loads (src/main/index.ts loads the
  // workspace after the onboarding gate), so committing a space here needs
  // no reopen and no migration prompt — there's nothing open yet to tear
  // down or migrate from.
  ipcMain.handle(ipcChannels.spaceChooseViaPicker, async (event): Promise<string | null> => {
    const win = BrowserWindow.fromWebContents(event.sender)
    const dialogOpts: Electron.OpenDialogOptions = {
      title: 'Choose a space folder',
      properties: ['openDirectory', 'createDirectory'],
      defaultPath: spacePickerDefaultPath(),
    }
    const result = win
      ? await dialog.showOpenDialog(win, dialogOpts)
      : await dialog.showOpenDialog(dialogOpts)
    if (result.canceled || !result.filePaths.length) return null
    return result.filePaths[0]
  })

  ipcMain.handle(ipcChannels.spaceCommit, async (_event, path: unknown): Promise<void> => {
    if (typeof path !== 'string' || !path) return
    mkdirSync(path, { recursive: true })
    setSpacePath(path)
  })

  ipcMain.handle(ipcChannels.onboardingRefreshStatus, async (): Promise<OnboardingStatusSnapshot> => {
    return await refreshOnboardingStatus()
  })

  ipcMain.handle(
    ipcChannels.onboardingInstall,
    async (
      _event,
      selections: Record<OnboardingComponentId, boolean>,
    ): Promise<OnboardingStatusSnapshot> => {
      const status = await runSkillInstallSelections(selections, broadcast)
      refreshAppMenu()
      return status
    },
  )

  ipcMain.on(ipcChannels.onboardingComplete, () => {
    const prev = loadOnboardingState()
    saveOnboardingState({ ...prev, completed: true, completedAt: Date.now() })
    closeAndResolve('complete')
    refreshAppMenu()
  })

  ipcMain.on(ipcChannels.onboardingDismiss, () => {
    const prev = loadOnboardingState()
    saveOnboardingState({ ...prev, dismissedAt: Date.now() })
    closeAndResolve('dismiss')
    refreshAppMenu()
  })
}
