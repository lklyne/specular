export function filePathToSrc(filePath: string): string {
  if (
    filePath.startsWith('local-file://') ||
    filePath.startsWith('http://') ||
    filePath.startsWith('https://')
  ) {
    return filePath
  }
  return `local-file://${filePath}`
}

export interface RendererFileApi {
  showFileInFinder: (path: string) => void
  reorderStack: (
    action: 'bring-forward' | 'send-backward' | 'bring-to-front' | 'send-to-back',
    targetId?: string,
  ) => void
  readNoteFile: (path: string) => Promise<string | null>
  writeNoteFile: (path: string, content: string) => Promise<boolean>
  applyWireframeContent: (entityId: string, content: string) => Promise<boolean>
  setWireframeSelection: (
    entityId: string,
    node: ({ id: string; type: string } & Record<string, unknown>) | null,
  ) => void
  renameNoteFile: (path: string, newName: string) => Promise<string | null>
}

export function getFileApi(): RendererFileApi {
  return (window as unknown as { electronAPI: RendererFileApi }).electronAPI
}
