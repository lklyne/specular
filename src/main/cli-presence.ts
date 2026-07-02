import { callApp, sessionId, getClientName } from './shared/app-client'
import type { PresenceLabelKey } from '../shared/types'

// ---------------------------------------------------------------------------
// Presence mapping for canvas verbs
// ---------------------------------------------------------------------------
// Browse verbs already emit presence via handleBrowse — this covers canvas ops.
// Typed against PresenceLabelKey so only keys the allowlist accepts can be
// added: verbs without a real label (create/update/delete/…) emit no presence.

const VERB_PRESENCE: Record<string, { labelKey: PresenceLabelKey; surface: string }> = {
  workspace:        { labelKey: 'scan_workspace', surface: 'canvas' },
  selection:        { labelKey: 'scan_workspace', surface: 'canvas' },
  'find-placement': { labelKey: 'find_placement', surface: 'canvas' },
  annotate:         { labelKey: 'add_annotation', surface: 'canvas' },
  annotations:      { labelKey: 'scan_workspace', surface: 'canvas' },
  annotation:       { labelKey: 'scan_workspace', surface: 'canvas' },
}

export function emitPresenceForVerb(verb: string): void {
  const entry = VERB_PRESENCE[verb]
  if (!entry) return
  // Fire-and-forget — don't block the command on presence
  callApp('/session/presence', {
    method: 'POST',
    body: JSON.stringify({
      sessionId,
      clientName: getClientName(),
      eventType: 'act',
      surface: entry.surface,
      phase: 'acting',
      labelKey: entry.labelKey,
    }),
  }).catch(() => {})
}
