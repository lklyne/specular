import type { AgentPresenceCursor, PresenceLabelKey } from './types'

const PRESENCE_LABELS: Record<
  PresenceLabelKey,
  string | null | ((targetName?: string | null) => string)
> = {
  scan_workspace: 'Scanning workspace',
  find_placement: 'Finding placement',
  create_page: 'Creating page',
  select_page: 'Selecting page',
  attach_page: 'Attaching to page',
  inspect_page: 'Inspecting page',
  find_target: (targetName) => (targetName ? `Finding ${targetName}` : 'Finding target'),
  click_target: (targetName) => (targetName ? `Clicking "${targetName}"` : 'Clicking target'),
  point_target: (targetName) => (targetName ? `Pointing at "${targetName}"` : 'Pointing'),
  drag_target: (targetName) => (targetName ? `Dragging on "${targetName}"` : 'Dragging'),
  type_text: (targetName) => (targetName ? `Typing in "${targetName}"` : 'Typing text'),
  select_option: (targetName) => (targetName ? `Selecting "${targetName}"` : 'Selecting option'),
  wait_page: (targetName) => (targetName ? `Waiting for ${targetName}` : 'Waiting for page'),
  scroll_page: 'Scrolling page',
  read_content: (targetName) => (targetName ? `Reading ${targetName}` : 'Reading content'),
  add_annotation: 'Adding annotation',
  thinking: 'Thinking',
  idle: null,
  departing: null,
}

function labelForKey(
  labelKey: PresenceLabelKey | null,
  targetName?: string | null,
): string | null {
  if (!labelKey) return null
  const label = PRESENCE_LABELS[labelKey] ?? null
  return typeof label === 'function' ? label(targetName) : label
}

function applyHint(baseLabel: string | null, hint?: string | null, taskLabel?: string | null): string | null {
  const trimmedHint = typeof hint === 'string' ? hint.trim() : ''
  if (trimmedHint) {
    return baseLabel ? `${baseLabel}: ${trimmedHint}` : trimmedHint
  }
  const trimmedTask = typeof taskLabel === 'string' ? taskLabel.trim() : ''
  if (trimmedTask) {
    return baseLabel ? `${baseLabel}: ${trimmedTask}` : trimmedTask
  }
  return baseLabel
}

export function summarizePresenceCursor(
  cursor: Pick<AgentPresenceCursor, 'labelKey' | 'targetName' | 'surface' | 'labelHint' | 'taskLabel'>,
): string | null {
  const label = applyHint(labelForKey(cursor.labelKey, cursor.targetName), cursor.labelHint, cursor.taskLabel)
  if (!label) return null
  return cursor.surface === 'page' ? `${label} in page` : `${label} on canvas`
}
