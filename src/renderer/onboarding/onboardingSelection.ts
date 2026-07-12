import type {
  OnboardingComponentId,
  OnboardingStatusSnapshot,
} from '../../shared/types'

const ONBOARDING_COMPONENT_IDS: OnboardingComponentId[] = ['cli', 'skill']

export function installableSelections(
  status: OnboardingStatusSnapshot,
  selections: Record<OnboardingComponentId, boolean>,
): Record<OnboardingComponentId, boolean> {
  return Object.fromEntries(
    ONBOARDING_COMPONENT_IDS.map((id) => [
      id,
      selections[id] && status[id].kind !== 'installed',
    ]),
  ) as Record<OnboardingComponentId, boolean>
}

export function hasInstallableSelection(
  status: OnboardingStatusSnapshot,
  selections: Record<OnboardingComponentId, boolean>,
): boolean {
  return Object.values(installableSelections(status, selections)).some(Boolean)
}
