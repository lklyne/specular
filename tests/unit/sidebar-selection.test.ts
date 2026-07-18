import { describe, expect, it } from 'vitest'
import { sidebarSelectionIntent } from '../../src/renderer/left-sidebar/sidebar-selection'

const visibleIds = ['note-a', 'note-b', 'note-c', 'page-a']

describe('sidebarSelectionIntent', () => {
  it('replaces the selection on a plain click', () => {
    expect(
      sidebarSelectionIntent({
        clickedId: 'note-b',
        orderedVisibleIds: visibleIds,
        lastClickedId: 'note-a',
        shiftKey: false,
        toggleKey: false,
      }),
    ).toEqual({ ids: ['note-b'], mode: 'replace', nextAnchorId: 'note-b' })
  })

  it('toggles one item for a command or control click', () => {
    expect(
      sidebarSelectionIntent({
        clickedId: 'note-b',
        orderedVisibleIds: visibleIds,
        lastClickedId: 'note-a',
        shiftKey: false,
        toggleKey: true,
      }),
    ).toEqual({ ids: ['note-b'], mode: 'toggle', nextAnchorId: 'note-b' })
  })

  it('adds the visible contiguous range from the last click in either direction', () => {
    expect(
      sidebarSelectionIntent({
        clickedId: 'page-a',
        orderedVisibleIds: visibleIds,
        lastClickedId: 'note-b',
        shiftKey: true,
        toggleKey: false,
      }),
    ).toEqual({
      ids: ['note-b', 'note-c', 'page-a'],
      mode: 'add',
      nextAnchorId: 'page-a',
    })

    expect(
      sidebarSelectionIntent({
        clickedId: 'note-a',
        orderedVisibleIds: visibleIds,
        lastClickedId: 'note-c',
        shiftKey: true,
        toggleKey: false,
      }),
    ).toEqual({
      ids: ['note-a', 'note-b', 'note-c'],
      mode: 'add',
      nextAnchorId: 'note-a',
    })
  })

  it('falls back to a plain click when the previous item is no longer visible', () => {
    expect(
      sidebarSelectionIntent({
        clickedId: 'note-c',
        orderedVisibleIds: visibleIds,
        lastClickedId: 'collapsed-child',
        shiftKey: true,
        toggleKey: false,
      }),
    ).toEqual({ ids: ['note-c'], mode: 'replace', nextAnchorId: 'note-c' })
  })
})
