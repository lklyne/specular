import { Collapsible } from '@base-ui/react/collapsible'
import { PRIMARY_BUTTON_BORDERED_CLASS } from '../../shared/primaryButton'
import {
  ChevronDown,
  ChevronRight,
  Laptop,
  Loader2,
  Play,
  Smartphone,
  Tablet,
  Trash2,
  Wrench,
} from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import type {
  Annotation,
  DevtoolsPanelPageSummary,
  DevtoolsPanelSelectionSummary,
  FixProgressEntry,
  InspectPanelState,
  OriginBindings,
} from '../../../shared/types'
import {
  dividerClass,
  isUnresolved,
  mutedClass,
} from '../rightDetailsPanelHelpers'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'
import { usePaneTheme } from '../PaneContext'
import {
  buildUnresolvedCountsByNodeId,
  getInspectDetailState,
  groupAnnotationsByOrigin,
  resolvePageDimensions,
} from '../rightDetailsPanelSelectors'
import { useClearInspectHoverOnLeave } from '../useClearInspectHoverOnLeave'
import { useElementCommentDraft } from '../useElementCommentDraft'
import { useInspectTreeState } from '../useInspectTreeState'
import { CommentRow } from './CommentsPane'
import { ElementCommentComposer } from './ElementCommentComposer'
import { InspectDetailSection } from './InspectDetailSection'
import { InspectTree } from './InspectTree'
import { PaneHeader } from './PaneHeader'
import { InfoIcon } from '../../shared/PanelIcons'

