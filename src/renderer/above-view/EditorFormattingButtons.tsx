// Bold/strikethrough/bullet-list toggles shared by every popover that edits
// markdown text inline (sticky notes, .md file notes).

import { Bold, List, Strikethrough } from 'lucide-react'
import type { StickyFormatState } from '../shared/markdown/markdown-format-state'
import { CanvasItemPopup } from './CanvasItemPopup'

export function EditorFormattingButtons({
  format,
  isDark,
  onBold,
  onStrikethrough,
  onBulletList,
}: {
  format: Pick<StickyFormatState, 'bold' | 'strikethrough' | 'bulletList'>
  isDark: boolean
  onBold: () => void
  onStrikethrough: () => void
  onBulletList: () => void
}) {
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
            onClick={onBold}
          >
            <Bold size={14} />
          </CanvasItemPopup.IconButton>
          <CanvasItemPopup.IconButton
            isDark={isDark}
            active={format.strikethrough}
            title="Strikethrough"
            ariaLabel="Strikethrough"
            onClick={onStrikethrough}
          >
            <Strikethrough size={14} />
          </CanvasItemPopup.IconButton>
          <CanvasItemPopup.IconButton
            isDark={isDark}
            active={format.bulletList}
            title="Bullet list"
            ariaLabel="Bullet list"
            onClick={onBulletList}
          >
            <List size={14} />
          </CanvasItemPopup.IconButton>
        </CanvasItemPopup.Section>
      </div>
      <CanvasItemPopup.Divider isDark={isDark} />
    </>
  )
}
