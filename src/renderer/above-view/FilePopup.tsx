// ADR 0008 §7 — file selection popup.

import type { ProjectedFileEntity, ProjectedLayoutData } from '../../shared/scene-projection'
import { useEffect, useState } from 'react'
import { Eye, EyeClosed, X } from 'lucide-react'
import {
  toggleBold,
  toggleBulletList,
  toggleStrikethrough,
} from '../shared/markdown/markdown-commands'
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

/** Name + formatting toggles for the one file the popup is about. */
function FileIdentitySections({
  file,
  isDark,
  isRenaming,
  setIsRenaming,
  onRename,
}: {
  file: ProjectedFileEntity
  isDark: boolean
  isRenaming: boolean
  setIsRenaming: (renaming: boolean) => void
  onRename: (id: string, next: string) => void
}) {
  return (
    <>
      <CanvasItemPopup.Section grow>
        <InlineEditLabel
          value={fileDisplayName(file.file)}
          isEditing={isRenaming}
          onStartEdit={() => setIsRenaming(true)}
          onCommit={(next) => {
            setIsRenaming(false)
            onRename(file.id, next)
          }}
          onCancel={() => setIsRenaming(false)}
          variant="canvas-chrome"
          isDark={isDark}
          titleClassName="min-w-0 flex-1 truncate pl-1.5 text-xs font-medium"
          onTitleClick={() => setIsRenaming(true)}
        />
      </CanvasItemPopup.Section>
      <CanvasItemPopup.Divider isDark={isDark} />
      {file.rendererTag === 'markdown' ? (
        <FormattingSection fileId={file.id} isDark={isDark} />
      ) : null}
    </>
  )
}

/**
 * The eye and the exit are the whole action row while focused. Arrange and
 * annotate act on a canvas selection, which the session doesn't have.
 */
function FocusBarActions({
  api,
  isDark,
  annotationsVisible,
}: {
  api: Pick<CanvasBgElectronAPI, 'restoreFocusCamera' | 'setFocusAnnotationsVisible'>
  isDark: boolean
  annotationsVisible: boolean
}) {
  const eyeLabel = annotationsVisible ? 'Hide other items' : 'Show other items'
  return (
    <CanvasItemPopup.Section>
      <CanvasItemPopup.IconButton
        isDark={isDark}
        title={eyeLabel}
        ariaLabel={eyeLabel}
        onClick={() => api.setFocusAnnotationsVisible(!annotationsVisible)}
      >
        {annotationsVisible ? <Eye size={14} /> : <EyeClosed size={14} />}
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
  )
}

/**
 * The focus bar belongs to the focused note regardless of selection, because
 * draw/placement tools clear or reassign selection mid-session and the bar must
 * stay pinned.
 */
function resolveFocusBar(
  focusedNoteEntity: ProjectedFileEntity | null,
  layout: ProjectedLayoutData,
): { entity: ProjectedFileEntity; annotationsVisible: boolean } | null {
  const session = layout.focusPresentation
  if (!focusedNoteEntity || session?.target.kind !== 'file') return null
  return { entity: focusedNoteEntity, annotationsVisible: session.annotationsVisible }
}

/** What the popup is about: the focused note if there is one, otherwise the
 *  file selection. */
type FilePopupSubject = {
  count: number
  /** Identity key for the show-delay and the rename reset. */
  ids: string
  isSingle: boolean
  single: ProjectedFileEntity | null
  entityIds: string[]
  noun: string
}

function describeSubject(
  focusedFile: ProjectedFileEntity | null,
  selectedFiles: ProjectedFileEntity[],
): FilePopupSubject {
  if (focusedFile) {
    return {
      count: 1,
      ids: focusedFile.id,
      isSingle: true,
      single: focusedFile,
      entityIds: [focusedFile.id],
      noun: 'file',
    }
  }
  const count = selectedFiles.length
  const isSingle = count === 1
  const single = isSingle ? selectedFiles[0]! : null
  return {
    count,
    ids: selectedFiles.map((f) => f.id).join('|'),
    isSingle,
    single,
    entityIds: single ? [single.id] : selectedFiles.map((f) => f.id),
    noun: isSingle ? 'file' : `${count} files`,
  }
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
  layout: ProjectedLayoutData
  selectedFiles: ProjectedFileEntity[]
  popupReady: boolean
  /** The note framed by a file-target focus session, or null. Non-null turns
   *  this popup into the focus bar. */
  focusedNoteEntity: ProjectedFileEntity | null
  onAnnotate: AnnotateHandler
}) {
  const focusBar = resolveFocusBar(focusedNoteEntity, layout)
  const isFocused = focusBar !== null
  const subject = describeSubject(focusBar?.entity ?? null, selectedFiles)

  // The show-delay is for transient selections (rubber-band, rapid clicks). The
  // focus bar is deliberate chrome, so skip it while focused — the session
  // auto-enters editing, which takes interaction out of 'idle' and would
  // otherwise close the bar the moment the note is being typed in.
  const delayedOpen = usePopupDelayedKey(subject.ids, popupReady && subject.count > 0)
  const open = isFocused || delayedOpen

  const [isRenaming, setIsRenaming] = useState(false)
  useEffect(() => {
    setIsRenaming(false)
  }, [subject.ids])

  if (subject.count === 0) return null

  return (
    <CanvasItemPopup.Root
      entityIds={subject.entityIds}
      layout={layout}
      open={open}
      // Pin to the viewport top only while focused; on leave, flip back to the
      // file-anchored placement immediately so the FLIP tween rides the camera
      // restore as one motion (durations match in restoreFocusCamera).
      placement={isFocused ? 'viewport-top' : 'above'}
      align={subject.isSingle ? 'stretch' : 'center'}
      offset={POPUP_OFFSET_Y}
    >
      <CanvasItemPopup.Frame isDark={isDark} flush={isFocused} fullWidth={isFocused}>
        {subject.single ? (
          <FileIdentitySections
            file={subject.single}
            isDark={isDark}
            isRenaming={isRenaming}
            setIsRenaming={setIsRenaming}
            onRename={(id, next) => api.renameFileEntity(id, next)}
          />
        ) : null}
        {focusBar ? (
          <FocusBarActions
            api={api}
            isDark={isDark}
            annotationsVisible={focusBar.annotationsVisible}
          />
        ) : (
          <CanvasItemPopup.EntityActions
            isDark={isDark}
            noun={subject.noun}
            count={subject.count}
            api={api}
            layout={layout}
            entityIds={subject.entityIds}
            onAnnotate={onAnnotate}
          />
        )}
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.Root>
  )
}
