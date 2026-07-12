/** Canonical list of presence-cursor label keys. `PresenceLabelKey` in
 *  `types.ts` and the runtime coercion set both derive from this array. */
export const PRESENCE_LABEL_KEYS = [
  'scan_workspace',
  'find_placement',
  'create_page',
  'select_page',
  'attach_page',
  'inspect_page',
  'find_target',
  'click_target',
  'point_target',
  'type_text',
  'select_option',
  'wait_page',
  'scroll_page',
  'read_content',
  'add_annotation',
  'thinking',
  'idle',
  'departing',
] as const
