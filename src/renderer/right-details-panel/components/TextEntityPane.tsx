import { Copy, StickyNote, Trash2 } from 'lucide-react'
import type { PanelTextEntityDetail } from '../../../shared/types'
import { mutedClass, paneActionBtnClass, paneDeleteBtnClass } from '../rightDetailsPanelHelpers'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'
import { usePaneTheme } from '../PaneContext'
import { PaneField } from './PaneSection'
import { ColorSwatchPicker } from './ColorSwatchPicker'
import { PaneHeader } from './PaneHeader'

export function TextEntityPane({ textEntity }: { textEntity: PanelTextEntityDetail }) {
  const isDark = usePaneTheme()
  const muted = mutedClass(isDark)

  return (
    <div className="flex flex-col">
      <PaneHeader
        icon={<StickyNote size={14} className="shrink-0 text-zinc-500" />}
        label={textEntity.text.slice(0, 40) || 'Text'}
        actions={
          <>
            <button
              type="button"
              className={paneActionBtnClass(isDark)}
              onClick={() => rightDetailsPanelApi.duplicateTextEntity(textEntity.id)}
              title="Duplicate"
              aria-label="Duplicate Text"
            >
              <Copy size={14} />
            </button>
            <button
              type="button"
              className={paneDeleteBtnClass(isDark)}
              onClick={() => rightDetailsPanelApi.deleteTextEntity(textEntity.id)}
              title="Delete"
              aria-label="Delete Text"
            >
              <Trash2 size={14} />
            </button>
          </>
        }
      />

      <div className="px-2 pt-2 pb-2">
        <ColorSwatchPicker
          activeColor={textEntity.color}
          isDark={isDark}
          palette="vivid"
          onSelectColor={(color) => rightDetailsPanelApi.updateTextEntity(textEntity.id, { color })}
        />
      </div>

      <PaneField label="Content">
        <div
          className={`rounded px-2 py-1.5 text-[11px] leading-5 ${isDark ? 'bg-zinc-800' : 'bg-zinc-100'}`}
        >
          {textEntity.text || <span className={muted}>(empty)</span>}
        </div>
      </PaneField>
    </div>
  )
}
