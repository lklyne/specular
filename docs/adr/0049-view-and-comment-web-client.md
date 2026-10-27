# ADR 0049: The view-and-comment web client runs the app's own crates

**Status:** Proposed. Built as `native/crates/specular-web`, `web/` and `server/`, run end to end against a local `wrangler dev`, not deployed. The user has not reviewed this ADR.
**Date:** 2026-10-27
**Related:** [ADR 0018](./0018-cloud-sync-and-canvas-sharing.md), the Cloudflare plan this builds the first slice of. [ADR 0048](./0048-crdt-sync-as-a-yrs-mirror.md), the yrs mirror that live editing would use later. [ADR 0042](./0042-update-view-render-loop-effects-only-io.md), the update/view/render loop the viewer drives.
**Code:** `native/crates/specular-web/src/{viewer,web,keys,bundle}.rs`, tests in `native/crates/specular-web/tests/viewer.rs`; `server/src/{published,comments,routes}.ts`; `web/app.js`; `server/scripts/share.ts`.

## Context

ADR 0018 sketched sharing a canvas as a Yjs document in a Durable Object, with pages shown in the browser as their last screenshot. The July spike built the server half of that (auth, capability links, assets) but no web client, and its sync half assumed the Electron app's Y.Doc, which ADR 0041 removed.

The first thing people asked for is narrower: send someone a link, let them look at the canvas and leave comments, and bring the comments back to the desktop. That needs no live editing and no CRDT.

The Rust core (`specular-doc`, `-interact`, `-scene`, `-compositor`) compiles for `wasm32-unknown-unknown`.

## Decision

**The web client is the app.** `specular-web` holds a real `App` and drives it with `update`, as the shells do. The scene is `specular_scene::view` and the compositor draws it, on WebGPU or, where a browser has none, WebGL2. There is no second renderer in TypeScript, and a canvas looks the same on the web as on the desktop, comment marks and the comment composer included. What a shell would do for an `Effect` is answered from the published bundle: a page is its snapshot, an image or a Document is its published file, and a page asked for the element under a point has none, so a comment there is on the point.

**A published version is a manifest of content-addressed assets.** The `.canvas` text, one PNG per page and every file the canvas names are uploaded to R2 by hash, and the manifest names them. Publishing again adds a version; a link always shows the newest. Nothing about the desktop's file format changes.

**What a viewer may do is held in the viewer, twice.** Input is filtered first: in pan mode a drag moves the camera and a click only focuses a comment, keys reach the app only while a comment is being written, and undo never does. After every input the document is compared with the last accepted one. A new annotation is kept when the link allows comments and goes out to the server; any other change is put back. The server checks scope again on every write, so a modified client can do no more than its link allows.

**Comments are stored as the annotations a `.canvas` holds.** The server stamps author, time and status, and adds `authorName` and `via: "web"` as unmodeled fields, which the Rust document keeps. Bringing comments home is a merge by id into the file's `annotations`.

**Polling, not a socket.** The page asks for the comment list every five seconds. A Durable Object can replace this when live editing arrives, and the yrs mirror of ADR 0048 is how it would.

**The spike's server is reused without its Yjs Durable Object.** better-auth principals, docs, capability links, redemption and assets are unchanged; versions and comments are two new D1 tables.

## Alternatives

**A TypeScript viewer** (canvas 2D or a WebGL library). Smaller to download, but a second renderer to keep in step with every kind, style and comment mark. Turned down because the Rust crates already run in the browser.

**Ship the CRDT first and treat a viewer as a read-only peer.** Gives live updates for free, but needs ADR 0048 wired into the app and a Durable Object speaking yrs before anyone can share anything. The view-and-comment slice does not depend on it and can move onto it later.

## Consequences

- The WebAssembly is 5.9 MB for a browser with WebGPU and 7.9 MB for one without, which also carries wgpu's WebGL2 backend and shader translator; `web/build.sh` builds both and the page loads the one it can use. The fonts in it are the bundled ones cut to Latin, Greek, Cyrillic and common symbols (`specular-compositor/fonts/web`), since a browser lends WebAssembly no system fonts to fall back on anyway.
- The browser draws SVG files, not `resvg`: the viewer asks the page for a raster at the size the app wants, as the desktop's loader is asked, and the page draws it with an `<img>`, which runs no script and fetches nothing. That costs no WebAssembly (`resvg` and its text stack were 3.4 MB) and sets SVG text in the viewer's own system fonts. An animated GIF shows its first frame.
- Pages are only as fresh as the last publish. Their snapshots come from the running app's `/pages/screenshot`; with the app closed a page publishes as an empty frame.
- On a touch screen one finger acts as the mouse and two pan and pinch-zoom. Text typed through an input method (IME) does not reach a comment.
- Publishing is a script (`server/scripts/share.ts`) until the desktop has a Share button. It keeps the owner's session in `~/.specular/share.json`, never beside the canvas.
- `std::time::Instant` panics on wasm32, so the compositor and `specular-core` use `web-time`'s, which is std's own type on every other target.
