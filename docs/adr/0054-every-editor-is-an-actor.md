# ADR 0054: Every editor is an actor with its own undo

**Status:** Proposed. Not built. Lyle asked for agents to be treated as any other multiplayer actor, each with its own undo stack, and for the CRDT to stay in the default build (2026-11-25). The Cmd+Z question below is open.
**Date:** 2026-11-25
**Related:** [ADR 0041](./0041-typed-document-inverse-command-undo.md), inverse-command undo, which this narrows. [ADR 0048](./0048-crdt-sync-as-a-yrs-mirror.md), the yrs mirror, which this makes permanent. [ADR 0050](./0050-live-rooms-for-shared-canvases.md), live rooms. [ADR 0029](./0029-presence-acts-anchored-to-truth.md) and [ADR 0046](./0046-agent-browser-announces-its-acts.md), agent presence. [ADR 0018](./0018-cloud-sync-and-canvas-sharing.md), which first named agents as peers.

## Context

Undo has two implementations behind `shared::Undo`:

- An unshared canvas has `History`, a stack of inverse commands (ADR 0041).
- A shared canvas has `CanvasSync`, which keeps a yrs document beside the typed one and undoes through yrs's `UndoManager`, tracking only this peer's transactions (ADR 0048).

An agent's write is not a peer's. It arrives as `Event::Api`, runs one `Command` through `update`, and lands on the same stack as the person's edits. Cmd+Z undoes whichever came last. The mirror knows two origins, local and remote, so on a shared canvas an in-app agent's edits count as the person's.

The CLI already sends a session id with every call, and presence draws a cursor per agent. The app drops the session calls.

## Decision

Everyone who edits a canvas is an **actor**: the person at the window, each agent session, each remote peer. Each has its own undo stack, and an undo reverts only that actor's steps.

- **The mirror is always on.** Every canvas has a `CanvasSync`, shared or not. `History` and the `shared::Undo` switch go; undo is one mechanism.
- **A step carries its actor.** `update` passes the actor with each document step. The person is the window's actor. An API write's actor is its session. A peer's update is already its own.
- **One undo manager per local actor**, each tracking its own origin over the one yrs document. An actor with no steps has none.
- **The session id becomes the actor's identity.** `/mcp/session/open` stops being dropped: it names the actor, and presence and undo share the id.
- **Agents get the verbs people have.** `specular undo` and `specular redo` act on the calling session's stack.
- **The typed `Document` is still the app's state** and `.canvas` is still the format. Commands still run through `Document::apply`. Only where undo is kept changes.
- **The mirror learns what changed from the command.** Each command names the items it touched, and only those are copied into the yrs document. The comparison against a shadow copy stays for changes made straight to the document (a drag frame, a page reporting its address), bounded to the items in flight.
- **The yrs document lives for the session.** It is rebuilt from the `.canvas` on open. Nothing new is written to disk. A sidecar that keeps undo across a restart is a later choice, not part of this.

Conflict rules are ADR 0048's and do not change: fields merge independently, one field is last writer wins, and a sticky's text is one field.

## Open: what Cmd+Z does to an agent's work

With a stack per actor, the person's Cmd+Z no longer undoes an agent's last edit. Either:

- **Revert is explicit.** The chat panel gets "undo this run", which drives the agent's stack. Cmd+Z is only ever the person's own work.
- **Cmd+Z reaches across.** The person's stack interleaves local agents' steps, so Cmd+Z undoes the last thing that happened in this window, as today.

The first is the multiplayer rule applied evenly, and is what this ADR assumes. It needs the chat action before the change ships, or an agent's mistake has no one-key way back.

## Alternatives

**Tag agents as a second origin only on shared canvases.** Less work. Undo would then behave differently depending on whether a canvas is shared.

**Selective undo on `History`**, skipping inverses for fields another actor has since changed. ADR 0048 turned this down as a second implementation of what `UndoManager` does.

**yrs as the app's state.** ADR 0041's reasons stand: untyped reads and a rewrite of every place a command is built.

**Another CRDT.** Loro is Rust-native with built-in undo. It was not evaluated, and the web viewer and the live room speak yrs's protocol.

## What was read, and what is not known

- ADR 0048 measured the mirror at 1 ns per event when nothing changed and 0.84 ms per changed event on 2,000 stickies, by whole-document comparison. The command-named journal is expected to make that proportional to the edit. Not measured.
- Whether the yrs crate allows several `UndoManager`s on one document, each with its own tracked origin, as Yjs does, is unchecked. It is the first thing to try.
- The yrs document keeps deleted items so undo can restore them, so it grows through a session.

## Consequences

- A local agent, a remote agent and a remote person are the same kind of thing.
- One undo mechanism, where there were two behind a switch.
- Every canvas pays for the mirror: a second copy of state and a write per touched item.
- Tests that end with `assert_undo_returns_to_start` run through the mirror. ADR 0048's swap-compatible `apply`, `settle`, `undo` and `redo` are what make that a change of type, not of tests.
- `specular-sync` and yrs are in every build by design.
