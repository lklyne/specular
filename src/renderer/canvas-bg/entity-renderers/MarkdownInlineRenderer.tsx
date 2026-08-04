import type { CanvasSceneFileEntity } from '../../../shared/types'
import { MarkdownEditor } from '../../shared/MarkdownEditor'
import { useEditorBridge } from '../../shared/markdown/text-editor-bridge'
import { useNoteContent } from './useNoteContent'

/** Reading padding on an ordinary canvas note card. */
export const NOTE_CONTENT_PADDING = '12px'

export function MarkdownInlineRenderer({
  entity,
  canEdit,
  isDark,
  contentPadding = NOTE_CONTENT_PADDING,
  onTextEditingChange,
  onOpenLink,
}: {
  entity: CanvasSceneFileEntity
  canEdit: boolean
  isDark: boolean
  /** Reading padding; lives inside the scroller so it scrolls with the text
   *  and leaves the scrollbar on the card's edge. Wider on the fullscreen
   *  focus card. */
  contentPadding?: string
  onTextEditingChange: (active: boolean) => void
  onOpenLink: (id: string, url: string) => void
}) {
  const { mdContent, loadError, localText, handleChange, handleFocus, handleBlur } = useNoteContent(
    entity,
    canEdit,
    onTextEditingChange,
  )
  // Publishes the live CodeMirror view while editing, so the .md file
  // popover can render formatting toggles.
  const editorBridge = useEditorBridge(entity.id, canEdit)

  const textColor = isDark ? '#e7e5e4' : '#1c1917'

  // A note that failed to load must say so. Reporting it as still loading
  // leaves the user waiting on a read that already finished.
  if (!canEdit && mdContent === null) {
    return (
      <div style={{ width: '100%', height: '100%', overflow: 'auto', padding: contentPadding }}>
        <span style={{ opacity: 0.4, fontSize: 14, color: textColor, fontFamily: 'system-ui, sans-serif' }}>
          {loadError ?? 'Loading…'}
        </span>
      </div>
    )
  }

  return (
    // No padding here: it lives on `.cm-content` (see `contentPadding`), so the
    // scroller spans the whole card.
    <div style={{ width: '100%', height: '100%', overflow: 'hidden' }}>
      <MarkdownEditor
        readOnly={!canEdit}
        value={localText}
        onChange={handleChange}
        onFocus={handleFocus}
        onBlur={handleBlur}
        onOpenLink={(url) => onOpenLink(entity.id, url)}
        onViewReady={editorBridge.onViewReady}
        onSelectionChange={editorBridge.onSelectionChange}
        isDark={isDark}
        autoFocus={canEdit}
        contentPadding={contentPadding}
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
