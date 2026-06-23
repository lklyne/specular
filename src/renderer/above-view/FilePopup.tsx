// ADR 0008 §7 — file selection popup. Per-renderer contributions come from
// `entity.popupContributions` via the plugin contribution surface.

import { useEffect, useState } from 'react'
import type {
  CanvasBgElectronAPI,
  CanvasSceneFileEntity,
  LayoutUpdateData,
} from '../../shared/types'
import { CanvasItemPopup } from './CanvasItemPopup'
import { InlineEditLabel } from '../shared/InlineEditLabel'
import { fileDisplayName } from '../canvas-bg/entityConstants'
import { renderPopupContributions } from './file-popup-contributions'
import { POPUP_OFFSET_Y, usePopupDelayedKey } from './usePopupDelayedKey'

export function FilePopup({
  api,
  isDark,
  layout,
  selectedFiles,
  interactionIdle,
  fileJsonModeMap,
  setFileJsonMode,
}: {
  api: Pick<
    CanvasBgElectronAPI,
    | 'renameFileEntity'
    | 'duplicateFileEntity'
    | 'deleteFileEntity'
    | 'writeNoteFile'
    | 'setFileDeviceOrientation'
    | 'toggleFileDeviceShell'
    | 'morphTextFile'
    | 'distributeSelection'
  >
  isDark: boolean
  layout: LayoutUpdateData
  selectedFiles: CanvasSceneFileEntity[]
  interactionIdle: boolean
  fileJsonModeMap: ReadonlyMap<string, boolean>
  setFileJsonMode: (entityId: string, jsonMode: boolean) => void
}) {
  const count = selectedFiles.length
  const ids = selectedFiles.map((f) => f.id).join('|')
  const open = usePopupDelayedKey(ids, interactionIdle && count > 0)

  const [isRenaming, setIsRenaming] = useState(false)
  useEffect(() => {
    setIsRenaming(false)
  }, [ids])

  if (count === 0) return null
  const isSingle = count === 1
  const single = isSingle ? selectedFiles[0] : null
  const entityIds = selectedFiles.map((f) => f.id)
  const noun = isSingle ? 'file' : `${count} files`

  return (
    <CanvasItemPopup.Root
      entityIds={entityIds}
      layout={layout}
      open={open}
      placement="above"
      align={isSingle ? 'stretch' : 'center'}
      offset={POPUP_OFFSET_Y}
    >
      <CanvasItemPopup.Frame isDark={isDark}>
        {single
          ? (() => {
              const contributions = renderPopupContributions(single, {
                api,
                isDark,
                jsonMode: fileJsonModeMap.get(single.id) ?? false,
                onJsonModeChange: setFileJsonMode,
              })
              return (
                <>
                  {contributions.length > 0 ? (
                    <>
                      {contributions}
                      <CanvasItemPopup.Divider isDark={isDark} />
                    </>
                  ) : null}
                  <CanvasItemPopup.Section grow>
                    <InlineEditLabel
                      value={fileDisplayName(single.file)}
                      isEditing={isRenaming}
                      onStartEdit={() => setIsRenaming(true)}
                      onCommit={(next) => {
                        setIsRenaming(false)
                        api.renameFileEntity(single.id, next)
                      }}
                      onCancel={() => setIsRenaming(false)}
                      variant="canvas-chrome"
                      isDark={isDark}
                      titleClassName="min-w-0 flex-1 truncate text-xs font-medium"
                      onTitleClick={() => setIsRenaming(true)}
                    />
                  </CanvasItemPopup.Section>
                  <CanvasItemPopup.Divider isDark={isDark} />
                </>
              )
            })()
          : null}
        <CanvasItemPopup.EntityActions
          isDark={isDark}
          noun={noun}
          count={count}
          onDuplicate={() => {
            for (const f of selectedFiles) api.duplicateFileEntity(f.id)
          }}
          onDelete={() => {
            for (const f of selectedFiles) api.deleteFileEntity(f.id)
          }}
          api={api}
        />
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.Root>
  )
}
