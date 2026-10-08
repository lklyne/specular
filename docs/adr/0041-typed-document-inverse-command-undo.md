# ADR 0041: A typed document with inverse-command undo, and no yrs

**Status:** Proposed. Built this way in `native/crates/specular-doc` during the Rust rebuild. The user has not reviewed this ADR.
**Date:** 2026-10-08
**Related:** [Rust native rebuild plan](../plans/rust-native-rebuild.md), tasks F2 and F3. [ADR 0018](./0018-cloud-sync-and-canvas-sharing.md), which assumes a Y.Doc. [ADR 0023](./0023-note-content-in-ydoc-for-undo.md) and [ADR 0025](./0025-single-workspace-mutation-seam.md), the Electron decisions this replaces in Rust.
**Code:** `native/crates/specular-doc/src/{document,command,history,entity}.rs` and `src/canvas/`.

## Context

The Electron app keeps workspace state twice. A Y.Doc holds what is saved and undone, runtime arrays hold what the app reads, and forward and reverse sync keep the two in step. Yjs is there for undo. Nothing shipped uses it for merging.

The Rust spike copied that shape with yrs. In Rust it meant string-keyed map reads that fail at run time, plus typed structs as a second copy to keep in sync.

## Decision

The document is plain typed structs, and there is one copy.

- `Document` holds entities, edges and annotations. `Entity` has an id, a rect, a label, an optional page anchor, an optional parent and a `Kind`. `Kind` is an enum of six: `Page`, `Text`, `File`, `Group`, `Drawing`, `Shape`.
- Every change is a `Command` value passed to `Document::apply`, which returns the command that undoes it. There are 16 variants, `Batch` among them. A refused command, or a batch with one refused member, leaves the document as it was.
- `History` is a separate struct holding two stacks of inverses. `Document::apply` alone records no undo step, which is how a load and a drag's unfinished frames change the document without one. One user action is one command or one batch, so one step.
- `History<S>` also carries the caller's state from each side of a step. The app's `S` is the selection, so undo and redo put the selection back too.
- Ids are the `.canvas` id strings, not slot keys. Undo has to restore the same identity, and edges, parents and anchors name ids by string on disk.
- Commands never cascade. Deleting an entity with its edges is a batch the caller builds. A reference may dangle, because files from other tools contain dangling references.
- Rects are `f64`, as JSON numbers are. An `f32` rect changed values on a load and save.
- Every struct has an `extra` map for JSON this code does not model. A node the reader cannot type at all is kept as raw JSON and written back.
- The `.canvas` writer is canonical, not byte-preserving. It writes what the Electron writer would.

Markdown Document text follows ADR 0023 without a Y.Doc. `Document` holds a transient `notes` map, changed by `Command::SetNote` and never written to `.canvas`. A finished edit is one history step.

## Alternatives

**Keep yrs.** It gives undo for free and leaves a path to CRDT sync. It costs the second copy and the sync code between the copies, which is most of what `src/main/runtime/space-*.ts` does in the Electron app. Turned down because nothing uses the merge.

**Slot-map keys for entities**, as the plan first sketched. Turned down in F2 for the identity reason above.

**Snapshot undo**, where each step stores the whole document. Simpler than inverses and it would work at this size. Not tried. Inverses were already cheap to write, one per command, and a step stays small for a drawing with thousands of points.

## What was measured

- `tests/canvas_repo.rs` loads and saves every `.canvas` under `tests/integration`, `resources/starter-space`, `native/fixtures` and `native/crates` and compares JSON values. `rich-workspace.canvas`, `Welcome.canvas` and `kitchen-sink.canvas` save byte for byte.
- Scenarios `f2` and `f3` change one shape in a file Electron wrote and check that one field changes in the saved file.
- One round-trip test applies every command and its inverse. The cleanup pass mutation-checked all 59 tests in `specular-doc` and `specular-core`. Six passed under a break their name described and were strengthened.
- No speed or memory comparison against yrs was made. The yrs document was deleted in F4 before anything measured it.

## Consequences

- Adding a kind is adding an enum variant. The compiler then lists every `match` that needs an arm. The audit counted 40 matches on `Kind` and no wildcard hiding a missing arm. This replaces the entity-kind registry and the capability table.
- There is no CRDT. Cloud sync (ADR 0018) would need a new design. The command log is the natural place to attach it, and that is an idea, not a tested one.
- Two apps cannot share undo history, and they do not paste each other's entities. The Rust clipboard payload is `specular:canvas:` plus a small `.canvas`, not Electron's.
- Reversing this means putting a CRDT under every place a `Command` is built (49 files in `specular-interact` and `specular-api` name one) and bringing back a second copy of state. Every later crate was written against `apply` returning an inverse.
- The first save of an Electron file is not a rewrite. Key order is Electron's, and a text keeps the size it was read with unless the session changed it.
