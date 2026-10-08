# ADR 0043: Each canvas keeps its own state in a Space

**Status:** Proposed. Built this way in `native/crates/specular-interact/src/space/` during the Rust rebuild. The user has not reviewed this ADR.
**Date:** 2026-10-08
**Related:** [Rust native rebuild plan](../plans/rust-native-rebuild.md), task P5. [ADR 0033](./0033-user-chosen-space-folder.md), which defines the space folder. CONTEXT.md, "Tab" and "Tab context", the Electron design this departs from.
**Code:** `specular-interact/src/space.rs` and `space/`, `specular-app/src/space/`.

## Context

In the Electron app the runtime arrays hold one canvas, the active tab. Every other tab is a serialized snapshot. Switching tabs serializes the one being left and loads the other into the arrays, and the undo history of the tab being left is thrown away. A write to a background tab loads that tab into the arrays for one turn and then restores the user's. Pages are refused there, and the write stays out of undo.

Two things users notice follow from that. Undo does not survive a tab switch, and an agent writing to a background canvas cannot add a page.

## Decision

`App` owns one `Space`: the folder and every canvas in it. Each `Canvas` is whole. It has its own `Document`, its own `History`, its own camera and its own selection.

- The active canvas's document, history and session are `App`'s own fields, so code that reads the app did not change. The other canvases are parked in their `Canvas` entries.
- A switch moves three structs out and three in. Nothing is serialized, and a parked canvas keeps its undo stack.
- Every canvas in the folder is read when the space opens, not on first switch. Listing, the sidebar's counts and a `--tab` read need no I/O, and a background canvas follows changes to its file.
- A `--tab` write runs with that canvas standing where the active one does and the user's session set aside, then everything is put back. It is one undo step in that canvas's own history. It may add pages, which are hosted when the canvas is shown.
- A `--tab` read builds a one-canvas `App` from a copy of that document, so every route handler reads it unchanged.
- Ids belong to a canvas. A duplicated canvas keeps its entity ids, where Electron remaps page ids because its page hosts are global.
- The five canvas operations are one `Action::Canvas`. Their file work comes back as effects: write, rename, trash, save the index.
- The space index is the Electron app's file, `.specular/workspace-meta.json`, read and written in its shape with unknown keys kept. Canvas files are named as Electron names them.

## Alternatives

**Electron's model**: one live canvas, the rest serialized. Less memory. Turned down for the two user-visible costs above.

**Load a canvas on first switch.** It would cut the open cost for a large space. Turned down because the sidebar, the tab routes and the file watch would each need a path for a canvas that is not loaded yet.

**Keep page hosts alive for background canvases**, so a switch back is instant and a page keeps its scroll and history. Not built. It would multiply live Chromium renderers by the number of canvases.

## What was measured

Not much, and that should be said plainly.

- The log entry for this work says "Nothing was measured" about switch cost and "Nothing was run in a window".
- Tests cover it without a window: 18 on the space and 7 on the sidebar through the testkit, 8 on the tab routes, and 14 in a temp folder on what each operation leaves on disk. Scenarios `l-sidebar` and `n-chrome-session` add, rename, delete and switch canvases, and `check.py` compares the canvases they save.
- Nobody has measured open time or memory for a space with many large canvases.

## Consequences

- Undo survives a switch. Come back to a canvas and Cmd+Z still undoes what you did there.
- A background canvas has no page hosts. A switch closes every page of the canvas left and creates every page of the one entered. A page reloads when its canvas comes back and loses its scroll and its back and forward history.
- A canvas switch gives a page a new host, so its CDP socket address changes. The CLI asks for the address on every command, so its verbs keep working.
- Memory and open time grow with the whole space, not with the active canvas.
- Deleting a canvas is not undoable. Its file goes to the system trash, which is the way back.
- Canvas names are unique everywhere once trimmed. Electron refuses a duplicate only in `tab new`.
- Both apps open on one space will overwrite each other's index. Run one at a time.
- A `.canvas` file another tool adds to the folder is not seen until the space is opened again.
- Reversing this means making the tab routes, the sidebar model and per-canvas autosave work from serialized snapshots again, and giving up undo across a switch.
