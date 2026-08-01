import type { EditorState } from '@codemirror/state'
import type { EditorView } from '@codemirror/view'
import type { CanvasSceneFileEntity } from '../../../shared/types'
import { MarkdownEditor } from '../../shared/MarkdownEditor'
import { useNoteContent } from './useNoteContent'

export function MarkdownInlineRenderer({
  entity,
  canEdit,
  isDark,
  onTextEditingChange,
  onOpenLink,
  onViewReady,
  onSelectionChange,
}: {
  entity: CanvasSceneFileEntity
  canEdit: boolean
  isDark: boolean
  onTextEditingChange: (active: boolean) => void
  onOpenLink: (id: string, url: string) => void
  /** Publishes the live CodeMirror view to `textEditorBridge` (above-view)
   *  while editing, so the .md file popover can render formatting toggles.
   *  Threaded down from FileBodyLayer — this file lives under canvas-bg/
   *  and must not import above-view directly. */
  onViewReady?: (view: EditorView | null) => void
  onSelectionChange?: (state: EditorState) => void
}) {
  const { mdContent, localText, handleChange, handleFocus, handleBlur } = useNoteContent(
    entity,
    canEdit,
    onTextEditingChange,
  )

  const textColor = isDark ? '#e7e5e4' : '#1c1917'

  if (!canEdit && mdContent === null) {
    return (
      <div style={{ width: '100%', height: '100%', overflow: 'auto', padding: 12 }}>
        <span style={{ opacity: 0.4, fontSize: 14, color: textColor, fontFamily: 'system-ui, sans-serif' }}>
          Loading...
        </span>
      </div>
    )
  }

  return (
    <div style={{ width: '100%', height: '100%', overflow: 'hidden', padding: 12 }}>
      <MarkdownEditor
        key={canEdit ? 'edit' : 'view'}
        readOnly={!canEdit}
        value={canEdit ? localText : (mdContent ?? '')}
        onChange={handleChange}
        onFocus={handleFocus}
        onBlur={handleBlur}
        onOpenLink={(url) => onOpenLink(entity.id, url)}
        onViewReady={onViewReady}
        onSelectionChange={onSelectionChange}
        isDark={isDark}
        autoFocus={canEdit}
        placeholder="Write your note..."
        style={{
          width: '100%',
          height: '100%',
          fontSize: 14,
          lineHeight: 1.5,
          color: textColor,
        }}
      />
    </div>
  )
}
