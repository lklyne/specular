/**
 * Where a fix runs.
 *
 * A comment on a page served from a bound repo runs in that repo — the source
 * is the better thing to edit, so the binding wins whenever it exists. A
 * comment whose selection names a file entity instead runs in the folder that
 * holds the file (the user's space folder), so the agent can read the artifact
 * and place a copy beside it.
 */

import { dirname } from 'path'
import type { Annotation } from '../../shared/types'
import { annotationOrigin } from '../../shared/annotation-utils'

export type FixTarget =
  | { kind: 'repo'; cwd: string; origin: string; autoFix: boolean }
  | { kind: 'space-folder'; cwd: string; filePath: string }

/** Origin → repo binding lookup, injected so the resolution stays pure. */
export type OriginBindingLookup = (
  origin: string,
) => { repoPath: string; autoFix: boolean } | null

export function resolveFixTarget(
  annotation: Annotation,
  getOriginBinding: OriginBindingLookup,
): FixTarget | null {
  const origin = annotationOrigin(annotation)
  if (origin) {
    const binding = getOriginBinding(origin)
    if (binding) {
      return { kind: 'repo', cwd: binding.repoPath, origin, autoFix: binding.autoFix }
    }
  }
  const selectionTarget = annotation.metadata?.selectionTarget
  if (selectionTarget?.kind === 'file' && selectionTarget.filePath) {
    return {
      kind: 'space-folder',
      cwd: dirname(selectionTarget.filePath),
      filePath: selectionTarget.filePath,
    }
  }
  return null
}

/**
 * Key the in-flight tracker and the progress log group by. Page origins are the
 * Comments panel's grouping; a space-folder run takes a file-scheme key so it
 * can never collide with one.
 */
export function fixTargetKey(target: FixTarget): string {
  return target.kind === 'repo' ? target.origin : `file://${target.cwd}`
}
