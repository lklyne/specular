// ADR 0008 §7 — file selection popup.

import { useEffect, useState } from 'react'
import { toggleBulletList, toggleWrap } from '../shared/markdown/markdown-commands'
import type { CanvasSceneFileEntity, LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { CanvasItemPopup } from './CanvasItemPopup'
import { EditorFormattingButtons } from './EditorFormattingButtons'
import { InlineEditLabel } from '../shared/InlineEditLabel'
import { fileDisplayName } from '../canvas-bg/entityConstants'
import { useActiveTextEditor } from './textEditorBridge'
import { POPUP_OFFSET_Y, usePopupDelayedKey } from './usePopupDelayedKey'
import type { AnnotateHandler } from './annotationMath'

const toggleBold = toggleWrap('**')
const toggleStrikethrough = toggleWrap('~~')

/**
 * Bold/strikethrough/bullet-list toggles for the single selected .md file,
 * shown only while it's being edited — a file popup has no whole-note
 * fallback like StickyNotePopover's, since it doesn't hold the file's
 * content, only a reference to it.
 */
function FormattingSection({ fileId, isDark }: { fileId: string; isDark: boolean }) {
  const activeEditor = useActiveTextEditor()
  if (activeEditor?.entityId !== fileId) return null
  return (
    <EditorFormattingButtons
      format={activeEditor.format}
      isDark={isDark}
      onBold={() => activeEditor.exec(toggleBold)}
      onStrikethrough={() => activeEditor.exec(toggleStrikethrough)}
      onBulletList={() => activeEditor.exec(toggleBulletList)}
    />
  )
}

export function FilePopup({
  api,
  isDark,
  layout,
  selectedFiles,
  popupReady,
  onAnnotate,
}: {
  api: Pick<
    CanvasBgElectronAPI,
    'renameFileEntity' | 'focusSelection' | 'arrangeSelection'
  >
  isDark: boolean
  layout: LayoutUpdateData
  selectedFiles: CanvasSceneFileEntity[]
  popupReady: boolean
  onAnnotate: AnnotateHandler
}) {
  const count = selectedFiles.length
  const ids = selectedFiles.map((f) => f.id).join('|')
  const open = usePopupDelayedKey(ids, popupReady && count > 0)

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
        {single ? (
          <>
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
                titleClassName="min-w-0 flex-1 truncate pl-1.5 text-xs font-medium"
                onTitleClick={() => setIsRenaming(true)}
              />
            </CanvasItemPopup.Section>
            <CanvasItemPopup.Divider isDark={isDark} />
            <FormattingSection fileId={single.id} isDark={isDark} />
          </>
        ) : null}
        <CanvasItemPopup.EntityActions
          isDark={isDark}
          noun={noun}
          count={count}
          api={api}
          layout={layout}
          entityIds={entityIds}
          onAnnotate={onAnnotate}
        />
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.Root>
  )
}