export function PagePane({
  inspect,
  annotations,
  selection,
  pages,
  fixProgress,
  originBindings,
}: {
  inspect: InspectPanelState
  annotations: Annotation[]
  selection?: DevtoolsPanelSelectionSummary
  pages: DevtoolsPanelPageSummary[]
  fixProgress: Record<string, FixProgressEntry>
  originBindings: OriginBindings
}) {
  const isDark = usePaneTheme()
  const muted = mutedClass(isDark)
  const divider = dividerClass(isDark)
  const elementsSectionRef = useRef<HTMLElement>(null)
  const activePage = inspect.activePageId
    ? pages.find((page) => page.id === inspect.activePageId)
    : undefined
  const activePageDimensions = activePage ? resolvePageDimensions(activePage) : {}
  const { activeDetail, hoveredDetail, selectedDetail } = getInspectDetailState(inspect)
  const unresolvedCountsByNodeId = buildUnresolvedCountsByNodeId(
    annotations,
    inspect.activePageId,
  )
  const { expanded, registerNodeElement, setExpanded } = useInspectTreeState(inspect)
  const {
    commentInputRef,
    elementCommentText,
    hasElementComment,
    setElementCommentText,
    submitElementComment,
  } = useElementCommentDraft({
    activeDetail,
    selection,
  })

  const clearInspectListState = (clearHover: boolean) => {
    rightDetailsPanelApi.clearInspectSelection()
    if (clearHover && inspect.activePageId) {
      rightDetailsPanelApi.setInspectHoverNode(inspect.activePageId, null)
    }
  }

  useClearInspectHoverOnLeave(inspect.activePageId ?? null, inspect.selectedNodeId ?? null)

  const collapsiblePanelClass =
    'h-[var(--collapsible-panel-height)] overflow-hidden transition-all ease-out data-[ending-style]:h-0 data-[starting-style]:h-0 duration-150 [&[hidden]:not([hidden=\'until-found\'])]:hidden'

  return (
    <div
      className="flex h-full min-h-0 flex-col"
      onPointerDownCapture={(event) => {
        if (inspect.enabled) return
        if (!inspect.selectedNodeId && !inspect.hoveredNodeId) return
        const elementsSection = elementsSectionRef.current
        if (!elementsSection) return
        const target = event.target
        if (!(target instanceof Node)) return
        if (elementsSection.contains(target)) return
        clearInspectListState(true)
      }}
    >
      <div className="thin-scrollbar min-h-0 flex-1 overflow-auto [&>section:first-of-type]:border-t-0">
        {/* Page header */}
        {inspect.activePageId ? (
          <PaneHeader
            icon={<PageFavicon faviconUrl={activePage?.faviconUrl} label={activePage?.label} width={activePageDimensions.width} />}
            label={activePage?.label ?? 'Page'}
            actions={
              <PageHeaderActions
                pageId={inspect.activePageId!}
                isDark={isDark}
              />
            }
          />
        ) : (
          <PaneHeader
            icon={<Laptop size={14} className="shrink-0 text-zinc-500" />}
            label="Waiting for page data…"
          />
        )}

        {/* Page comments (collapsible, only when there are unresolved comments) */}
        <PageCommentsSection
          annotations={annotations}
          activePageId={inspect.activePageId}
          isDark={isDark}
          divider={divider}
          muted={muted}
          collapsiblePanelClass={collapsiblePanelClass}
          fixProgress={fixProgress}
          originBindings={originBindings}
        />

        {/* Inspect tree (collapsible) */}
        <section ref={elementsSectionRef} className={`border-t ${divider}`}>
          <Collapsible.Root defaultOpen>
            <div className="flex items-center">
              <Collapsible.Trigger
                className={`group flex flex-1 items-center gap-1.5 px-2 py-2 text-[12px] font-medium`}
              >
                <ChevronDown size={12} className="hidden group-data-[panel-open]:block" />
                <ChevronRight size={12} className="block group-data-[panel-open]:hidden" />
                Inspect Tree
              </Collapsible.Trigger>
              <div className="group relative pr-3">
                <button
                  type="button"
                  className={`rounded p-1 opacity-30 hover:opacity-100 ${muted} hover:text-zinc-600 dark:hover:text-zinc-300`}
                  aria-label="Show inspect diagnostics"
                  title="Show inspect diagnostics"
                >
                  <InfoIcon className="size-3.5" />
                </button>
                <div
                  className={`pointer-events-none invisible absolute top-5 right-0 z-20 w-64 rounded border px-2 py-1.5 text-[10px] leading-4 opacity-0 shadow-sm transition-all group-hover:visible group-hover:opacity-100 group-focus-within:visible group-focus-within:opacity-100 ${
                    isDark
                      ? 'border-zinc-700 bg-zinc-900 text-zinc-300'
                      : 'border-zinc-300 bg-zinc-50 text-zinc-700'
                  }`}
                >
                  <div>Mode: {inspect.mode === 'page_locked' ? 'Page locked' : 'Global target'}</div>
                  {inspect.diagnostics ? (
                    <>
                      <div>Collector: {inspect.diagnostics.collector}</div>
                      <div>Nodes: {inspect.diagnostics.nodeCount}</div>
                      <div>React: {inspect.diagnostics.reactNodeCount}</div>
                      <div>DOM: {inspect.diagnostics.domFallbackNodeCount}</div>
                      <div>Source refs: {inspect.diagnostics.sourceLocationCount}</div>
                    </>
                  ) : (
                    <div>No diagnostics available.</div>
                  )}
                </div>
              </div>
            </div>
            <Collapsible.Panel className={collapsiblePanelClass}>
              <div
                className="thin-scrollbar pb-2"
                onPointerLeave={() => {
                  if (inspect.activePageId) {
                    rightDetailsPanelApi.setInspectHoverNode(
                      inspect.activePageId,
                      inspect.selectedNodeId,
                    )
                  }
                }}
              >
                {inspect.treeRootIds.length ? (
                  <InspectTree
                    treeRootIds={inspect.treeRootIds}
                    activePageId={inspect.activePageId!}
                    nodesById={inspect.nodesById}
                    unresolvedCountsByNodeId={unresolvedCountsByNodeId}
                    expanded={expanded}
                    setExpanded={setExpanded}
                    hoveredNodeId={inspect.hoveredNodeId}
                    selectedNodeId={inspect.selectedNodeId}
                    registerNodeElement={registerNodeElement}
                  />
                ) : (
                  <div className={`px-4 py-3 text-xs ${muted}`}>
                    No inspect hierarchy available yet.
                  </div>
                )}
              </div>
            </Collapsible.Panel>
          </Collapsible.Root>
        </section>

        {/* Element detail (collapsible) */}
        <section className={`border-t ${divider}`}>
          <Collapsible.Root defaultOpen>
            <Collapsible.Trigger
              className={`group flex w-full items-center gap-1.5 px-2 py-2 text-[12px] font-medium`}
            >
              <ChevronDown size={12} className="hidden group-data-[panel-open]:block" />
              <ChevronRight size={12} className="block group-data-[panel-open]:hidden" />
              Element Detail
            </Collapsible.Trigger>
            <Collapsible.Panel className={collapsiblePanelClass}>
              <div className="px-2 pb-3">
                <InspectDetailSection
                  activeDetail={activeDetail}
                  hoveredDetail={hoveredDetail}
                  isDark={isDark}
                  mutedClass={muted}
                  selectedDetail={selectedDetail}
                />
              </div>
            </Collapsible.Panel>
          </Collapsible.Root>
        </section>
      </div>

      {/* Comment composer (pinned at bottom) */}
      <section className={`shrink-0 border-t px-2 py-2 ${divider}`}>
        <ElementCommentComposer
          active={Boolean(activeDetail)}
          commentInputRef={commentInputRef}
          elementCommentText={elementCommentText}
          hasElementComment={hasElementComment}
          onChange={setElementCommentText}
          onSubmit={submitElementComment}
        />
      </section>
    </div>
  )
}

// --- Page Comments (unresolved comments anchored to the active page) ---

function unresolvedCommentsForPage(
  annotations: Annotation[],
  activePageId: string | null,
): Annotation[] {
  if (!activePageId) return []
  return annotations.filter((a) => {
    if (!isUnresolved(a.status)) return false
    if (a.anchor.type === 'canvas') return false
    if (a.anchor.type === 'region') {
      return a.metadata?.regionComponents?.some(
        (rc) => rc.pageId === activePageId,
      ) ?? false
    }
    return a.anchor.pageId === activePageId
  })
}

