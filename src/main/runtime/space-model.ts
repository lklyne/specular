/**
 * Workspace Model
 *
 * Owns the workspace data collections: groups, edges, annotations, and tabs.
 * These are the persisted, undoable workspace state. Pages (pages) remain
 * in runtime-context.ts because they hold non-serializable WebContentsView refs.
 *
 * The Y.Doc in space-doc.ts mirrors this data for undo/redo.
 * The diff-sync in space-observers.ts keeps them in sync.
 */

import type {
  Annotation,
  PersistedWorkspaceTab,
  WorkspaceEdge,
  WorkspaceGroup,
} from '../../shared/types'

export const workspaceAnnotations: Annotation[] = []
export const workspaceGroups: WorkspaceGroup[] = []
export const workspaceEdges: WorkspaceEdge[] = []
export const spaceTabs: PersistedWorkspaceTab[] = []
export let activeSpaceTabId: string | null = null

export function setActiveSpaceTabId(value: string | null): void {
  activeSpaceTabId = value
}
