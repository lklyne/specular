# ADR 0027 — Sync sets decouple navigation sync from groups

**Status:** Accepted
**Date:** 2026-07-06
**Related:** [ADR 0003 — Page as canonical name for live web items](./0003-page-as-canonical-name-for-live-web-items.md).
**Supersedes:** the per-page `linked` boolean and its group-scoped peer partition.

## Context

"Link" meant: navigating one page drives its peers. It was a per-page `linked: boolean`, and the peer set was scoped by group — `linkedPeersOf` matched pages that were `linked` **and** shared the source's `groupId`, with all group-less pages sharing one implicit bucket. Consequences:

- You could not have two independent linked sets on the canvas without grouping them. Two loose linked pages always synced with every other loose linked page.
- Grouping silently implied syncing, conflating two unrelated concerns.
- The word "link" collided with the edge-based `link_pages` MCP tool (which creates a visual **Edge**, not navigation), and with the JSON Canvas `link` node (which *is* a page).
- The toggle was scattered across four surfaces: canvas context menu, right-details-panel button, a single-page IPC, and a selection IPC.

Multi-page selection (the popup toolbar) made independent sets the natural ask: select N pages → sync them; select a different M → sync those, independently.

## Decision

**A "sync set" is a shared `syncId` string on 2+ pages, independent of groups. One surface, one field.**

1. **`syncId: string | null` replaces `linked: boolean`.** Membership is "shares a non-null `syncId`." Peers are pages with the same id — `groupId` is no longer consulted. Grouping no longer implies syncing; a sync set can span groups. The field persists on the JSON Canvas `link` node and round-trips through undo like any page field (`PAGE_DOC_FIELD_SET` enforces it).

2. **Toggle-merge is the only verb.** `setSyncForSelection(pageIds)` (in `navigation-sync.ts`): if the whole selection already shares one set, clear it (unsync); otherwise mint one id and stamp every selected page. A selection under two pages is a no-op — sets of one are meaningless. Any set left below two members after a membership change auto-dissolves.

3. **One surface.** The multi-select page popup's chain-icon button is the only entry point. The context-menu item, the right-panel button, and the single-page toggle IPC are deleted. Display honesty comes from `isPageSynced` (has a live peer), not the raw id, so a lingering orphan id reads as unsynced.

4. **"Sync" not "link."** The user-facing concept is *sync* (the icon stays a chain). This frees "link" for the edge concept (`link_pages`) and removes the word overload.

Migration is not handled — old `.canvas` files with `linkedBrowsing`/`linked` load as unsynced. Acceptable: sync is cheap to re-establish and the feature had no committed users.

## Consequences

- Independent sync sets coexist freely; the common multi-breakpoint / component-states layout paths mint one shared `syncId` per generated cluster.
- Scroll sync rides the same `syncPeersOf`, so it decoupled from groups for free.
- Visualization of *which* set a page belongs to is deferred — today a page only knows it is synced, not to whom. A future per-set identity (color / connector) can read `syncId` without touching this model.
