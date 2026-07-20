/**
 * GroupRenameLabel — the group title's DOM presence in aboveView.
 *
 * The visible glyphs are painted by GroupLabelCanvasSurface in screen space
 * (crisp at any zoom), and pointer input routes through the shared hit-test's
 * `group-label` target in `useCanvasPointerRouter` (drag/select on press,
 * rename on double-click). What remains here is the furniture the canvas and
 * router can't provide:
 *
 *   - a transparent-text element sized like the label, for the grab cursor
 *     and the full-title tooltip on hover;
 *   - the rename input (ADR 0002 §2, `data-overlay-ui` so the router yields
 *     while editing), driven by the unified
 *     `canvas-{request,commit,cancel}-entity-edit` IPC pair — `isRenaming`
 *     derives from `editingEntityId === group.id`, never local state.
 */

import type { CanvasSceneGroupEntity, LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { InlineEditLabel } from '../shared/InlineEditLabel'

export function GroupRenameOverlay({
  api,
  layoutData,
  isDark,
  editingEntityId,
}: {
  api: CanvasBgElectronAPI
  layoutData: LayoutUpdateData
  isDark: boolean
  editingEntityId: string | null
}) {
  const groups = layoutData.groups ?? []
  if (!groups.length) return null
  return (
    <>
      {groups.map((group) => (
        <GroupRenameItem
          key={group.id}
          api={api}
          layoutData={layoutData}
          group={group}
          isDark={isDark}
          isRenaming={editingEntityId === group.id}
        />
      ))}
    </>
  )
}

function GroupRenameItem({
  api,
  layoutData,
  group,
  isDark,
  isRenaming,
}: {
  api: CanvasBgElectronAPI
  layoutData: LayoutUpdateData
  group: CanvasSceneGroupEntity
  isDark: boolean
  isRenaming: boolean
}) {
  // Text is visible only while renaming, when the InlineEditLabel input
  // inherits this color; at rest the canvas paints the glyphs.
  const labelColorClass = !isRenaming
    ? 'text-transparent'
    : group.color
      ? isDark ? 'text-zinc-100' : 'text-zinc-900'
      : isDark ? 'text-zinc-300' : 'text-zinc-700'
  // The label sits above group.screenY and inside aboveView's overlay-local
  // coordinate space; subtract canvasOrigin.y to drop into overlay coords.
  const left = group.screenX
  const top = group.screenY - layoutData.canvasOrigin.y

  return (
    <div
      data-overlay-ui={isRenaming ? true : undefined}
      className={`pointer-events-auto absolute select-none text-[11px] font-medium ${labelColorClass}`}
      style={{
        left,
        top,
        transform: 'translateY(-100%)',
        whiteSpace: 'nowrap',
        cursor: isRenaming ? 'text' : 'grab',
      }}
    >
      <span className="inline-flex items-center pb-1">
        <InlineEditLabel
          value={group.label}
          isEditing={isRenaming}
          onCommit={(next) => {
            api.renameGroup(group.id, next)
            api.commitEntityEdit()
          }}
          onCancel={() => api.cancelEntityEdit()}
          variant="canvas-chrome"
          isDark={isDark}
          titleClassName="whitespace-nowrap"
          inputClassName="min-w-[120px] border-0 bg-transparent text-[11px] font-medium outline-none focus:outline-none"
        />
      </span>
    </div>
  )
}
