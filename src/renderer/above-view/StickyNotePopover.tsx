// ADR 0008 §4 — text selection popup. Plain and sticky count as same kind
// for color so color edits apply uniformly across both in multi-select.

import { Bold, List, Strikethrough } from 'lucide-react'
import { slotForStorage } from '../../shared/canvas-colors'
import { toggleBulletList, toggleWrap } from '../shared/markdown/markdown-commands'
import type { CanvasSceneTextEntity, LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { CanvasItemPopup } from './CanvasItemPopup'
import { ColorDropdown } from './ColorDropdown'
import { useActiveStickyEditor } from './stickyEditorBridge'
import { TEXT_SIZE_DEFAULT, TextSizeDropdown } from './TextSizeDropdown'
import { POPUP_OFFSET_Y, sharedValue, usePopupDelayedKey } from './usePopupDelayedKey'
import type { AnnotateHandler } from './annotationMath'

const toggleBold = toggleWrap('**')
const toggleStrikethrough = toggleWrap('~~')

/**
 * Bold/strikethrough/bullet-list toggles for the sticky currently being
 * edited. Only rendered while exactly one text entity is selected and it's
 * the one live in `stickyEditorBridge` — otherwise there's no CodeMirror
 * view to dispatch commands into.
 */
function FormattingSection({
  isDark,
  selectedTextEntities,
}: {
  isDark: boolean
  selectedTextEntities: CanvasSceneTextEntity[]
}) {
  const activeEditor = useActiveStickyEditor()
  if (selectedTextEntities.length !== 1) return null
  if (!activeEditor || activeEditor.entityId !== selectedTextEntities[0].id) return null
  const { format, exec } = activeEditor
  return (
    <>
      {/* Pressing a toggle must not blur the editor — a blur commits the
          deferred blur handler and exits edit mode before the click's
          command can run against the live selection. */}
      <div onMouseDown={(e) => e.preventDefault()}>
        <CanvasItemPopup.Section>
          <CanvasItemPopup.IconButton
            isDark={isDark}
            active={format.bold}
            title="Bold"
            ariaLabel="Bold"
            onClick={() => exec(toggleBold)}
          >
            <Bold size={14} />
          </CanvasItemPopup.IconButton>
          <CanvasItemPopup.IconButton
            isDark={isDark}
            active={format.strikethrough}
            title="Strikethrough"
            ariaLabel="Strikethrough"
            onClick={() => exec(toggleStrikethrough)}
          >
            <Strikethrough size={14} />
          </CanvasItemPopup.IconButton>
          <CanvasItemPopup.IconButton
            isDark={isDark}
            active={format.bulletList}
            title="Bullet list"
            ariaLabel="Bullet list"
            onClick={() => exec(toggleBulletList)}
          >
            <List size={14} />
          </CanvasItemPopup.IconButton>
        </CanvasItemPopup.Section>
      </div>
      <CanvasItemPopup.Divider isDark={isDark} />
    </>
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
        <FormattingSection isDark={isDark} selectedTextEntities={selectedTextEntities} />
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
