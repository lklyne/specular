// ADR 0008 — page selection popup. URL/nav redundancy with PageChrome is
// accepted per §6.

import { useEffect, useRef, useState } from 'react'
import {
  AlignHorizontalDistributeCenter,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  Eye,
  EyeClosed,
  Maximize2,
  RotateCw,
  X,
} from 'lucide-react'
import { resolveAddressInput } from '../../shared/url'
import { VIEWPORT_PRESETS } from '../../shared/constants'
import type { CanvasScenePageEntity, LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { PagePresetDropdown } from '../shared/PagePresetDropdown'
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
    | 'focusSelection'
    | 'restoreFocusCamera'
    | 'setFocusPresentationMode'
    | 'setFocusAnnotationsVisible'
    | 'distributeSelection'
  >
  isDark: boolean
  layout: LayoutUpdateData
  selectedPages: CanvasScenePageEntity[]
  interactionIdle: boolean
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
  const open = usePopupDelayedKey(
    popupKey,
    focusedPageEntity != null || (interactionIdle && count > 0),
  )

  // Hold optimistic URL until the navigate IPC → broadcast round-trip catches up.
  const [draftUrl, setDraftUrl] = useState<string | null>(null)
  const [presetDropdownOpen, setPresetDropdownOpen] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)
  const suppressPresetOpenRef = useRef(false)

  const currentUrl = single?.url
  useEffect(() => {
    if (draftUrl !== null && currentUrl === draftUrl) setDraftUrl(null)
  }, [currentUrl, draftUrl])

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
  const value = draftUrl ?? (single && single.url !== 'about:blank' ? single.url : '')

  const commitUrl = () => {
    if (!single || draftUrl === null) return
    const trimmed = draftUrl.trim()
    if (!trimmed || trimmed === single.url) {
      setDraftUrl(null)
      return
    }
    const normalized = resolveAddressInput(trimmed)
    setDraftUrl(normalized)
    api.navigatePage(single.id, normalized)
  }

  const entityIds = single ? [single.id] : selectedPages.map((p) => p.id)
  const noun = isSingle ? 'page' : `${count} pages`

  const useFlushFocusMenu = pinPopupToViewportTop
  const popupLayoutDependency = pinPopupToViewportTop ? 'viewport-top' : 'above'

  const presetLabel = focusPresentation
    ? focusPresentation.authoredLabel
    : single
    ? (() => {
        const preset = VIEWPORT_PRESETS[single.presetIndex]
        const isCustom = !preset || single.width !== preset.width || single.height !== preset.height
        return isCustom ? 'Custom' : preset.label
      })()
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
        {isSingle && single ? (
          <>
            <CanvasItemPopup.Section>
              <CanvasItemPopup.IconButton
                isDark={isDark}
                title="Back"
                ariaLabel="Go back"
                onClick={() => api.goBackPage(single.id)}
              >
                <ChevronLeft
                  size={14}
                  style={!single.canGoBack ? { opacity: 0.3 } : undefined}
                />
              </CanvasItemPopup.IconButton>
              <CanvasItemPopup.IconButton
                isDark={isDark}
                title="Forward"
                ariaLabel="Go forward"
                onClick={() => api.goForwardPage(single.id)}
              >
                <ChevronRight
                  size={14}
                  style={!single.canGoForward ? { opacity: 0.3 } : undefined}
                />
              </CanvasItemPopup.IconButton>
              <CanvasItemPopup.IconButton
                isDark={isDark}
                title={single.isLoading ? 'Loading…' : 'Reload'}
                ariaLabel="Reload page"
                onClick={() => api.reloadPage(single.id)}
              >
                <RotateCw size={12} className={single.isLoading ? 'animate-spin' : ''} />
              </CanvasItemPopup.IconButton>
            </CanvasItemPopup.Section>
            <CanvasItemPopup.Divider isDark={isDark} />
            <CanvasItemPopup.Section grow>
              <input
                ref={inputRef}
                type="text"
                value={value}
                placeholder="Type a URL"
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
            <CanvasItemPopup.Divider isDark={isDark} />
            <CanvasItemPopup.Section>
              {focusPresentation ? (
                <div className={tabGroupClass} role="group" aria-label="Focused viewport mode">
                  <PagePresetDropdown
                    isDark={isDark}
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
          {count >= 3 ? (
            <CanvasItemPopup.IconButton
              isDark={isDark}
              title="Distribute spacing"
              ariaLabel="Distribute spacing"
              onClick={() => api.distributeSelection()}
            >
              <AlignHorizontalDistributeCenter size={14} />
            </CanvasItemPopup.IconButton>
          ) : null}
        </CanvasItemPopup.Section>
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.Root>
  )
}
