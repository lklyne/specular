import type { Annotation, FixProgressEntry, LayoutUpdateData, WorkspaceBounds } from '../../shared/types'
import {
  canvasRectToScreenRect,
  pendingElementScreenRect,
  type AnnotationLiveBboxLookup,
  type PendingAnnotation,
} from './annotationMath'
import { Play } from 'lucide-react'
import { CircleCheckIcon, MoreVerticalIcon, TrashIcon } from '../shared/PanelIcons'
import { CommentBubble, CommentInput } from '../shared/CommentPrimitives'
import { FixEventList, fixStatusLabel } from '../shared/FixEventList'

const REGION_COMPOSER_WIDTH = 320
const REGION_COMPOSER_MARGIN = 12

/**
 * Single pending-annotation composer (ADR 0006). One component handles all
 * three anchor types — element, canvas-point, region — with placement keyed
 * off the anchor: above-right of the element bbox, adjacent to the click
 * point, or above-right of the region rect. Element/canvas-point drafts
 * arrive via `pendingAnnotation`; region drafts arrive via
 * `pendingRegionRect`. Only one is set at a time.
 *
 * `pendingPosition` is computed at render time by the caller — for element
 * pendings it consults the live bbox the page reports on scroll so the
 * composer follows page content (ADR 0006).
 */
export function PendingAnnotationComposer({
  clearDraft,
  commentInputRef,
  commentText,
  elementNameDraft,
  layoutData,
  pendingAnnotation,
  pendingPosition,
  pendingRegionRect,
  setCommentText,
  setElementNameDraft,
  submitPendingAnnotation,
  submitRegionAnnotation,
}: {
  clearDraft: () => void
  commentInputRef: React.RefObject<HTMLTextAreaElement | null>
  commentText: string
  elementNameDraft: string
  layoutData: LayoutUpdateData
  pendingAnnotation: PendingAnnotation | null
  pendingPosition: { left: number; top: number; width: number } | null
  pendingRegionRect: WorkspaceBounds | null
  setCommentText: React.Dispatch<React.SetStateAction<string>>
  setElementNameDraft: React.Dispatch<React.SetStateAction<string>>
  submitPendingAnnotation: () => void
  submitRegionAnnotation: () => void
}) {
  if (pendingAnnotation) {
    const left = pendingPosition?.left ?? pendingAnnotation.composerX
    const top = pendingPosition?.top ?? pendingAnnotation.composerY
    const width = pendingPosition?.width ?? pendingAnnotation.composerWidth
    const isElementAnchor = pendingAnnotation.request.anchor.type === 'element'
    return (
      <ComposerBox
        clearDraft={clearDraft}
        commentInputRef={commentInputRef}
        commentText={commentText}
        left={left}
        top={top}
        width={width}
        setCommentText={setCommentText}
        submit={submitPendingAnnotation}
        submitLabel="Submit comment"
        elementNameDraft={isElementAnchor ? elementNameDraft : undefined}
        setElementNameDraft={isElementAnchor ? setElementNameDraft : undefined}
      />
    )
  }
  if (pendingRegionRect) {
    const screen = canvasRectToScreenRect(layoutData, pendingRegionRect)
    const overlayTop = screen.top - layoutData.canvasOrigin.y
    const composerX = Math.min(
      Math.max(screen.left, 8),
      window.innerWidth - REGION_COMPOSER_WIDTH - 8,
    )
    const composerY = overlayTop + screen.height + REGION_COMPOSER_MARGIN
    return (
      <>
        <div
          className="pointer-events-none absolute rounded border-2 border-dashed border-blue-500/90 bg-blue-500/10"
          style={{ left: screen.left, top: overlayTop, width: screen.width, height: screen.height }}
        />
        <ComposerBox
          clearDraft={clearDraft}
          commentInputRef={commentInputRef}
          commentText={commentText}
          left={composerX}
          top={composerY}
          width={REGION_COMPOSER_WIDTH}
          setCommentText={setCommentText}
          submit={submitRegionAnnotation}
          submitLabel="Submit region annotation"
        />
      </>
    )
  }
  return null
}

