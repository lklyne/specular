/**
 * Option-list model and highlight math for the in-page `<select>` fallback
 * dropdown (ADR 0038 — offscreen pages).
 *
 * Pure: reads only the duck-typed option fields below, touches no DOM APIs, so
 * the list shaping and keyboard navigation are unit-testable without jsdom.
 */

export type SelectOptionLike = {
  /** `label` attribute when present; the native menulist prefers it over text. */
  label?: string
  text: string
  value: string
  disabled?: boolean
  selected?: boolean
  /** Label of the enclosing `<optgroup>`, if any. */
  groupLabel?: string | null
  /** `disabled` on the enclosing `<optgroup>`, which disables its options. */
  groupDisabled?: boolean
}

export type SelectLike = {
  options: ArrayLike<SelectOptionLike>
}

export type SelectFallbackItem = {
  kind: 'option' | 'group'
  label: string
  value?: string
  /** True for group headers, disabled options, and options in a disabled group. */
  disabled: boolean
  /** Index into `select.options` for an option; -1 for a group header. */
  index: number
}

export type SelectFallbackModel = {
  items: SelectFallbackItem[]
  /** Index into `items` of the selected option, or -1 when nothing is selected. */
  selectedIndex: number
}

export type HighlightKey =
  | 'ArrowDown'
  | 'ArrowUp'
  | 'Home'
  | 'End'
  | 'PageDown'
  | 'PageUp'

const PAGE_STEP = 10

function optionLabel(option: SelectOptionLike): string {
  const raw = option.label || option.text || ''
  return raw.replace(/\s+/g, ' ').trim()
}

/**
 * Flatten a menulist into the rows the dropdown paints: one row per option,
 * preceded by a non-selectable header row wherever an `<optgroup>` starts.
 */
export function selectFallbackModel(select: SelectLike): SelectFallbackModel {
  const items: SelectFallbackItem[] = []
  let selectedIndex = -1
  let openGroup: string | null = null

  for (let i = 0; i < select.options.length; i += 1) {
    const option = select.options[i]
    const groupLabel = option.groupLabel ?? null
    if (groupLabel !== openGroup) {
      openGroup = groupLabel
      if (groupLabel) {
        items.push({ kind: 'group', label: groupLabel, disabled: true, index: -1 })
      }
    }
    if (option.selected) selectedIndex = items.length
    items.push({
      kind: 'option',
      label: optionLabel(option),
      value: option.value,
      disabled: Boolean(option.disabled) || Boolean(option.groupDisabled),
      index: i,
    })
  }

  return { items, selectedIndex }
}

export function isSelectableItem(item: SelectFallbackItem | undefined): boolean {
  return item?.kind === 'option' && !item.disabled
}

function selectablePositions(items: readonly SelectFallbackItem[]): number[] {
  const positions: number[] = []
  for (let i = 0; i < items.length; i += 1) {
    if (isSelectableItem(items[i])) positions.push(i)
  }
  return positions
}

/**
 * Where a navigation key moves the highlight. Returns an index into `items`,
 * skipping group headers and disabled options; arrows wrap, paging clamps.
 * `current` may be -1 (nothing highlighted yet) or point at a non-selectable
 * row, in which case navigation starts from the nearest end.
 */
export function nextHighlight(
  items: readonly SelectFallbackItem[],
  current: number,
  key: HighlightKey,
): number {
  const positions = selectablePositions(items)
  if (positions.length === 0) return current
  const last = positions.length - 1
  const at = positions.indexOf(current)

  switch (key) {
    case 'Home':
      return positions[0]
    case 'End':
      return positions[last]
    case 'ArrowDown':
      return at < 0 ? positions[0] : positions[(at + 1) % positions.length]
    case 'ArrowUp':
      return at < 0 ? positions[last] : positions[(at - 1 + positions.length) % positions.length]
    case 'PageDown':
      return positions[Math.min(last, (at < 0 ? 0 : at) + PAGE_STEP)]
    case 'PageUp':
      return positions[Math.max(0, (at < 0 ? last : at) - PAGE_STEP)]
  }
}

/**
 * Type-ahead: first selectable row whose label starts with `prefix`, searching
 * from `from` (exclusive) and wrapping once. Returns -1 when nothing matches.
 */
export function typeAheadMatch(
  items: readonly SelectFallbackItem[],
  prefix: string,
  from: number,
): number {
  const needle = prefix.trim().toLowerCase()
  if (!needle) return -1
  const positions = selectablePositions(items)
  if (positions.length === 0) return -1
  const at = positions.indexOf(from)
  // A single repeated character cycles through same-initial rows, so start
  // after the current row; a growing prefix should be able to re-match it.
  const startOffset = needle.length > 1 && at >= 0 ? 0 : 1
  for (let step = startOffset; step < positions.length + startOffset; step += 1) {
    const position = positions[(at + step + positions.length) % positions.length]
    if (items[position].label.toLowerCase().startsWith(needle)) return position
  }
  return -1
}
