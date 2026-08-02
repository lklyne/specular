import type {
  ConnectedRepo,
  FixConfig,
  FixModel,
  FixPermissions,
  OnboardingComponentId,
  OnboardingProgressEvent,
  OnboardingStatusSnapshot,
  SettingsBootstrapData,
  ThemeData,
} from '../types'

export interface SettingsElectronAPI {
  getInitialData: () => Promise<SettingsBootstrapData>
  refreshStatus: () => Promise<OnboardingStatusSnapshot>
  installSkills: (
    selections: Record<OnboardingComponentId, boolean>,
  ) => Promise<OnboardingStatusSnapshot>
  setComponentInstalled: (
    component: OnboardingComponentId,
    installed: boolean,
  ) => Promise<OnboardingStatusSnapshot>
  setFixConfig: (config: { model: FixModel; permissions: FixPermissions }) => void
  removeOriginBinding: (origin: string) => void
  repoConnectViaPicker: () => Promise<ConnectedRepo | null>
  repoDisconnect: (id: string) => Promise<void>
  repoBindOrigin: (repoId: string, origin: string) => Promise<ConnectedRepo | null>
  /** Runs the full §3 change-space flow (picker, migration prompts, reopen).
   *  Returns the new resolved space path, or null if canceled at any step. */
  spaceChangeViaPicker: () => Promise<string | null>
  spaceRevealInFinder: () => void
  close: () => void
  onSkillProgress: (callback: (event: OnboardingProgressEvent) => void) => () => void
  onFixConfigChanged: (callback: (config: FixConfig) => void) => () => void
  onConnectedReposChanged: (callback: (repos: ConnectedRepo[]) => void) => () => void
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
  onSpaceChanged: (callback: (data: { path: string; isDefault: boolean }) => void) => () => void
}
