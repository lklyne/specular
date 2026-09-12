import { registerAnnotationInspectionIpc } from './ipc/register-annotation-inspection-ipc'
import { registerAppIpc } from './ipc/register-app-ipc'
import { registerCanvasIpc } from './ipc/register-canvas-ipc'
import { registerCommentHoverIpc } from './ipc/register-comment-hover-ipc'
import { registerOnboardingIpc } from './ipc/register-onboarding-ipc'
import { registerPageChromeIpc } from './ipc/register-page-chrome-ipc'
import { registerRepoIpc } from './ipc/register-repo-ipc'
import { registerRightDetailsPanelIpc } from './ipc/register-right-details-panel-ipc'
import { registerSettingsIpc } from './ipc/register-settings-ipc'
import { registerToolbarIpc } from './ipc/register-toolbar-ipc'
import { registerDebugIpc } from './ipc/register-debug-ipc'

export function registerIpcHandlers(): void {
  registerAppIpc()
  registerDebugIpc()
  registerToolbarIpc()
  registerCanvasIpc()
  registerRightDetailsPanelIpc()
  registerPageChromeIpc()
  registerRepoIpc()
  registerAnnotationInspectionIpc()
  registerCommentHoverIpc()
  registerOnboardingIpc()
  registerSettingsIpc()
}
