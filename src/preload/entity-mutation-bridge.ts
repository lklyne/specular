import { ipcRenderer } from 'electron'
import type { EntityUpdatePatchMap, UpdatableEntityKind } from '../shared/types'
import { ipcChannels } from '../shared/ipc-contract'

/**
 * Entity-mutation IPC entries shared verbatim by the canvas-bg and
 * right-details-panel bridges. Spread into each bridge's api object so the
 * wiring lives once; per-bridge extras (refreshFileEntity, drawing/shape
 * duplication, …) stay in their own files.
 */
export const entityMutationBridge = {
  updateEntity: <K extends UpdatableEntityKind>(
    kind: K,
    id: string,
    patch: EntityUpdatePatchMap[K],
  ) => ipcRenderer.send(ipcChannels.canvasUpdateEntity, { kind, id, patch }),
  duplicateTextEntity: (id: string) =>
    ipcRenderer.send(ipcChannels.canvasDuplicateTextEntity, { id }),
  deleteTextEntity: (id: string) =>
    ipcRenderer.send(ipcChannels.canvasDeleteTextEntity, { id }),
  duplicateFileEntity: (id: string) =>
    ipcRenderer.send(ipcChannels.canvasDuplicateFileEntity, { id }),
  deleteFileEntity: (id: string) =>
    ipcRenderer.send(ipcChannels.canvasDeleteFileEntity, { id }),
}
