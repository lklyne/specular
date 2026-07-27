// ADR 0008 — page selection popup. URL/nav redundancy with PageChrome is
// accepted per §6.

import { useEffect, useRef, useState } from 'react'
import {
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  Eye,
  EyeClosed,
  Link2,
  Maximize2,
  MessageSquarePlus,
  RotateCw,
  Smartphone,
  X,
} from 'lucide-react'
import { resolveAddressInput } from '../../shared/url'
import { VIEWPORT_PRESETS } from '../../shared/constants'
import type { CanvasScenePageEntity, LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { PagePresetDropdown } from '../shared/PagePresetDropdown'
import { THEME_MODE_ICON, THEME_MODE_LABEL, nextThemeMode } from '../shared/themeModeCycle'
import { selectionAnnotationBounds } from './annotationMath'
import type { AnnotateHandler } from './annotationMath'
import { CanvasItemPopup } from './CanvasItemPopup'
import { DeviceViewportPopupControls } from './DeviceViewportPopupControls'
import { POPUP_OFFSET_Y, usePopupDelayedKey } from './usePopupDelayedKey'

const URL_INPUT_MIN_WIDTH = 280

function popupTabButtonClass(isDark: boolean, active: boolean, widthClass = 'w-6'): string {
  const base =
    `flex h-6 ${widthClass} items-center justify-center gap-1 rounded-[6px] border-0 text-xs transition-colors`
  if (active) {
    return isDark
      ? `${base} bg-[rgba(253,248,245,0.1)] text-zinc-100`
      : `${base} bg-[var(--surface-popup)] text-zinc-900`
  }
  return isDark
    ? `${base} text-zinc-300 hover:bg-[rgba(253,248,245,0.1)] hover:text-zinc-100`
    : `${base} text-zinc-600 hover:bg-[var(--color-stone-100)] hover:text-zinc-900`
}

export function PagePopup({
  api,
  isDark,
  layout,
  selectedPages,
  interactionIdle,
  onAnnotate,
}: {
  api: Pick<
    CanvasBgElectronAPI,
    | 'navigatePage'
    | 'goBackPage'
    | 'goForwardPage'
    | 'reloadPage'
    | 'setDeviceOrientation'
    | 'toggleDeviceShell'
    | 'setPagePreset'
    | 'setPageCustom'
    | 'setPageColorScheme'
    | 'focusSelection'
    | 'restoreFocusCamera'
    | 'setFocusPresentationMode'
    | 'setFocusAnnotationsVisible'
    | 'arrangeSelection'
    | 'toggleSyncSelection'
    | 'unsyncPage'
  >
  isDark: boolean
  layout: LayoutUpdateData
  selectedPages: CanvasScenePageEntity[]
  interactionIdle: boolean
  onAnnotate: AnnotateHandler
}) {
  // During a focus session the bar belongs to the focused page regardless of
  // selection — draw/placement tools clear or reassign selection mid-session,
  // but the focus bar must stay pinned. Outside focus it's a normal
  // single-page selection popup.
  const focusedPageEntity = layout.focusPresentation
    ? (layout.entities.find(
        (entity): entity is CanvasScenePageEntity =>
          entity.kind === 'page' && entity.id === layout.focusPresentation!.pageId,
      ) ?? null)
    : null

  const count = focusedPageEntity ? 1 : selectedPages.length
  const single = focusedPageEntity ?? (count === 1 ? selectedPages[0] : null)
  const popupKey = single ? single.id : selectedPages.map((p) => p.id).join('|')
  // The show-delay is for transient selections (rubber-band, rapid clicks). The
  // focus bar is deliberate chrome, so skip it while focused — otherwise
  // switching focused pages re-arms the delay on the new page id and the bar
  // blinks out for POPUP_SHOW_DELAY_MS mid-transition.
  const delayedOpen = usePopupDelayedKey(popupKey, interactionIdle && count > 0)
  const open = focusedPageEntity != null || delayedOpen

  // Hold optimistic URL until the navigate IPC → broadcast round-trip catches up.
  const [draftUrl, setDraftUrl] = useState<string | null>(null)
  const [presetDropdownOpen, setPresetDropdownOpen] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)
  const suppressPresetOpenRef = useRef(false)

  // Nav + URL bar act on the whole page selection. Linked peers of any selected
  // page are handled downstream by navigatePage() in the main process.
  const navPages = single ? [single] : selectedPages
  const entityIds = navPages.map((p) => p.id)
  const canGoBack = navPages.some((p) => p.canGoBack)
  const canGoForward = navPages.some((p) => p.canGoForward)
  const anyLoading = navPages.some((p) => p.isLoading)
  const allFramed = navPages.every((p) => p.showDeviceFrame ?? false)
  // Sync is a multi-page relationship; only offered for 2+ selected pages.
  const canSync = !single && selectedPages.length >= 2
  // "Unsync" only when the whole selection is one shared set — mirrors main's
  // clear-vs-merge decision. A selection straddling two sets reads as "Sync"
  // and merges, rather than falsely offering to unsync.
  const firstSyncId = selectedPages[0]?.syncId ?? null
  const allShareOneSet =
    canSync && firstSyncId != null && selectedPages.every((p) => p.syncId === firstSyncId)
  // One shared URL across the selection, else blank so the placeholder shows.
  const distinctUrls = new Set(
    navPages.map((p) => p.url).filter((url) => url && url !== 'about:blank'),
  )
  const sharedUrl = distinctUrls.size === 1 ? [...distinctUrls][0]! : ''

  useEffect(() => {
    if (draftUrl !== null && sharedUrl === draftUrl) setDraftUrl(null)
  }, [sharedUrl, draftUrl])

  const focusPresentation =
    single && layout.focusPresentation?.pageId === single.id
      ? layout.focusPresentation
      : null
  const focusMode = focusPresentation?.mode ?? null
  useEffect(() => {
    if (focusMode !== 'device') {
      suppressPresetOpenRef.current = false
      setPresetDropdownOpen(false)
    }
  }, [focusMode])
  // Pin to the viewport top only while focused; on leave, flip back to the
  // page-anchored placement immediately so the FLIP tween rides the camera
  // restore as one motion (durations match in restoreFocusCamera).
  const pinPopupToViewportTop = focusPresentation !== null

  if (count === 0) return null
  const isSingle = count === 1
  const value = draftUrl ?? sharedUrl
  const urlPlaceholder = isSingle ? 'Type a URL' : `${count} pages selected`

  const commitUrl = () => {
    if (draftUrl === null) return
    const trimmed = draftUrl.trim()
    if (!trimmed || trimmed === sharedUrl) {
      setDraftUrl(null)
      return
    }
    const normalized = resolveAddressInput(trimmed)
    setDraftUrl(normalized)
    for (const id of entityIds) api.navigatePage(id, normalized)
  }

  const goBack = () => entityIds.forEach((id) => api.goBackPage(id))
  const goForward = () => entityIds.forEach((id) => api.goForwardPage(id))
  const reload = () => entityIds.forEach((id) => api.reloadPage(id))
  // Drive every page to one target state (only the ones off the target flip),
  // so a mixed selection resolves to all-on rather than inverting each page.
  const toggleFrameAll = () => {
    const target = !allFramed
    for (const p of navPages) {
      if ((p.showDeviceFrame ?? false) !== target) api.toggleDeviceShell(p.id)
    }
  }

  const useFlushFocusMenu = pinPopupToViewportTop
  const popupLayoutDependency = pinPopupToViewportTop ? 'viewport-top' : 'above'

  const activeIsCustom = single
    ? (() => {
        const preset = VIEWPORT_PRESETS[single.presetIndex]
        return !preset || single.width !== preset.width || single.height !== preset.height
      })()
    : false
  const activePresetIndex = single && !activeIsCustom ? single.presetIndex : null

  // Batch preset: highlight the shared preset when every selected page matches
  // it, else null → "Multiple". Custom pages count as their own key so a mixed
  // set never falsely reads as one preset.
  const multiPresetIndex = (() => {
    if (single) return null
    const keys = new Set(
      selectedPages.map((p) => {
        const preset = VIEWPORT_PRESETS[p.presetIndex]
        return !preset || p.width !== preset.width || p.height !== preset.height
          ? 'custom'
          : p.presetIndex
      }),
    )
    const only = keys.size === 1 ? [...keys][0] : null
    return typeof only === 'number' ? only : null
  })()
  const multiPresetLabel =
    multiPresetIndex != null ? VIEWPORT_PRESETS[multiPresetIndex].label : 'Multiple'

  const presetLabel = focusPresentation
    ? focusPresentation.authoredLabel
    : single
    ? activeIsCustom
      ? 'Custom'
      : VIEWPORT_PRESETS[single.presetIndex].label
    : null

  const sizeTriggerClass = isDark
    ? 'flex h-6 items-center gap-1 rounded-[6px] border-0 px-2 text-xs text-zinc-300 transition-colors hover:bg-[rgba(253,248,245,0.1)] hover:text-zinc-100'
    : 'flex h-6 items-center gap-1 rounded-[6px] border-0 px-2 text-xs text-zinc-600 transition-colors hover:bg-[var(--color-stone-100)] hover:text-zinc-900'
  const focusSizeTriggerClass = popupTabButtonClass(isDark, focusMode === 'device', 'px-2')
  const tabGroupClass = isDark
    ? 'flex h-7 items-center gap-0.5 rounded-[7px] bg-black/15 p-0.5'
    : 'flex h-7 items-center gap-0.5 rounded-[7px] bg-zinc-900/10 p-0.5'

  return (
    <CanvasItemPopup.Root
      entityIds={entityIds}
      layout={layout}
      open={open}
      placement={popupLayoutDependency}
      align={isSingle ? 'stretch' : 'center'}
      offset={POPUP_OFFSET_Y}
    >
      <CanvasItemPopup.Frame
        isDark={isDark}
        flush={useFlushFocusMenu}
        fullWidth={pinPopupToViewportTop}
      >
        <CanvasItemPopup.Section>
          <CanvasItemPopup.IconButton
            isDark={isDark}
            title="Back"
            ariaLabel="Go back"
            onClick={goBack}
          >
            <ChevronLeft size={14} style={!canGoBack ? { opacity: 0.3 } : undefined} />
          </CanvasItemPopup.IconButton>
          <CanvasItemPopup.IconButton
            isDark={isDark}
            title="Forward"
            ariaLabel="Go forward"
            onClick={goForward}
          >
            <ChevronRight size={14} style={!canGoForward ? { opacity: 0.3 } : undefined} />
          </CanvasItemPopup.IconButton>
          <CanvasItemPopup.IconButton
            isDark={isDark}
            title={anyLoading ? 'Loading…' : 'Reload'}
            ariaLabel="Reload page"
            onClick={reload}
          >
            <RotateCw size={12} className={anyLoading ? 'animate-spin' : ''} />
          </CanvasItemPopup.IconButton>
        </CanvasItemPopup.Section>
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.Section grow>
          <input
            ref={inputRef}
            type="text"
            value={value}
            placeholder={urlPlaceholder}
            onChange={(e) => setDraftUrl(e.target.value)}
            onBlur={commitUrl}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault()
                commitUrl()
                inputRef.current?.blur()
              } else if (e.key === 'Escape') {
                e.preventDefault()
                setDraftUrl(null)
                inputRef.current?.blur()
              }
            }}
            onPointerDown={(e) => e.stopPropagation()}
            className={`min-w-0 flex-1 rounded-[6px] border px-2 py-1 text-xs outline-none focus:ring-1 ${
              isDark
                ? 'border-zinc-700 bg-zinc-950 text-zinc-100 placeholder:text-zinc-500 focus:ring-blue-500/40'
                : 'border-zinc-300 bg-white text-zinc-900 placeholder:text-zinc-400 focus:ring-blue-500/40'
            }`}
            style={{ minWidth: URL_INPUT_MIN_WIDTH }}
          />
        </CanvasItemPopup.Section>
        {isSingle && single ? (
          <>
            <CanvasItemPopup.Divider isDark={isDark} />
            <CanvasItemPopup.Section>
              {focusPresentation ? (
                <div className={tabGroupClass} role="group" aria-label="Focused viewport mode">
                  <PagePresetDropdown
                    isDark={isDark}
                    activePreset={activePresetIndex}
                    customActive={activeIsCustom}
                    onOpenChange={(nextOpen) => {
                      if (
                        nextOpen &&
                        (focusPresentation.mode !== 'device' || suppressPresetOpenRef.current)
                      ) {
                        suppressPresetOpenRef.current = false
                        api.setFocusPresentationMode('device')
                        setPresetDropdownOpen(false)
                        return
                      }
                      setPresetDropdownOpen(nextOpen)
                    }}
                    onSelectPreset={(index) => api.setPagePreset(single.id, index)}
                    onSelectCustom={() => api.setPageCustom(single.id)}
                    open={presetDropdownOpen}
                    trigger={
                      <button
                        type="button"
                        className={focusSizeTriggerClass}
                        title="Device size"
                        aria-label="Use the saved device size while focused"
                        aria-pressed={focusPresentation.mode === 'device'}
                        onPointerDownCapture={(event) => {
                          if (focusPresentation.mode === 'device') return
                          event.preventDefault()
                          event.stopPropagation()
                          suppressPresetOpenRef.current = true
                          setPresetDropdownOpen(false)
                          api.setFocusPresentationMode('device')
                        }}
                        onClickCapture={(event) => {
                          if (suppressPresetOpenRef.current) {
                            event.preventDefault()
                            event.stopPropagation()
                            suppressPresetOpenRef.current = false
                            return
                          }
                          if (focusPresentation.mode === 'device') return
                          event.preventDefault()
                          event.stopPropagation()
                          if (event.detail === 0) {
                            suppressPresetOpenRef.current = true
                            setPresetDropdownOpen(false)
                            api.setFocusPresentationMode('device')
                          }
                        }}
                      >
                        <span className="max-w-28 truncate">{presetLabel}</span>
                        <ChevronDown size={10} className="shrink-0 opacity-50" />
                      </button>
                    }
                  />
                  <button
                    type="button"
                    className={popupTabButtonClass(isDark, focusPresentation.mode === 'fit', 'px-2')}
                    title="Fit to canvas"
                    aria-label="Fit the page to the canvas"
                    aria-pressed={focusPresentation.mode === 'fit'}
                    onClick={() => api.setFocusPresentationMode('fit')}
                  >
                    Fit
                  </button>
                  <button
                    type="button"
                    className={popupTabButtonClass(isDark, focusPresentation.mode === 'fill', 'px-2')}
                    title="Fill the window"
                    aria-label="Fill the window like a browser"
                    aria-pressed={focusPresentation.mode === 'fill'}
                    onClick={() => api.setFocusPresentationMode('fill')}
                  >
                    Fill
                  </button>
                </div>
              ) : (
                <PagePresetDropdown
                  isDark={isDark}
                  activePreset={activePresetIndex}
                  customActive={activeIsCustom}
                  onSelectPreset={(index) => api.setPagePreset(single.id, index)}
                  onSelectCustom={() => api.setPageCustom(single.id)}
                  trigger={
                    <button type="button" className={sizeTriggerClass} title="Page size">
                      <span className="truncate">{presetLabel}</span>
                      <ChevronDown size={10} className="shrink-0 opacity-50" />
                    </button>
                  }
                />
              )}
            </CanvasItemPopup.Section>
            <CanvasItemPopup.Divider isDark={isDark} />
            <DeviceViewportPopupControls
              isDark={isDark}
              showDeviceFrame={single.showDeviceFrame ?? false}
              orientation={single.deviceOrientation ?? 'portrait'}
              noun="page"
              disabled={focusPresentation?.mode === 'fill'}
              onToggleDeviceFrame={() => api.toggleDeviceShell(single.id)}
              onSetOrientation={(orientation) =>
                api.setDeviceOrientation(single.id, orientation)
              }
            />
            <CanvasItemPopup.Divider isDark={isDark} />
            <CanvasItemPopup.Section>
              {(() => {
                const mode = single.colorScheme ?? 'system'
                const Icon = THEME_MODE_ICON[mode]
                return (
                  <CanvasItemPopup.IconButton
                    isDark={isDark}
                    title={`${THEME_MODE_LABEL[mode]} color scheme`}
                    ariaLabel={`Color scheme: ${THEME_MODE_LABEL[mode]}. Click to change.`}
                    onClick={() => {
                      const next = nextThemeMode(mode)
                      api.setPageColorScheme(single.id, next === 'system' ? null : next)
                    }}
                  >
                    <Icon size={14} isDark={isDark} />
                  </CanvasItemPopup.IconButton>
                )
              })()}
            </CanvasItemPopup.Section>
          </>
        ) : null}
        {!isSingle ? (
          <>
            <CanvasItemPopup.Divider isDark={isDark} />
            <CanvasItemPopup.Section>
              <PagePresetDropdown
                isDark={isDark}
                activePreset={multiPresetIndex}
                customActive={false}
                hideCustom
                onSelectPreset={(index) =>
                  selectedPages.forEach((p) => api.setPagePreset(p.id, index))
                }
                onSelectCustom={() => {}}
                trigger={
                  <button type="button" className={sizeTriggerClass} title="Page size">
                    <span className="truncate">{multiPresetLabel}</span>
                    <ChevronDown size={10} className="shrink-0 opacity-50" />
                  </button>
                }
              />
            </CanvasItemPopup.Section>
            <CanvasItemPopup.Divider isDark={isDark} />
            <CanvasItemPopup.Section>
              <CanvasItemPopup.IconButton
                isDark={isDark}
                active={allFramed}
                title="Device frame"
                ariaLabel="Toggle device frame for selected pages"
                onClick={toggleFrameAll}
              >
                <Smartphone size={14} />
              </CanvasItemPopup.IconButton>
            </CanvasItemPopup.Section>
          </>
        ) : null}
        <CanvasItemPopup.Section>
          {focusPresentation ? (
            <CanvasItemPopup.IconButton
              isDark={isDark}
              title={focusPresentation.annotationsVisible ? 'Hide other items' : 'Show other items'}
              ariaLabel={focusPresentation.annotationsVisible ? 'Hide other items' : 'Show other items'}
              onClick={() =>
                api.setFocusAnnotationsVisible(!focusPresentation.annotationsVisible)
              }
            >
              {focusPresentation.annotationsVisible ? (
                <Eye size={14} />
              ) : (
                <EyeClosed size={14} />
              )}
            </CanvasItemPopup.IconButton>
          ) : null}
          <CanvasItemPopup.ArrangeButtons
            isDark={isDark}
            count={count}
            arrange={api.arrangeSelection}
          />
          {canSync ? (
            <CanvasItemPopup.IconButton
              isDark={isDark}
              active={allShareOneSet}
              title={allShareOneSet ? 'Unsync navigation' : 'Sync navigation'}
              ariaLabel={allShareOneSet ? 'Unsync navigation' : 'Sync navigation'}
              onClick={() => api.toggleSyncSelection()}
            >
              <Link2 size={14} />
            </CanvasItemPopup.IconButton>
          ) : null}
          {!isSingle
            ? (() => {
                const annotateRect = selectionAnnotationBounds(layout.entities, entityIds)
                if (!annotateRect) return null
                return (
                  <CanvasItemPopup.IconButton
                    isDark={isDark}
                    title={`Annotate ${count} pages`}
                    ariaLabel={`Annotate ${count} pages`}
                    onClick={() => onAnnotate(entityIds, annotateRect)}
                  >
                    <MessageSquarePlus size={14} />
                  </CanvasItemPopup.IconButton>
                )
              })()
            : null}
          {single && single.synced ? (
            <CanvasItemPopup.IconButton
              isDark={isDark}
              active
              title="Unsync navigation"
              ariaLabel="Unsync this page from its sync set"
              onClick={() => api.unsyncPage(single.id)}
            >
              <Link2 size={14} />
            </CanvasItemPopup.IconButton>
          ) : null}
          {isSingle ? (
            <CanvasItemPopup.IconButton
              isDark={isDark}
              title={focusPresentation ? 'Exit focus' : 'Focus page'}
              ariaLabel={focusPresentation ? 'Exit focus' : 'Focus page'}
              onClick={() => {
                if (focusPresentation) api.restoreFocusCamera()
                else api.focusSelection()
              }}
            >
              {focusPresentation ? <X size={14} /> : <Maximize2 size={14} />}
            </CanvasItemPopup.IconButton>
          ) : null}
        </CanvasItemPopup.Section>
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.Root>
  )
}
