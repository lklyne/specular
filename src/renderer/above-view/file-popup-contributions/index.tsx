// ADR 0008 §7 — renderer-side dispatch for file popup plugin contributions.
// To add a contribution: declare the tag in `PopupContributionTag`
// (src/shared/types.ts), declare it in the main-side plugin claim, and add a
// case in the switch below.

import type { ReactNode } from 'react'
import type { CanvasSceneFileEntity, PopupContributionTag } from '../../../shared/types'
import type { CanvasBgElectronAPI } from '../../../shared/electron-api/canvas-bg'
import { MarkdownMorphContribution } from './MarkdownMorphContribution'
import { WireframeThemeContribution } from './WireframeThemeContribution'
import { WireframeJsonModeContribution } from './WireframeJsonModeContribution'
import { WireframeDeviceControlsContribution } from './WireframeDeviceControlsContribution'

export interface FilePopupContributionCtx {
  api: Pick<
    CanvasBgElectronAPI,
    | 'writeNoteFile'
    | 'setFileDeviceOrientation'
    | 'toggleFileDeviceShell'
    | 'morphTextFile'
  >
  isDark: boolean
  jsonMode: boolean
  onJsonModeChange: (entityId: string, jsonMode: boolean) => void
}

export function renderPopupContributions(
  entity: CanvasSceneFileEntity,
  ctx: FilePopupContributionCtx,
): ReactNode[] {
  const tags: PopupContributionTag[] = entity.popupContributions ?? []
  return tags.map((tag) => renderOne(tag, entity, ctx))
}

function renderOne(
  tag: PopupContributionTag,
  entity: CanvasSceneFileEntity,
  ctx: FilePopupContributionCtx,
): ReactNode {
  switch (tag) {
    case 'wireframe-theme':
      return (
        <WireframeThemeContribution
          key={tag}
          api={ctx.api}
          isDark={ctx.isDark}
          entity={entity}
        />
      )
    case 'wireframe-json-mode':
      return (
        <WireframeJsonModeContribution
          key={tag}
          isDark={ctx.isDark}
          entity={entity}
          jsonMode={ctx.jsonMode}
          onJsonModeChange={ctx.onJsonModeChange}
        />
      )
    case 'wireframe-device-controls':
      return (
        <WireframeDeviceControlsContribution
          key={tag}
          api={ctx.api}
          isDark={ctx.isDark}
          entity={entity}
        />
      )
    case 'markdown-morph-to-text':
      return (
        <MarkdownMorphContribution
          key={tag}
          api={ctx.api}
          isDark={ctx.isDark}
          entity={entity}
        />
      )
  }
}