function PageCommentsSection({
  annotations,
  activePageId,
  isDark,
  divider,
  muted,
  collapsiblePanelClass,
  fixProgress,
  originBindings,
}: {
  annotations: Annotation[]
  activePageId: string | null
  isDark: boolean
  divider: string
  muted: string
  collapsiblePanelClass: string
  fixProgress: Record<string, FixProgressEntry>
  originBindings: OriginBindings
}) {
  const pageComments = unresolvedCommentsForPage(annotations, activePageId)
  if (!pageComments.length) return null
  // Fix is scoped to this page's comments — each is queued individually rather
  // than via the origin-wide trigger, so we don't touch the canvas's other pages.
  const hasBinding = groupAnnotationsByOrigin(pageComments).some(
    (g) => originBindings[g.origin],
  )
  const working = pageComments.some((a) => fixProgress[a.id]?.status === 'running')
  const fixPageComments = () => {
    if (!hasBinding || working) return
    for (const annotation of pageComments) {
      rightDetailsPanelApi.fixSingleAnnotation(annotation.id)
    }
  }
  const fixBtnClass = PRIMARY_BUTTON_BORDERED_CLASS
  return (
    <section className={`border-t ${divider}`}>
      <Collapsible.Root defaultOpen>
        <div className="flex items-center">
          <Collapsible.Trigger
            className={`group flex flex-1 items-center gap-1.5 px-2 py-2 text-[12px] font-medium`}
          >
            <ChevronDown size={12} className="hidden group-data-[panel-open]:block" />
            <ChevronRight size={12} className="block group-data-[panel-open]:hidden" />
            Comments
            <span className={`text-[10px] font-normal ${muted}`}>
              ({pageComments.length})
            </span>
          </Collapsible.Trigger>
          {hasBinding ? (
            <button
              type="button"
              onClick={fixPageComments}
              disabled={working}
              title="Fix this page's comments"
              className={`mr-2 inline-flex items-center gap-1.5 rounded-md border px-2 py-1 text-[11px] font-medium disabled:opacity-40 ${fixBtnClass}`}
            >
              {working ? (
                <Loader2 size={11} className="shrink-0 animate-spin" />
              ) : (
                <Play size={11} className="shrink-0" />
              )}
              <span>Fix {pageComments.length}</span>
            </button>
          ) : null}
        </div>
        <Collapsible.Panel className={collapsiblePanelClass}>
          <div className="space-y-2 px-2 pb-2">
            {pageComments.map((annotation) => (
              <CommentRow
                key={annotation.id}
                annotation={annotation}
                isDark={isDark}
                mutedClass={muted}
                rowHoverClass={isDark ? 'hover:bg-zinc-700/55' : 'hover:bg-zinc-50'}
                focusRowClass=""
                registerAnnotationElement={() => {}}
                progress={fixProgress[annotation.id]}
              />
            ))}
          </div>
        </Collapsible.Panel>
      </Collapsible.Root>
    </section>
  )
}

// --- Page Header Actions (inline with PaneHeader) ---

function PageHeaderActions({
  pageId,
  isDark,
}: {
  // `pageId` comes from inspect.activePageId (always present).
  pageId: string
  isDark: boolean
}) {
  const btnClass = `rounded p-1 ${
    isDark ? 'text-zinc-400 hover:bg-zinc-700 hover:text-zinc-200' : 'text-zinc-500 hover:bg-zinc-200 hover:text-zinc-700'
  }`
  const deleteBtnClass = `rounded p-1 ${
    isDark ? 'text-zinc-400 hover:bg-red-500/12 hover:text-red-400' : 'text-zinc-500 hover:bg-red-50 hover:text-red-600'
  }`

  return (
    <div className="flex items-center gap-0.5">
      <button type="button" className={btnClass} aria-label="Open DevTools" title="Open DevTools" onClick={() => rightDetailsPanelApi.openBrowserDevTools()}>
        <Wrench size={13} />
      </button>
      <button type="button" className={deleteBtnClass} aria-label="Delete Page" title="Delete Page" onClick={() => rightDetailsPanelApi.deletePage(pageId)}>
        <Trash2 size={13} />
      </button>
    </div>
  )
}

function PageFavicon({
  faviconUrl,
  label,
  width,
}: {
  faviconUrl?: string | null
  label?: string
  width?: number
}) {
  const [imageFailed, setImageFailed] = useState(false)

  useEffect(() => {
    setImageFailed(false)
  }, [faviconUrl])

  if (faviconUrl && !imageFailed) {
    return (
      <img
        alt=""
        aria-hidden="true"
        src={faviconUrl}
        className="h-[14px] w-[14px] shrink-0 rounded-[3px]"
        onError={() => setImageFailed(true)}
      />
    )
  }

  const Icon = viewportIcon(label, width)
  return <Icon size={14} className="shrink-0 text-zinc-500" />
}

function viewportIcon(label?: string, width?: number) {
  if (label?.startsWith('iPhone')) return Smartphone
  if (label?.startsWith('iPad')) return Tablet
  if (typeof width !== 'number') return Laptop
  if (width < 600) return Smartphone
  if (width < 1100) return Tablet
  return Laptop
}
