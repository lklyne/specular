// ADR 0008 §4 — text selection popup. Plain and sticky count as same kind
// for color so color edits apply uniformly across both in multi-select.

import { slotForStorage } from '../../shared/canvas-colors'
import {
  toggleBold,
  toggleBulletList,
  toggleStrikethrough,
} from '../shared/markdown/markdown-commands'
import {
  toggleWholeNoteBullets,
  toggleWholeNoteWrap,
  wholeNoteFormatState,
} from '../shared/markdown/whole-note-format'
import type { CanvasSceneTextEntity, LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { CanvasItemPopup } from './CanvasItemPopup'
import { ColorDropdown } from './ColorDropdown'
import { EditorFormattingButtons } from './EditorFormattingButtons'
import { useActiveTextEditor } from '../shared/markdown/text-editor-bridge'
import { TEXT_SIZE_DEFAULT, TextSizeDropdown } from './TextSizeDropdown'
import { POPUP_OFFSET_Y, sharedValue, usePopupDelayedKey } from './usePopupDelayedKey'
import type { AnnotateHandler } from './annotationMath'

function wholeNoteFallbackFormat(entities: CanvasSceneTextEntity[]) {
  const states = entities.map((e) => wholeNoteFormatState(e.text))
  return {
    bold: states.every((s) => s.bold),
    strikethrough: states.every((s) => s.strikethrough),
    bulletList: states.every((s) => s.bulletList),
  }
}

/**
 * Bold/strikethrough/bullet-list toggles. While a selected sticky is being
 * edited, commands dispatch into its live CodeMirror view (via
 * `textEditorBridge`) and reflect the cursor's format. Otherwise they
 * apply to the whole text of every selected entity, like color and size.
 */
function FormattingSection({
  api,
  isDark,
  selectedTextEntities,
}: {
  api: Pick<CanvasBgElectronAPI, 'updateEntity'>
  isDark: boolean
  selectedTextEntities: CanvasSceneTextEntity[]
}) {
  const activeEditor = useActiveTextEditor()
  const editor =
    selectedTextEntities.length === 1 &&
    activeEditor?.entityId === selectedTextEntities[0].id
      ? activeEditor
      : null

  // The whole-note fallback re-derives from full note text, so only pay for
  // it when there is no live editor (whose cursor moves re-render this).
  const format = editor ? editor.format : wholeNoteFallbackFormat(selectedTextEntities)

  const applyToNotes = (transform: (text: string) => string) => {
    for (const e of selectedTextEntities) {
      api.updateEntity('text', e.id, { text: transform(e.text) })
    }
  }
  const onBold = editor
    ? () => editor.exec(toggleBold)
    : () => applyToNotes((text) => toggleWholeNoteWrap(text, '**'))
  const onStrikethrough = editor
    ? () => editor.exec(toggleStrikethrough)
    : () => applyToNotes((text) => toggleWholeNoteWrap(text, '~~'))
  const onBulletList = editor
    ? () => editor.exec(toggleBulletList)
    : () => applyToNotes(toggleWholeNoteBullets)

  return (
    <EditorFormattingButtons
      format={format}
      isDark={isDark}
      onBold={onBold}
      onStrikethrough={onStrikethrough}
      onBulletList={onBulletList}
    />
  )
}

export function StickyNotePopover({
  api,
  isDark,
  layout,
  selectedTextEntities,
  popupReady,
  onAnnotate,
}: {
  api: Pick<
    CanvasBgElectronAPI,
    | 'updateEntity'
    | 'focusSelection'
    | 'arrangeSelection'
  >
  isDark: boolean
  layout: LayoutUpdateData
  selectedTextEntities: CanvasSceneTextEntity[]
  popupReady: boolean
  onAnnotate: AnnotateHandler
}) {
  const count = selectedTextEntities.length
  const ids = selectedTextEntities.map((e) => e.id).join('|')
  const open = usePopupDelayedKey(ids, popupReady && count > 0)
  if (count === 0) return null

  const sharedColor = sharedValue(selectedTextEntities.map((e) => e.color))
  const activeSlot = slotForStorage(sharedColor)
  const sharedTextSize = sharedValue(
    selectedTextEntities.map((e) => e.textSize ?? TEXT_SIZE_DEFAULT),
  )

  const entityIds = selectedTextEntities.map((e) => e.id)
  const noun = count === 1 ? 'sticky note' : `${count} text entities`

  return (
    <CanvasItemPopup.Root
      entityIds={entityIds}
      layout={layout}
      open={open}
      placement="above"
      offset={POPUP_OFFSET_Y}
    >
      <CanvasItemPopup.Frame isDark={isDark}>
        <CanvasItemPopup.Section>
          <TextSizeDropdown
            isDark={isDark}
            value={sharedTextSize ?? TEXT_SIZE_DEFAULT}
            ariaLabel={`Set ${noun} text size`}
            onPick={(size) => {
              for (const e of selectedTextEntities) {
                api.updateEntity('text', e.id, { textSize: size })
              }
            }}
          />
        </CanvasItemPopup.Section>
        <CanvasItemPopup.Divider isDark={isDark} />
        <ColorDropdown
          isDark={isDark}
          palette="soft"
          activeSlot={activeSlot}
          role="fill"
          noun={noun}
          onPick={(storage) => {
            for (const e of selectedTextEntities) {
              api.updateEntity('text', e.id, { color: storage })
            }
          }}
        />
        <CanvasItemPopup.Divider isDark={isDark} />
        <FormattingSection api={api} isDark={isDark} selectedTextEntities={selectedTextEntities} />
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
