import { ipcChannels } from '../../shared/ipc-contract'
import { ipcMain } from 'electron'
import type {
  OnboardingComponentId,
  OnboardingProgressEvent,
  OnboardingStatusSnapshot,
} from '../../shared/types'
import { isDark } from '../runtime/preferences'
import { loadOnboardingState, saveOnboardingState } from '../runtime/preferences'
import { getOnboardingStatus } from '../onboarding-status'
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
  ipcMain.handle(ipcChannels.onboardingGetInitialData, async () => ({
    theme: { isDark: isDark() },
    status: await getOnboardingStatus(),
    mode: getOnboardingMode(),
  }))

  ipcMain.handle(ipcChannels.onboardingRefreshStatus, async (): Promise<OnboardingStatusSnapshot> => {
    return await getOnboardingStatus()
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
