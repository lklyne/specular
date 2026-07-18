import type {
  CanvasEntityKind,
  LeftSidebarBootstrapData,
  LeftSidebarData,
  SidebarSectionKey,
  ThemeData,
} from '../types'
import type { SelectionMutationMode } from '../selection-modifiers'

export interface LeftSidebarElectronAPI {
  revealPage: (
    pageId: string,
    selectionIds?: string[],
    mode?: SelectionMutationMode,
  ) => void
  openAnnotationThread: (annotationId: string) => void
  revealEntity: (
    entityId: string,
    entityKind: CanvasEntityKind,
    selectionIds?: string[],
    mode?: SelectionMutationMode,
  ) => void
  deleteEntity: (entityId: string, entityKind: CanvasEntityKind) => void
  deleteSelection: () => void
  revealGroup: (groupId: string) => void
  ungroupGroup: (groupId: string) => void
  selectTab: (tabId: string) => void
  createTab: () => void
  renameTab: (tabId: string, name: string) => Promise<boolean>
  renamePage: (pageId: string, name: string) => Promise<boolean>
  renameGroup: (groupId: string, name: string) => Promise<boolean>
  renameFileEntity: (entityId: string, name: string) => Promise<boolean>
  renameTextEntity: (entityId: string, name: string) => Promise<boolean>
  renameDrawingEntity: (entityId: string, name: string) => Promise<boolean>
  deleteTab: (tabId: string) => void
  reorderTab: (tabId: string, toIndex: number) => void
  reorderSidebarItem: (
    section: SidebarSectionKey,
    draggedId: string,
    anchorId: string | null,
    position: 'before' | 'after',
    parentId: string | null,
  ) => void
  deletePage: (pageId: string) => void
  setTextEditing: (active: boolean) => void
  getInitialData: () => Promise<LeftSidebarBootstrapData>
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
  onSidebarData: (callback: (data: LeftSidebarData) => void) => () => void
}
