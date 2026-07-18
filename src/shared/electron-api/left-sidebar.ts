import type {
  CanvasEntityKind,
  LeftSidebarBootstrapData,
  LeftSidebarData,
  SidebarSectionKey,
  ThemeData,
} from '../types'

export interface LeftSidebarElectronAPI {
  revealPage: (pageId: string) => void
  openAnnotationThread: (annotationId: string) => void
  revealEntity: (entityId: string, entityKind: CanvasEntityKind) => void
  deleteEntity: (entityId: string, entityKind: CanvasEntityKind) => void
  revealGroup: (groupId: string) => void
  ungroupGroup: (groupId: string) => void
  selectTab: (tabId: string) => void
  createTab: () => void
  renameTab: (tabId: string, name: string) => void
  renamePage: (pageId: string, name: string) => void
  renameGroup: (groupId: string, name: string) => void
  renameFileEntity: (entityId: string, name: string) => void
  renameTextEntity: (entityId: string, name: string) => void
  renameDrawingEntity: (entityId: string, name: string) => void
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
