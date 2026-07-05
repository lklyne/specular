import { ipcChannels } from '../shared/ipc-contract'
import { ipcRenderer } from 'electron'
import {
  createScrollSyncData,
  resolveScrollTop,
} from '../shared/scroll-sync'
import type { ScrollSyncData } from '../shared/types'

const LINKED_SCROLL_SUPPRESSION_MS = 300
const FOLLOWER_SETTLE_THRESHOLD_PX = 1
const FOLLOWER_LERP = 0.22
const FOLLOWER_MAX_STEP_PX = 96

let scrollPageRequested = false
let suppressScrollBroadcastUntil = 0
let lastBroadcastScrollData: ScrollSyncData | null = null
let followerTargetScrollTop: number | null = null
let followerTargetScrollLeft: number | null = null
let followerAnimationFrame = 0

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value))
}

function maxScrollLeft(): number {
  return Math.max(
    0,
    (document.documentElement?.scrollWidth ?? 0) - window.innerWidth,
    (document.body?.scrollWidth ?? 0) - window.innerWidth,
  )
}

export function seedScrollSyncBaseline(): void {
  lastBroadcastScrollData = createScrollSyncData()
}

export function startFollowerAnimation(): void {
  if (followerAnimationFrame !== 0) return

  const tick = () => {
    followerAnimationFrame = 0

    if (followerTargetScrollTop === null || followerTargetScrollLeft === null) {
      return
    }

    const deltaTop = followerTargetScrollTop - window.scrollY
    const deltaLeft = followerTargetScrollLeft - window.scrollX

    if (
      Math.abs(deltaTop) <= FOLLOWER_SETTLE_THRESHOLD_PX &&
      Math.abs(deltaLeft) <= FOLLOWER_SETTLE_THRESHOLD_PX
    ) {
      window.scrollTo({
        top: followerTargetScrollTop,
        left: followerTargetScrollLeft,
        behavior: 'auto',
      })
      followerTargetScrollTop = null
      followerTargetScrollLeft = null
      return
    }

    const nextTop =
      window.scrollY +
      clamp(deltaTop * FOLLOWER_LERP, -FOLLOWER_MAX_STEP_PX, FOLLOWER_MAX_STEP_PX)
    const nextLeft =
      window.scrollX +
      clamp(deltaLeft * FOLLOWER_LERP, -FOLLOWER_MAX_STEP_PX, FOLLOWER_MAX_STEP_PX)

    suppressScrollBroadcastUntil = Date.now() + LINKED_SCROLL_SUPPRESSION_MS
    window.scrollTo({
      top: nextTop,
      left: nextLeft,
      behavior: 'auto',
    })

    followerAnimationFrame = window.requestAnimationFrame(tick)
  }

  followerAnimationFrame = window.requestAnimationFrame(tick)
}

export function stopFollowerAnimation(): void {
  if (followerAnimationFrame !== 0) {
    window.cancelAnimationFrame(followerAnimationFrame)
    followerAnimationFrame = 0
  }
  followerTargetScrollTop = null
  followerTargetScrollLeft = null
}

export function clearScrollSuppression(): void {
  suppressScrollBroadcastUntil = 0
}

function hasMeaningfulScrollDelta(
  previous: ScrollSyncData | null,
  next: ScrollSyncData,
): boolean {
  if (!previous) return true
  if (previous.anchorSelector !== next.anchorSelector) return true
  if (Math.abs((previous.anchorProgress ?? 0) - (next.anchorProgress ?? 0)) > 0.03) {
    return true
  }
  if (Math.abs(previous.xProgress - next.xProgress) > 0.01) return true
  if (Math.abs(previous.yProgress - next.yProgress) > 0.01) return true
  return false
}

export function queueScrollSyncBroadcast(interactive: boolean): void {
  if (!interactive || Date.now() < suppressScrollBroadcastUntil) return
  if (scrollPageRequested) return
  scrollPageRequested = true

  window.requestAnimationFrame(() => {
    scrollPageRequested = false
    if (!interactive || Date.now() < suppressScrollBroadcastUntil) return
    const nextData = createScrollSyncData()
    if (!hasMeaningfulScrollDelta(lastBroadcastScrollData, nextData)) return
    lastBroadcastScrollData = nextData
    ipcRenderer.send(ipcChannels.pageScrollChanged, nextData)
  })
}

export function applyIncomingLinkedScroll(data: ScrollSyncData): void {
  suppressScrollBroadcastUntil = Date.now() + LINKED_SCROLL_SUPPRESSION_MS
  lastBroadcastScrollData = data
  followerTargetScrollTop = resolveScrollTop(data)
  followerTargetScrollLeft = maxScrollLeft() * Math.max(0, Math.min(1, data.xProgress))
  startFollowerAnimation()
}