function ComposerBox({
  clearDraft,
  commentInputRef,
  commentText,
  left,
  top,
  width,
  setCommentText,
  submit,
  submitLabel,
  elementNameDraft,
  setElementNameDraft,
}: {
  clearDraft: () => void
  commentInputRef: React.RefObject<HTMLTextAreaElement | null>
  commentText: string
  left: number
  top: number
  width: number
  setCommentText: React.Dispatch<React.SetStateAction<string>>
  submit: () => void
  submitLabel?: string
  elementNameDraft?: string
  setElementNameDraft?: React.Dispatch<React.SetStateAction<string>>
}) {
  const showElementName = elementNameDraft !== undefined && setElementNameDraft !== undefined
  return (
    <div
      className="pointer-events-auto absolute z-50"
      data-overlay-ui
      style={{ left, top, width }}
    >
      <div className="flex flex-col gap-1 rounded-[8px] border border-[var(--surface-popover-border)] bg-[var(--surface-popover-subtle)] p-1.5 shadow-lg">
        {showElementName ? (
          <input
            type="text"
            value={elementNameDraft}
            onChange={(event) => setElementNameDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Escape') { event.preventDefault(); clearDraft() }
              if (event.key === 'Enter') { event.preventDefault(); commentInputRef.current?.focus() }
              event.stopPropagation()
            }}
            placeholder="Element name"
            aria-label="Element name"
            className="w-full rounded-[6px] bg-transparent px-2 py-1 text-[12px] font-medium text-zinc-900 outline-none placeholder:text-zinc-400 dark:text-zinc-100 dark:placeholder:text-zinc-500"
          />
        ) : null}
        <div className="relative pl-1.5 pb-1.5">
          <CommentInput
            inputRef={commentInputRef}
            autoFocus={!showElementName}
            value={commentText}
            onChange={setCommentText}
            onSubmit={submit}
            submitLabel={submitLabel}
            onKeyDown={(event) => {
              if (event.key === 'Escape') { event.preventDefault(); clearDraft() }
              event.stopPropagation()
            }}
          />
        </div>
      </div>
    </div>
  )
}

/**
 * Outline drawn around the element targeted by a pending element-anchored
 * comment. Single-click element selection through the comment tool sets a
 * `pendingAnnotation`, which suppresses the page-paints hover preview — so
 * without this outline the user has no visual confirmation of what they
 * just selected. The region case keeps its outlines because each page
 * paints them from the held marquee rect.
 */
export function PendingElementOutline({
  pending,
  layoutData,
  liveBboxes,
}: {
  pending: PendingAnnotation | null
  layoutData: LayoutUpdateData
  liveBboxes: AnnotationLiveBboxLookup
}) {
  if (!pending) return null
  const rect = pendingElementScreenRect(pending, layoutData, liveBboxes)
  if (!rect) return null
  return (
    <div
      className="pointer-events-none absolute"
      style={{
        left: rect.left,
        top: rect.top,
        width: Math.max(1, rect.width),
        height: Math.max(1, rect.height),
        border: '1px dashed rgba(59, 130, 246, 0.95)',
        background: 'rgba(59, 130, 246, 0.14)',
        boxShadow: '0 0 0 1px rgba(255, 255, 255, 0.22) inset',
        boxSizing: 'border-box',
        zIndex: 40,
      }}
    />
  )
}

