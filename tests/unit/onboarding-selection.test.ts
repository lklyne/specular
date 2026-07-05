import { describe, expect, it } from 'vitest'
import type { OnboardingComponentId, OnboardingStatusSnapshot } from '../../src/shared/types'
import {
  hasInstallableSelection,
  installableSelections,
} from '../../src/renderer/onboarding/onboardingSelection'

const baseStatus: OnboardingStatusSnapshot = {
  cli: { kind: 'missing' },
  skill: { kind: 'missing' },
  agentBrowser: { kind: 'missing' },
  claudeDirExists: true,
}

function selections(value: boolean): Record<OnboardingComponentId, boolean> {
  return { cli: value, skill: value, agentBrowser: value }
}

describe('onboarding selection helpers', () => {
  it('does not submit already-installed rows for installation', () => {
    const status: OnboardingStatusSnapshot = {
      ...baseStatus,
      cli: { kind: 'installed' },
      skill: { kind: 'outdated', detail: 'New bundled version available.' },
    }

    expect(installableSelections(status, selections(true))).toEqual({
      cli: false,
      skill: true,
      agentBrowser: true,
    })
  })

  it('ignores checked installed rows when deciding if install can run', () => {
    const status: OnboardingStatusSnapshot = {
      cli: { kind: 'installed' },
      skill: { kind: 'installed' },
      agentBrowser: { kind: 'installed' },
      claudeDirExists: true,
    }

    expect(hasInstallableSelection(status, selections(true))).toBe(false)
  })
})
