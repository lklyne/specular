import Markdown from 'react-markdown'
import type { CanvasSceneFileEntity } from '../../../shared/types'
import { MarkdownEditor } from '../../shared/MarkdownEditor'
import { useNoteContent } from './useNoteContent'

function renderMarkdownBody(mdContent: string | null) {
  if (mdContent == null) return <span style={{ opacity: 0.4 }}>Loading...</span>
  if (mdContent === '') return <span style={{ opacity: 0.4 }}>Write your note...</span>
  return <Markdown>{mdContent}</Markdown>
}

export function MarkdownInlineRenderer({
  entity,
  canEdit,
  isDark,
  onTextEditingChange,
}: {
  entity: CanvasSceneFileEntity
  canEdit: boolean
  isDark: boolean
  onTextEditingChange: (active: boolean) => void
}) {
  const { mdContent, localText, handleChange, handleFocus, handleBlur } = useNoteContent(
    entity,
    canEdit,
    onTextEditingChange,
  )

  const textColor = isDark ? '#e7e5e4' : '#1c1917'

  return (
    <div style={{ width: '100%', height: '100%', overflow: 'auto', padding: 12 }}>
      {canEdit ? (
        <MarkdownEditor
          value={localText}
          onChange={handleChange}
          onFocus={handleFocus}
          onBlur={handleBlur}
          isDark={isDark}
          autoFocus
          style={{
            width: '100%',
            height: '100%',
            fontSize: 14,
            lineHeight: 1.5,
            color: textColor,
          }}
        />
      ) : (
        <div
          className="text-block-markdown"
          style={{
            fontSize: 14,
            color: textColor,
            fontFamily: 'system-ui, sans-serif',
            wordBreak: 'break-word',
          }}
        >
          {renderMarkdownBody(mdContent)}
        </div>
      )}
    </div>
  )
}