export function AnnotationThreadPopover({
  api,
  closeThread,
  drawCursor,
  drawInteractionEnabled,
  openThread,
  openThreadMenu,
  progress,
  replyText,
  setOpenThreadMenu,
  setReplyText,
  submitThreadReply,
  threadInputRef,
  threadPosition,
}: {
  api: {
    deleteAnnotation: (annotationId: string) => void
    resolveAnnotation: (annotationId: string) => void
    fixSingleAnnotation: (annotationId: string) => void
  }
  closeThread: () => void
  drawCursor: string
  drawInteractionEnabled: boolean
  openThread: Annotation | null
  openThreadMenu: boolean
  progress?: FixProgressEntry
  replyText: string
  setOpenThreadMenu: React.Dispatch<React.SetStateAction<boolean>>
  setReplyText: React.Dispatch<React.SetStateAction<string>>
  submitThreadReply: () => void
  threadInputRef: React.RefObject<HTMLTextAreaElement | null>
  threadPosition: { left: number; top: number; width: number } | null
}) {
  if (!openThread || !threadPosition) return null

  return (
    <>
      <div
        className="pointer-events-auto absolute inset-0 z-40"
        onPointerDown={(event) => {
          if (event.pointerType === 'mouse' && event.button !== 0) return
          closeThread()
        }}
      />
      <div
        className="pointer-events-auto absolute z-50"
        data-overlay-ui
        style={{
          left: threadPosition.left,
          top: threadPosition.top,
          width: threadPosition.width,
        }}
      >
        <div className="rounded-2xl border border-[var(--surface-popover-border)] bg-[var(--surface-popover-subtle)] text-zinc-900 shadow-xl dark:text-zinc-100">
          <div
            className="flex items-center justify-between border-b border-zinc-200 px-2.5 py-1.5 dark:border-zinc-700"
            style={{ cursor: drawInteractionEnabled ? drawCursor : undefined }}
          >
            <div className="text-[12px] font-semibold">Comment</div>
            <div className="flex items-center gap-1">
              <button
                type="button"
                data-overlay-ui
                className="flex h-7 w-7 items-center justify-center rounded-full text-zinc-500 hover:bg-[var(--surface-popover)] disabled:opacity-40 dark:text-zinc-300 dark:hover:bg-[var(--surface-popover)]"
                aria-label="Fix with agent"
                title="Fix with agent"
                disabled={progress?.status === 'running'}
                onClick={() => api.fixSingleAnnotation(openThread.id)}
              >
                <Play className="size-3.5" />
              </button>
              <div className="relative">
                <button
                  type="button"
                  data-overlay-ui
                  className="flex h-7 w-7 items-center justify-center rounded-full text-zinc-500 hover:bg-[var(--surface-popover)] dark:text-zinc-300 dark:hover:bg-[var(--surface-popover)]"
                  aria-label="More actions"
                  title="More actions"
                  onClick={() => setOpenThreadMenu((current) => !current)}
                >
                  <MoreVerticalIcon />
                </button>
                {openThreadMenu ? (
                  <div
                    data-overlay-ui
                    className="absolute right-0 top-8 z-[60] min-w-40 rounded-[10px] border border-[var(--surface-popover-border)] bg-[var(--surface-popover-subtle)] p-1 text-zinc-900 shadow-xl dark:text-zinc-100"
                  >
                    <button
                      type="button"
                      data-overlay-ui
                      className="flex w-full cursor-default items-center gap-2 rounded-[7px] px-2.5 py-1.5 text-left text-xs text-zinc-900 hover:bg-[var(--surface-popover)] dark:text-zinc-100 dark:hover:bg-[var(--surface-popover)]"
                      onClick={() => {
                        setOpenThreadMenu(false)
                        api.resolveAnnotation(openThread.id)
                        closeThread()
                      }}
                    >
                      <CircleCheckIcon className="size-3" />
                      <span>Resolve</span>
                    </button>
                    <button
                      type="button"
                      data-overlay-ui
                      className="flex w-full cursor-default items-center gap-2 rounded-[7px] px-2.5 py-1.5 text-left text-xs text-zinc-900 hover:bg-[var(--surface-popover)] dark:text-zinc-100 dark:hover:bg-[var(--surface-popover)]"
                      onClick={() => {
                        setOpenThreadMenu(false)
                        api.deleteAnnotation(openThread.id)
                        closeThread()
                      }}
                    >
                      <TrashIcon className="size-3" />
                      <span>Delete</span>
                    </button>
                  </div>
                ) : null}
              </div>
              <button
                type="button"
                className="flex h-7 w-7 items-center justify-center rounded-full text-zinc-500 hover:bg-[var(--surface-popover)] dark:text-zinc-300 dark:hover:bg-[var(--surface-popover)]"
                onClick={closeThread}
                aria-label="Close"
              >
                ×
              </button>
            </div>
          </div>
          {openThread.anchor.type === 'element' && openThread.elementName ? (
            <div className="border-b border-zinc-200 px-2.5 py-1.5 text-[12px] font-medium text-zinc-900 dark:border-zinc-700 dark:text-zinc-100">
              {openThread.elementName}
            </div>
          ) : null}
          <div className="max-h-[320px] space-y-2.5 overflow-auto px-2.5 py-2.5">
            <CommentBubble author={openThread.author} text={openThread.text} fallback="Drawing feedback" />
            {openThread.replies.map((reply, idx) => (
              <CommentBubble key={`${openThread.id}:reply:${idx}`} author={reply.author} text={reply.text} />
            ))}
          </div>
          {progress ? (
            <ThreadFixProgress progress={progress} />
          ) : null}
          <div className="border-t border-zinc-200 px-2.5 py-2.5 dark:border-zinc-700">
            <div className="relative rounded-[16px] border border-zinc-300 bg-zinc-50 py-1.5 pl-2.5 pr-1.5 dark:border-zinc-600 dark:bg-zinc-900/40">
              <CommentInput
                inputRef={threadInputRef}
                value={replyText}
                onChange={setReplyText}
                onSubmit={submitThreadReply}
                placeholder="Reply"
                submitLabel="Send reply"
                buttonClassName="bg-zinc-200 text-zinc-700 hover:bg-zinc-300 dark:bg-zinc-700 dark:text-zinc-100 dark:hover:bg-zinc-600"
              />
            </div>
          </div>
        </div>
      </div>
    </>
  )
}

function ThreadFixProgress({ progress }: { progress: FixProgressEntry }) {
  const eventCount = progress.events.length

  const statusLabel = fixStatusLabel(progress.status)

  const statusColor =
    progress.status === 'running'
      ? 'text-blue-600 dark:text-blue-400'
      : progress.status === 'failed'
        ? 'text-red-600 dark:text-red-400'
        : 'text-emerald-600 dark:text-emerald-400'

  return (
    <div className="border-t border-zinc-200 dark:border-zinc-700">
      <div className="flex items-center justify-between px-2.5 py-1.5 text-[11px]">
        <span className={`font-medium ${statusColor}`}>{statusLabel}</span>
        <span className="text-zinc-400 dark:text-zinc-500">
          {eventCount} event{eventCount === 1 ? '' : 's'}
        </span>
      </div>
      {eventCount > 0 ? (
        <FixEventList events={progress.events} className="max-h-[160px] px-2.5 pb-2" />
      ) : (
        <div className="px-2.5 pb-2 text-[11px] text-zinc-400 dark:text-zinc-500">
          Waiting for output…
        </div>
      )}
      {progress.error ? (
        <div className="border-t border-zinc-200 px-2.5 py-1.5 text-[11px] text-red-700 dark:border-zinc-700 dark:text-red-300">
          {progress.error}
        </div>
      ) : null}
    </div>
  )
}
