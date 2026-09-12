/**
 * The option-list model and highlight math behind the in-page `<select>`
 * fallback dropdown (ADR 0038). Offscreen pages get no native popup menu, so
 * this model *is* the menu: if it mis-shapes optgroups or lets the highlight
 * land on a disabled row, the dropdown commits values the native control never
 * would.
 *
 * Mutation-verified by (a) deleting the `groupLabel !== openGroup` guard in
 * selectFallbackModel, which emits a header per option and fails the optgroup
 * cases, and (b) dropping the `disabled` term from `isSelectableItem`, which
 * makes nextHighlight stop on disabled rows and fails the skip cases.
 */

import { describe, expect, it } from 'vitest'
import {
  nextHighlight,
  selectFallbackModel,
  typeAheadMatch,
  type SelectFallbackItem,
  type SelectOptionLike,
} from '../../src/preload/select-fallback-model'

function option(
  text: string,
  extra: Partial<SelectOptionLike> = {},
): SelectOptionLike {
  return { text, value: text.toLowerCase(), ...extra }
}

describe('selectFallbackModel', () => {
  it('emits one header per optgroup and keeps option indices aligned with select.options', () => {
    const model = selectFallbackModel({
      options: [
        option('Plain'),
        option('Cheddar', { groupLabel: 'Cheese' }),
        option('Brie', { groupLabel: 'Cheese', selected: true }),
        option('Pinot', { groupLabel: 'Wine' }),
      ],
    })

    expect(model.items).toEqual<SelectFallbackItem[]>([
      { kind: 'option', label: 'Plain', value: 'plain', disabled: false, index: 0 },
      { kind: 'group', label: 'Cheese', disabled: true, index: -1 },
      { kind: 'option', label: 'Cheddar', value: 'cheddar', disabled: false, index: 1 },
      { kind: 'option', label: 'Brie', value: 'brie', disabled: false, index: 2 },
      { kind: 'group', label: 'Wine', disabled: true, index: -1 },
      { kind: 'option', label: 'Pinot', value: 'pinot', disabled: false, index: 3 },
    ])
    // Index into items (the row to highlight), not into select.options.
    expect(model.selectedIndex).toBe(3)
  })

  it('disables an option when the option or its group is disabled, and prefers the label attribute', () => {
    const model = selectFallbackModel({
      options: [
        option('Ok'),
        option('Nope', { disabled: true }),
        option('Grouped', { groupLabel: 'Off', groupDisabled: true }),
        option('  spaced\n  out  ', { label: 'Short name' }),
      ],
    })

    expect(model.items.map((item) => [item.label, item.disabled])).toEqual([
      ['Ok', false],
      ['Nope', true],
      ['Off', true],
      ['Grouped', true],
      ['Short name', false],
    ])
    expect(model.selectedIndex).toBe(-1)
  })
})

const GROUPED = selectFallbackModel({
  options: [
    option('a'),
    option('b', { disabled: true }),
    option('c', { groupLabel: 'G' }),
    option('d', { groupLabel: 'G' }),
  ],
}).items
// items: [a, b(disabled), G(header), c, d] → selectable rows 0, 3, 4

describe('nextHighlight', () => {
  it('skips disabled options and group headers', () => {
    expect(nextHighlight(GROUPED, 0, 'ArrowDown')).toBe(3)
    expect(nextHighlight(GROUPED, 3, 'ArrowUp')).toBe(0)
  })

  it('wraps at both ends for arrows', () => {
    expect(nextHighlight(GROUPED, 4, 'ArrowDown')).toBe(0)
    expect(nextHighlight(GROUPED, 0, 'ArrowUp')).toBe(4)
  })

  it('starts from an end when nothing is highlighted yet', () => {
    expect(nextHighlight(GROUPED, -1, 'ArrowDown')).toBe(0)
    expect(nextHighlight(GROUPED, -1, 'ArrowUp')).toBe(4)
  })

  it('jumps to the first and last selectable row for Home and End', () => {
    expect(nextHighlight(GROUPED, 4, 'Home')).toBe(0)
    expect(nextHighlight(GROUPED, 0, 'End')).toBe(4)
  })

  it('clamps paging instead of wrapping', () => {
    const long = selectFallbackModel({
      options: Array.from({ length: 30 }, (_, i) => option(`o${i}`)),
    }).items
    expect(nextHighlight(long, 0, 'PageDown')).toBe(10)
    expect(nextHighlight(long, 25, 'PageDown')).toBe(29)
    expect(nextHighlight(long, 4, 'PageUp')).toBe(0)
    expect(nextHighlight(GROUPED, 0, 'PageDown')).toBe(4)
  })

  it('holds still when no row is selectable', () => {
    const allDisabled = selectFallbackModel({
      options: [option('x', { disabled: true })],
    }).items
    expect(nextHighlight(allDisabled, 0, 'ArrowDown')).toBe(0)
  })
})

describe('typeAheadMatch', () => {
  const items = selectFallbackModel({
    options: [
      option('Apple'),
      option('Apricot'),
      option('Avocado', { disabled: true }),
      option('Banana'),
    ],
  }).items

  it('cycles through same-initial rows on a repeated single character', () => {
    expect(typeAheadMatch(items, 'a', -1)).toBe(0)
    expect(typeAheadMatch(items, 'a', 0)).toBe(1)
    // Avocado is disabled, so the cycle wraps back to Apple.
    expect(typeAheadMatch(items, 'a', 1)).toBe(0)
  })

  it('re-matches the highlighted row as the prefix grows', () => {
    expect(typeAheadMatch(items, 'ap', 0)).toBe(0)
    expect(typeAheadMatch(items, 'apr', 0)).toBe(1)
  })

  it('is case-insensitive and returns -1 with no match', () => {
    expect(typeAheadMatch(items, 'BAN', -1)).toBe(3)
    expect(typeAheadMatch(items, 'z', -1)).toBe(-1)
    expect(typeAheadMatch(items, '', 0)).toBe(-1)
  })
})
