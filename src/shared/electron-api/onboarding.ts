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
  onProgress: (callback: (event: OnboardingProgressEvent) => void) => () => void
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
}
