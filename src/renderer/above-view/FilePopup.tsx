// ADR 0008 §7 — file selection popup.

import { useEffect, useState } from 'react'
import { Eye, EyeClosed, X } from 'lucide-react'
import {
  toggleBold,
  toggleBulletList,
  toggleStrikethrough,
} from '../shared/markdown/markdown-commands'
import type { CanvasSceneFileEntity, LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { CanvasItemPopup } from './CanvasItemPopup'
import { EditorFormattingButtons } from './EditorFormattingButtons'
import { InlineEditLabel } from '../shared/InlineEditLabel'
import { fileDisplayName } from '../canvas-bg/entityConstants'
import { useActiveTextEditor } from '../shared/markdown/text-editor-bridge'
import { POPUP_OFFSET_Y, usePopupDelayedKey } from './usePopupDelayedKey'
import type { AnnotateHandler } from './annotationMath'

const INACTIVE_FORMAT = { bold: false, strikethrough: false, bulletList: false }

/**
 * Bold/strikethrough/bullet-list toggles for the single selected .md file.
 * Always rendered so entering edit mode causes no layout shift, but only
 * enabled while the file is being edited — a file popup has no whole-note
 * fallback like StickyNotePopover's, since it doesn't hold the file's
 * content, only a reference to it.
 */
function FormattingSection({ fileId, isDark }: { fileId: string; isDark: boolean }) {
  const activeEditor = useActiveTextEditor()
  const editor = activeEditor?.entityId === fileId ? activeEditor : null
  return (
    <EditorFormattingButtons
      format={editor ? editor.format : INACTIVE_FORMAT}
      isDark={isDark}
      disabled={!editor}
      onBold={() => editor?.exec(toggleBold)}
      onStrikethrough={() => editor?.exec(toggleStrikethrough)}
      onBulletList={() => editor?.exec(toggleBulletList)}
    />
  )
}

export function FilePopup({
  api,
  isDark,
  layout,
  selectedFiles,
  popupReady,
  focusedNoteEntity,
  onAnnotate,
}: {
  api: Pick<
    CanvasBgElectronAPI,
    | 'renameFileEntity'
    | 'focusSelection'
    | 'arrangeSelection'
    | 'restoreFocusCamera'
    | 'setFocusAnnotationsVisible'
  >
  isDark: boolean
  layout: LayoutUpdateData
  selectedFiles: CanvasSceneFileEntity[]
  popupReady: boolean
  /** The note framed by a file-target focus session, or null. Non-null turns
   *  this popup into the focus bar: it belongs to the focused note regardless of
   *  selection, because draw/placement tools clear or reassign selection
   *  mid-session and the bar must stay pinned. */
  focusedNoteEntity: CanvasSceneFileEntity | null
  onAnnotate: AnnotateHandler
}) {
  const focusBar =
    focusedNoteEntity && layout.focusPresentation?.target.kind === 'file'
      ? { entity: focusedNoteEntity, session: layout.focusPresentation }
      : null

  const count = focusBar ? 1 : selectedFiles.length
  const ids = focusBar ? focusBar.entity.id : selectedFiles.map((f) => f.id).join('|')
  // The show-delay is for transient selections (rubber-band, rapid clicks). The
  // focus bar is deliberate chrome, so skip it while focused — the session
  // auto-enters editing, which takes interaction out of 'idle' and would
  // otherwise close the bar the moment the note is being typed in.
  const delayedOpen = usePopupDelayedKey(ids, popupReady && count > 0)
  const open = focusBar !== null || delayedOpen

  const [isRenaming, setIsRenaming] = useState(false)
  useEffect(() => {
    setIsRenaming(false)
  }, [ids])

  if (count === 0) return null
  const isSingle = count === 1
  const single = focusBar?.entity ?? (isSingle ? selectedFiles[0] : null)
  const entityIds = single ? [single.id] : selectedFiles.map((f) => f.id)
  const noun = isSingle ? 'file' : `${count} files`

  return (
    <CanvasItemPopup.Root
      entityIds={entityIds}
      layout={layout}
      open={open}
      // Pin to the viewport top only while focused; on leave, flip back to the
      // file-anchored placement immediately so the FLIP tween rides the camera
      // restore as one motion (durations match in restoreFocusCamera).
      placement={focusBar ? 'viewport-top' : 'above'}
      align={isSingle ? 'stretch' : 'center'}
      offset={POPUP_OFFSET_Y}
    >
      <CanvasItemPopup.Frame isDark={isDark} flush={focusBar !== null} fullWidth={focusBar !== null}>
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
            {single.rendererTag === 'markdown' ? (
              <FormattingSection fileId={single.id} isDark={isDark} />
            ) : null}
          </>
        ) : null}
        {focusBar ? (
          // Focused: the eye and the exit are the whole action row. Arrange and
          // annotate act on a canvas selection, which the session doesn't have.
          <CanvasItemPopup.Section>
            <CanvasItemPopup.IconButton
              isDark={isDark}
              title={focusBar.session.annotationsVisible ? 'Hide other items' : 'Show other items'}
              ariaLabel={
                focusBar.session.annotationsVisible ? 'Hide other items' : 'Show other items'
              }
              onClick={() =>
                api.setFocusAnnotationsVisible(!focusBar.session.annotationsVisible)
              }
            >
              {focusBar.session.annotationsVisible ? <Eye size={14} /> : <EyeClosed size={14} />}
            </CanvasItemPopup.IconButton>
            <CanvasItemPopup.IconButton
              isDark={isDark}
              title="Exit focus"
              ariaLabel="Exit focus"
              onClick={() => api.restoreFocusCamera()}
            >
              <X size={14} />
            </CanvasItemPopup.IconButton>
          </CanvasItemPopup.Section>
        ) : (
          <CanvasItemPopup.EntityActions
            isDark={isDark}
            noun={noun}
            count={count}
            api={api}
            layout={layout}
            entityIds={entityIds}
            onAnnotate={onAnnotate}
          />
        )}
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.Root>
  )
}
