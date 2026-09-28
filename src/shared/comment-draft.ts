import type { AnnotationCreateRequest, WorkspaceBounds } from './types'

/**
 * An unsaved comment, held in main between the canvas gesture that started it
 * and the sidebar composer that finishes it. One draft at a time — starting a
 * new one replaces whatever was open. The gesture decides the kind (mirroring
 * the anchor decision in ADR 0006); `label` is the chip text the composer
 * shows in place of a full anchor description.
 */
export type CommentDraft =
  | {
      id: string
      kind: 'element'
      /** `anchor.type: 'element'`; `metadata.inspectContext` carries the
       *  element snapshot the agent prompt reads. */
      request: AnnotationCreateRequest
      label: string
    }
  | {
      id: string
      kind: 'point'
      canvasX: number
      canvasY: number
      label: string
    }
  | {
      id: string
      kind: 'region'
      canvasRect: WorkspaceBounds
      label: string
    }
  | {
      id: string
      kind: 'selection'
      canvasRect: WorkspaceBounds
      entityIds: string[]
      label: string
    }
