/**
 * `applyToolDefaultPatch` — the one door every tool-default control writes
 * through (text/sticky font and size, shape kind, brush).
 *
 * `ToolDefaultPatch` is a discriminated union pairing each scope with only the
 * keys that scope holds, and the mutator addresses `[scope][key]` straight off
 * that pair. What can go wrong is therefore addressing: a patch landing in the
 * wrong scope, or overwriting a sibling key in the right one. Both are silent
 * — the write succeeds, just not where the user asked — so these assert the
 * whole defaults blob, not only the slot that was meant to change.
 *
 * Mutation-verified by (a) hard-coding the write to the 'draw' scope — the
 * per-scope cases fail on the untouched-neighbours assertion; (b) hard-coding
 * the key to 'color' — the textFont and textSize cases fail.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { applyToolDefaultPatch, getToolDefaults } from '../../src/main/runtime/tool-defaults'
import { DEFAULT_TOOL_DEFAULTS } from '../../src/shared/tool-defaults'
import { saveToolDefaults } from '../../src/main/runtime/preferences'

let harness: WorkspaceHarness
beforeEach(() => {
  harness ??= bootWorkspaceHarness()
  harness.reset()
  saveToolDefaults(DEFAULT_TOOL_DEFAULTS)
})
afterAll(() => harness?.dispose())

describe('applyToolDefaultPatch', () => {
  it('writes each scope/key pair into that slot and nothing else', () => {
    const cases = [
      { scope: 'add-text', key: 'textFont', value: 'mono' },
      { scope: 'add-text', key: 'textSize', value: 32 },
      { scope: 'add-text', key: 'color', value: null },
      { scope: 'add-sticky', key: 'textFont', value: 'hand' },
      { scope: 'add-sticky', key: 'color', value: '5' },
      { scope: 'add-shape', key: 'shapeKind', value: 'ellipse' },
      { scope: 'add-shape', key: 'strokeWidth', value: 4 },
      { scope: 'draw', key: 'brushType', value: 'highlight' },
      { scope: 'draw', key: 'color', value: '#111111' },
    ] as const

    for (const patch of cases) {
      saveToolDefaults(DEFAULT_TOOL_DEFAULTS)
      applyToolDefaultPatch(patch)
      expect(getToolDefaults()).toEqual({
        ...DEFAULT_TOOL_DEFAULTS,
        [patch.scope]: { ...DEFAULT_TOOL_DEFAULTS[patch.scope], [patch.key]: patch.value },
      })
    }
  })

  it('leaves the sibling font untouched when one scope changes', () => {
    applyToolDefaultPatch({ scope: 'add-sticky', key: 'textFont', value: 'mono' })
    expect(getToolDefaults()['add-sticky'].textFont).toBe('mono')
    expect(getToolDefaults()['add-text'].textFont).toBe(
      DEFAULT_TOOL_DEFAULTS['add-text'].textFont,
    )
  })

  it('survives the round-trip through normalization, so a font persists', () => {
    applyToolDefaultPatch({ scope: 'add-text', key: 'textFont', value: 'hand' })
    // saveToolDefaults re-normalizes on the way in; an unrecognized token
    // would be silently dropped back to the default here.
    expect(getToolDefaults()['add-text'].textFont).toBe('hand')
  })
})
