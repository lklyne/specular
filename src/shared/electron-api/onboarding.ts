import type {
  OnboardingBootstrapData,
  OnboardingComponentId,
  OnboardingProgressEvent,
  OnboardingStatusSnapshot,
  ThemeData,
} from '../types'

export interface OnboardingElectronAPI {
  getInitialData: () => Promise<OnboardingBootstrapData>
  install: (
    selections: Record<OnboardingComponentId, boolean>,
  ) => Promise<OnboardingStatusSnapshot>
  complete: () => void
  dismiss: () => void
  /** Just the folder picker — no migration logic (onboarding runs before any
   *  workspace loads, so there's nothing to migrate). Returns the picked
   *  path, or null if canceled. */
  spaceChooseViaPicker: () => Promise<string | null>
  /** mkdir + persist the chosen (or accepted-default) space path. */
  spaceCommit: (path: string) => Promise<void>
  onProgress: (callback: (event: OnboardingProgressEvent) => void) => () => void
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
}
