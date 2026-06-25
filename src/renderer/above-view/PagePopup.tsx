// ADR 0008 — page selection popup. URL/nav redundancy with PageChrome is
// accepted per §6.

import { useEffect, useRef, useState } from 'react'
import { ChevronDown, ChevronLeft, ChevronRight, RotateCw } from 'lucide-react'
import { normalizeUserUrl } from '../../shared/url'
import { VIEWPORT_PRESETS } from '../../shared/constants'
import type {
  CanvasBgElectronAPI,
  CanvasScenePageEntity,
  LayoutUpdateData,
} from '../../shared/types'
import { PagePresetDropdown } from '../shared/PagePresetDropdown'
import { CanvasItemPopup } from './CanvasItemPopup'
import { DeviceViewportPopupControls } from './DeviceViewportPopupControls'
import { POPUP_OFFSET_Y, usePopupDelayedKey } from './usePopupDelayedKey'

const URL_INPUT_MIN_WIDTH = 280

export function PagePopup({
  api,
  isDark,
  layout,
  selectedPages,
  interactionIdle,
}: {
  api: Pick<
    CanvasBgElectronAPI,
    | 'duplicatePage'
    | 'deletePage'
    | 'navigatePage'
    | 'goBackPage'
    | 'goForwardPage'
    | 'reloadPage'
    | 'setDeviceOrientation'
    | 'toggleDeviceShell'
    | 'setPagePreset'
    | 'setPageCustom'
    | 'focusSelection'
    | 'distributeSelection'
  >
  isDark: boolean
  layout: LayoutUpdateData
  selectedPages: CanvasScenePageEntity[]
  interactionIdle: boolean
}) {
  const count = selectedPages.length
  const ids = selectedPages.map((p) => p.id).join('|')
  const open = usePopupDelayedKey(ids, interactionIdle && count > 0)

  // Hold optimistic URL until the navigate IPC → broadcast round-trip catches up.
  const [draftUrl, setDraftUrl] = useState<string | null>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  const single = count === 1 ? selectedPages[0] : null
  const currentUrl = single?.url
  useEffect(() => {
    if (draftUrl !== null && currentUrl === draftUrl) setDraftUrl(null)
  }, [currentUrl, draftUrl])

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
    const normalized = normalizeUserUrl(trimmed)
    setDraftUrl(normalized)
    api.navigatePage(single.id, normalized)
  }

  const entityIds = selectedPages.map((p) => p.id)
  const noun = isSingle ? 'page' : `${count} pages`

  const presetLabel = single
    ? (() => {
        const preset = VIEWPORT_PRESETS[single.presetIndex]
        const isCustom = !preset || single.width !== preset.width || single.height !== preset.height
        return isCustom ? 'Custom' : preset.label
      })()
    : null

  const sizeTriggerClass = isDark
    ? 'flex h-6 items-center gap-1 rounded-[6px] border-0 px-2 text-xs text-zinc-300 transition-colors hover:bg-[rgba(253,248,245,0.1)] hover:text-zinc-100'
    : 'flex h-6 items-center gap-1 rounded-[6px] border-0 px-2 text-xs text-zinc-600 transition-colors hover:bg-[#fdf8f5] hover:text-zinc-900'

  return (
    <CanvasItemPopup.Root
      entityIds={entityIds}
      layout={layout}
      open={open}
      placement="above"
      align={isSingle ? 'stretch' : 'center'}
      offset={POPUP_OFFSET_Y}
    >
      <CanvasItemPopup.Frame isDark={isDark}>
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
            </CanvasItemPopup.Section>
            <CanvasItemPopup.Divider isDark={isDark} />
            <DeviceViewportPopupControls
              isDark={isDark}
              showDeviceFrame={single.showDeviceFrame ?? false}
              orientation={single.deviceOrientation ?? 'portrait'}
              noun="page"
              onToggleDeviceFrame={() => api.toggleDeviceShell(single.id)}
              onSetOrientation={(orientation) =>
                api.setDeviceOrientation(single.id, orientation)
              }
            />
          </>
        ) : null}
        <CanvasItemPopup.EntityActions
          isDark={isDark}
          noun={noun}
          count={count}
          onDuplicate={() => {
            for (const p of selectedPages) api.duplicatePage(p.id)
          }}
          onDelete={() => {
            for (const p of selectedPages) api.deletePage(p.id)
          }}
          api={api}
        />
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.Root>
  )
}
