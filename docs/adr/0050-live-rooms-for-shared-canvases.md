# ADR 0050: A shared canvas goes live through a room that only relays yrs updates

**Status:** Proposed. Built in `server/`, `native/crates/specular-shell/src/share/`, `specular-interact` and the web viewer, run end to end against a local `wrangler dev`, not deployed. The user has not reviewed this ADR.
**Date:** 2026-10-28
**Related:** [ADR 0048](./0048-crdt-sync-as-a-yrs-mirror.md), the yrs mirror whose updates the room carries. [ADR 0049](./0049-view-and-comment-web-client.md), the view-and-comment slice this makes live. [ADR 0018](./0018-cloud-sync-and-canvas-sharing.md), the Cloudflare plan.
**Code:** `server/src/live-room.ts`, `server/src/routes.ts` (`docs/:doc/live`), tests in `server/test/live.test.ts`; `native/crates/specular-shell/src/share/{mod,socket,link,publish,account}.rs`, `src/app/share_run.rs`; `native/crates/specular-interact/src/{shared,share_button}.rs`, `src/panel/share.rs`; `native/crates/specular-shell/src/kit/view/share.rs`; `web/app.js` (`connectLive`).

## Context

ADR 0048 gave a canvas a yrs document beside the typed one and proved two apps converge when something carries their updates. ADR 0049 published a canvas for the web to view, polling for comments. Neither carried updates between machines.

## Decision

**One Durable Object per doc, which merges and relays and never builds the document.** A peer's socket gets the room's whole state first (every stored update merged with `Y.mergeUpdates`, empty for a new room), then every other peer's updates as they come. The room keeps updates in its SQLite storage and merges them into one row past 64. It holds no canvas logic, so a schema change in the app needs no server change. The worker authorises the socket from a redeemed connection token and passes the scope; only `edit` may send, and a `view` or `comment` peer that sends is closed with 4003.

**The first canvas into an empty room is the shared one.** An app that connects to an empty room shares its canvas; one that connects to a room with state joins it, replacing its document (`specular share join` makes a new canvas for it). After a dropped connection the app takes the room's state and sends everything it holds, so edits made offline reach the room.

**The desktop publishes in Rust.** Copy link in the Share popover publishes the active canvas the first time (anonymous sign-in, doc, assets, version, the three capability links) on a thread of the shell, then connects through the edit link. The owner's session and which doc each canvas file became stay in `~/.specular/share.json`, the format `share.ts` writes. `SPECULAR_SHARE_SERVER` picks the server.

**The web viewer follows the room.** It joins the room's state, or the first update into a room that was empty, and from then on shows the live canvas instead of the published version. It still cannot edit (ADR 0049).

## Consequences

- Page snapshots are not published by the Share button yet: the publish thread cannot reach the page hosts, so the web shows a page as its card until `share.ts publish` adds snapshots.
- A share link revoked on the server ends the live share on the next connection: the room refuses it and the popover says why under Copy link.
- Nothing is deployed. The deployed worker needs the `LiveRoom` Durable Object binding and its migration (`server/wrangler.jsonc`).
