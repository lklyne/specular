import { randomUUID } from 'crypto'
import type { WorkspaceGroup } from '../shared/types'
import { findPageById } from './runtime/page-runtime'
import { workspaceGroups } from './runtime/workspace-model'
import { markDirty } from './runtime/layout-dirty'
import { scheduleWorkspaceAutosave } from './runtime/workspace-autosave'

export function makeId(prefix: string): string {
  return `${prefix}_${randomUUID()}`
}

export function pageCurrentUrl(pageId: string | undefined): string | null {
  if (!pageId) return null
  const page = findPageById(pageId)
  if (!page) return null
  const currentUrl = page.pageView.webContents.getURL()
  return currentUrl || 'about:blank'
}

export function createGroup(group: WorkspaceGroup): WorkspaceGroup {
  const nextGroup: WorkspaceGroup = {
    ...group,
    metadata: cloneMetadata(group.metadata),
  }
  workspaceGroups.push(nextGroup)
  markDirty('canvas', 'sidebar')
  scheduleWorkspaceAutosave()
  return nextGroup
}

export function cloneMetadata(
  metadata: Record<string, unknown> | undefined,
): Record<string, unknown> | undefined {
  return metadata ? { ...metadata } : undefined
}
