# ADR 0048: CRDT sync as a yrs mirror beside the typed document

**Status:** Proposed. Built as `native/crates/specular-sync` and wired into `specular-interact`; no transport yet. The user has not reviewed this ADR.
**Date:** 2026-10-27
**Related:** [ADR 0041](./0041-typed-document-inverse-command-undo.md), which removed yrs and left cloud sync needing "a new design". [ADR 0018](./0018-cloud-sync-and-canvas-sharing.md), the Cloudflare plan, which assumes a Y.Doc.
**Code:** `native/crates/specular-sync/src/{sync,fields,order}.rs`, tests in `native/crates/specular-sync/tests/sync.rs`.

## Context

ADR 0041 made the canvas plain typed structs, changed only by `Command`s through `Document::apply`. That removed the second copy of state that Yjs had forced on the Electron app, and with it any way to merge two people's edits.

It also said putting a CRDT back would mean rewriting every place a `Command` is built, 49 files. That is true only if the CRDT becomes the app's state. Every change already goes through one function, so a CRDT can sit beside the document instead.

## Decision

The `Document` stays the app's only copy of the canvas. `CanvasSync` holds a yrs document beside it and keeps the two in step.

- **Local changes go out.** After a command runs, the items it touched are copied into the yrs document, and only the fields that differ are written. The yrs updates are collected for a transport to send.
- **Peers' changes come in as commands.** A peer's update is merged into the yrs document. Observers note which items it touched, and those items are turned back into the same `Command`s the app uses (`SetRect`, `SetLabel`, `InsertEntity`, `SetOrder`, and so on), applied with no undo step. Nothing else in the app has to know where a change came from.
- **Layout.** There are four root maps. `entities`, `edges` and `annotations` map an id to the item's `.canvas` fields, one yrs key per top-level field and per key of the `specular` extension object. `notes` maps a markdown file's path to a Y.Text. Text positions use UTF-16 offsets, as Yjs does, so a JavaScript peer such as the `y-partyserver` Durable Object from the July spike can share the document.
- **Fields merge independently; one field is last writer wins.** A move and a rename of the same sticky at the same time both land. Two moves leave one of them, the same one on every peer. A `Kind`'s fields are separate keys too, so one person changing a sticky's text while another changes its size keeps both. The text itself, though, is one field: two people typing in the same sticky still overwrite each other.
- **Stack order is fractional keys.** Each entity, edge and annotation carries an `~z` order key, and the order is the sort by key and then id. A reorder rewrites keys only for the items outside the longest run already in order, so two peers reordering different items both keep their change. Two inserts at the same place can mint the same key; the id breaks the tie and the next reorder separates them.
- **Undo reverts only this peer's steps.** Undo uses yrs's `UndoManager`, tracking only local step transactions. A field a peer changed after my step keeps the peer's value when I undo. `CanvasSync<S>` carries the same before and after state per step as `History<S>`, so the selection still comes back. `apply`, `apply_from`, `settle`, `undo` and `redo` match `History`'s, so a shared canvas can swap one for the other.
- **Drags.** Frames reported with `record` reach peers as they happen but are not undo steps. A step first copies the document's state for the items it touches into the yrs document without recording it, and then writes the step. So the undo of a drag goes back to where the drag started even though peers saw every frame.

### In the app

- Each canvas's undo state is `shared::Undo`: the plain `History<Selection>`, or a `CanvasSync<Selection>` while the canvas is shared. `App.history` and a parked canvas's `history` have this type, so `gesture::apply_fitted`, undo, redo and settle go through it unchanged.
- At the end of every `update`, `shared::flush` reconciles each shared canvas against its document and returns `Effect::ShareSend { canvas, update }`. Changes made straight to the document (drag frames, a page reporting its address, a group refit, a file read again from disk) are found by comparing it with a shadow copy of the last state shared, so no call site has to report them.
- `Document::stamp()` changes on every applied command, so the comparison is skipped when nothing changed. It costs 1 ns per event while nothing changes, and 0.84 ms per changed event on 2,000 stickies. An item-level change journal would make the second number proportional to what changed.
- `Event::Share(ShareEvent::{Start, Join, Receive, Stop})` drives it. `Receive` names a canvas, so a background canvas's peer updates land too, and the canvas is written. A peer's change keeps page hosts, notes and the selection in step, as an undo does. It also lands mid-drag; the drag's next frame wins any field the two share.
- A canvas read again from its file stays shared: its undo steps are cleared, and the new contents go to peers.

## Alternatives

**yrs as the app's state** (reversing ADR 0041). It would merge everything for free, including undo. Turned down for the reasons ADR 0041 gives: a second copy, untyped reads, and 49 files to change.

**A server-ordered command log**, Figma's model: the Durable Object puts commands in order and every peer applies them in that order. Simpler and typed, but edits made offline have to be rebased by hand, and a JavaScript peer couldn't reuse Yjs tooling. Still open if the yrs mirror turns out to be too heavy.

**Selective undo on top of `History`**, skipping inverses whose fields a peer has since changed. It would mean a second implementation of what `UndoManager` already does.

## What was measured

- Nine behaviour tests, among them: a joining peer gets five fixture canvases exactly (kitchen sink, rich workspace, Welcome, 300 drawings, 200 edges, plus a note with emoji); three peers with concurrent field edits, inserts, removals, edges and comments converge; concurrent reorders and same-place inserts converge; offline edits merge on reconnect through state vectors; undo leaves a peer's rename or later move alone; undo and redo of a removal and an insert reach peers with the parent and stack position intact; a drag syncs each frame and undoes in one step; concurrent typing in one note keeps both.
- Three deliberate breaks each fail a test: leaving out the drag baseline, replacing a note's text whole instead of the changed span, and tracking peers' transactions in undo.
- On `stickies-2000.canvas`, release build, in a Linux container: publishing takes 48 ms, joining 35 ms, and the full state is 900 KB. A move sent and applied on the other peer takes 0.23 ms. Reversing all 2,000 items takes 58 ms and a 365 KB update.
- `cargo check --target wasm32-unknown-unknown -p specular-sync` passes. It has not run in a browser.

## Consequences

- A cloud transport now has something to carry. Nothing ships until the app uses `CanvasSync` for shared canvases (see below).
- Two copies of state exist again, but only for a shared canvas, and the copy runs one way through `Command`s. A peer's change the document refuses stays in the yrs document and is skipped locally with a warning. The one such case found so far, two groups moved into each other at the same time, is repaired: every peer breaks the loop at the group with the greatest id, takes the other move, and writes the result back, so all peers end the same. Any other refused change could still leave a peer's document drifting from the others.
- Document-level `extra` (view state such as `appState`) is not shared.
- A sticky's text and drawing points are last writer wins.
- The yrs document keeps deleted items so that undo can restore them, so it grows with history. Compaction is a server concern.

## Not done yet

1. **A transport.** Shell-side WebSocket to the `y-partyserver` Durable Object from the `claude/session-h8p3gr` spike, which already speaks this protocol.
2. **A way to start sharing from the UI or the CLI**, mapping a shared document id to a canvas in the shell.
3. **Awareness** (cursors, selections) over Yjs awareness, and a CRDT text field for stickies if last writer wins proves too coarse.
