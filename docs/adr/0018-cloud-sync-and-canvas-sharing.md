# ADR 0018 — Cloud sync, canvas sharing, and agents as peers

**Status:** Proposed
**Date:** 2026-06-08
**Builds on:** the existing Yjs state layer (`src/main/runtime/workspace-doc.ts`, `workspace-observers.ts`) and the asset model (`src/main/runtime/image-assets.ts`). No code has landed for this ADR; it records the intended architecture before any of it is built.
**Related:** [ADR 0003 — `Page` as the canonical name for live web items](./0003-page-as-canonical-name-for-live-web-items.md) (live pages are the entity kind with no cloud-renderable pixels), [`docs/architecture.md`](../architecture.md) (two-layer state model), [`docs/file-formats.md`](../file-formats.md) (`.canvas` and the `assets/` folder).

## Context

Specular is local-first: state lives in a `Y.Doc` (Yjs CRDT), is serialized to human-readable `.canvas` files on disk, and never depends on a server for core data. Two product wants pull against that:

1. **Cloud access for agents.** A local or cloud agent should be able to store and edit canvases without a desktop app necessarily running.
2. **Sharing + multiplayer.** A "Share" button that mints a link; anyone with the link co-edits in real time; the same link can be pasted into a cloud agent so it edits the canvas as a peer.

The owner's stated direction: **self-hosted now, a service they build and charge for later** — explicitly *not* a third-party SaaS data plane (Liveblocks et al. are out as the core dependency). Cloudflare (Workers / Durable Objects / R2) is the preferred infrastructure.

The favorable starting position is that the hard part is already done. State is already a `Y.Doc`, and a **reverse-sync path** (Y.Doc → runtime arrays → `requestLayout`) already exists in `workspace-observers.ts`, today driven by undo/redo. A network sync provider needs exactly that machinery: remote transactions look, to the runtime, like an undo applying. The forward path (`syncRuntimeToDoc`) already tags transactions with an origin (`'user'`), which we will lean on to distinguish remote/agent edits from local ones.

Two things are *not* free and shape every decision below:

- **Live page nodes have no pixels outside the desktop app.** A `page` (ADR 0003) is a live `WebContentsView` rendering a real website. The Durable Object that syncs a canvas holds *data*, not rendered pages. Anything that needs page pixels in the cloud (browser multiplayer, cloud screenshots) needs a separate render path.
- **`.canvas` portability must survive going to the cloud.** Today a `file` entity stores a local filesystem path (`image-assets.ts` writes to `assets/` and stores the path). A cloud asset is an object-storage key. If we bake either location into the file, the `.canvas` stops being portable between desktop and cloud.

## Decision

Adopt a **Yjs-over-Cloudflare** sync substrate, a **split data plane** (CRDT doc vs blob storage), an **asset-id indirection** that keeps `.canvas` portable, and a **capability-link** sharing model in which agents are ordinary authenticated peers.

### 1. Sync substrate — attach a provider to the existing `Y.Doc`

Each **canvas maps to one Durable Object**, keyed by a stable **doc id**. The DO runs a Yjs sync server (`y-partykit` on Cloudflare, with WebSocket Hibernation). Clients — desktop app, web client, agents — connect to the DO and share one `Y.Doc`; CRDT merge handles concurrent edits with no conflict resolution code.

Integration points already exist:
- The provider attaches to the doc from `getActiveDoc()` (`workspace-doc.ts`) — one site.
- Inbound remote updates flow through the existing Y.Doc → runtime observer (`workspace-observers.ts`), generalized beyond undo so remote transactions patch runtime arrays and `requestLayout()`.
- **Origin tagging is load-bearing.** `syncRuntimeToDoc` already stamps `'user'`; remote and agent edits are stamped with distinct origins so we can (a) keep them out of the local undo stack and (b) attribute presence ("edited by agent").

We treat the desktop app as authoritative-capable but not required: the DO persists the doc itself, so a canvas lives in the cloud whether or not any app instance is connected.

### 2. Split data plane — doc holds references, R2 holds bytes

**Image/video bytes never enter the `Y.Doc`.** Binary in a CRDT bloats the doc, forces every peer to download every byte into memory, and collides with Durable Object message/storage limits. Instead:

- **R2** stores asset bytes (S3-compatible; **zero egress** — the decisive cost property for a media-heavy product).
- The doc stores only an asset **reference** (id + dimensions + `objectFit`), which is tiny and merges cleanly.

This mirrors what the app already does locally (`image-assets.ts`: bytes to `assets/`, reference in the entity). The cloud swaps "`assets/` folder + path" for "R2 bucket + key" with no change to the doc shape.

Upload flow: client/agent uploads bytes via a Worker (presigned R2 PUT or proxied) → Worker returns the key → the reference is written into the file entity → propagates to all peers → peers GET bytes from R2 (CDN-cached).

### 3. Asset-id indirection — keep `.canvas` portable

A file entity stores a **stable asset id**, not a location. Location is **resolved per environment**:

| Environment | Asset id resolves to |
|---|---|
| Desktop / local | `assets/<id>.<ext>` on disk |
| Cloud | R2 object key |

The resolver is the single place the two location schemes meet — the same shape as the serializer being the only place `page` ⇄ JSON Canvas `link` meet (ADR 0003). This preserves the file-format invariant ("a `.canvas` is portable and tool-readable") and means the same canvas opens in the desktop app and the cloud service without rewriting paths. The concrete on-disk encoding (e.g. an `id` field under the entity's `specular` namespace per CONTEXT.md's extension convention) is left to implementation, but the **principle — store an id, resolve a location — is the decision.**

### 4. Sharing — capability links, agents as peers

A **share link** is a URL carrying the **doc id** plus a **capability token**: `…/c/{docId}#t={token}`. Opening it connects another peer to the same DO. The token, not the link path, is the security boundary:

- **Scoped roles** baked into the token: `view` | `comment` | `edit`.
- **Expiry and revocability**: a Worker issues and validates tokens; the grant lives in D1/KV; revoke = delete the grant.
- **Agent tokens are first-class and distinct**: narrower scope, short TTL, audit-logged, separately revocable from human share links. Granting an agent edit access is its own act, not reuse of a human session.

**An agent is just another authenticated peer.** Paste the link to a cloud agent; it opens a headless `Y.Doc` in Node, connects to the DO with the token, and mutates the maps. Edits appear live to every human on the link. The agent never renders — it only edits data — which is precisely why the CRDT route makes "paste link → agent edits" trivial.

**Presence** uses Yjs awareness, wired into the cursor layer that already exists (`presence-manager.ts`, the `agent-layer` overlay / `AgentCursorLayer`). This is an extension of existing surfaces, not a new subsystem.

### 5. Render-dependent features — explicit two-case model

Because live pages have no cloud pixels, **browser multiplayer** and **cloud screenshots** are governed by where rendering happens:

| Capability | Desktop app connected | Pure cloud (no app) |
|---|---|---|
| Edit structured content (text/shape/edge/image refs) | Full | Full |
| Edit/place via agent | Full | Full |
| Render live page nodes | Full (live `WebContentsView`) | Needs a render path: iframes (cross-origin limited) or cached R2 snapshots |
| Screenshot the canvas | **Today, via existing pipeline** (`region-capture.ts` / `frame-compositor.ts`, `/frames/screenshot-composite`) | Needs Cloudflare Browser Rendering for live pages, or composite from cached page snapshots (`agent-snapshot-cache.ts`) for structured + last-known page tiles |

The decision is to **lean on the existing app-side capture pipeline as the high-fidelity path**, and treat cloud-side rendering (Browser Rendering / cached-snapshot compositing) as an additive capability, not a prerequisite for sync or sharing.

### 6. Cost posture (guesstimate, recorded for context)

Order-of-magnitude on the Cloudflare stack; the Workers Paid floor dominates until there are real users, after which marginal cost is roughly **$1–2/user/month**, driven by Durable Object active duration (hibernation is the main lever), request volume (Yjs message chattiness — batch/debounce), then R2 storage. R2's zero egress is what keeps a media product viable. **LLM inference for agents will dwarf hosting** — hosting is the rounding error.

| Scale | Est. total/mo |
|---|---|
| Single user | ~$5 (platform floor; doc hibernates) |
| 10 users | ~$10–20 |
| 100 users | ~$50–200 |

## Alternatives considered

**A. Self-hosted Yjs server on a VPS (Hocuspocus or y-sweet).** Both are strong, open, and avoid the $5 Cloudflare floor. Rejected as the *primary* choice because the owner wants to build and own a billable service on Cloudflare, and R2's zero-egress economics specifically suit an image-heavy product. Hocuspocus/y-sweet remain viable fallbacks — the substrate decision (Yjs + provider + split blob storage) is provider-agnostic, so a move costs only the transport, not the model.

**B. Liveblocks (managed SaaS).** Best DX and a server-side mutation API ideal for agents. Rejected: it's a closed third-party data plane, antithetical to local-first-as-core and to "a service I build and charge for."

**C. Sync the `.canvas` files instead of the doc (git or S3/MinIO).** Beautifully local-first and great for *async* agent edits and audit (git gives diffable history of canvas changes for free). Rejected as the realtime mechanism: file-granular sync gives last-writer-wins / merge conflicts, no presence, no live co-editing. Kept as a **complementary** capability — git-on-`.canvas` is cheap to add later for versioning/audit and does not compete with the CRDT path.

**D. Put image bytes in the `Y.Doc`.** Simplest to reason about; rejected outright — CRDT bloat, all-peers-download-everything, DO limits, no CDN/transforms/dedup.

**E. Swap Yjs for another local-first engine (Jazz, Triplit, ElectricSQL, PowerSync, Automerge-repo).** Excellent tools, but adopting one means replacing the CRDT/state layer we already have working, with no win over attaching a provider to the existing `Y.Doc`. Rejected absent a concrete wall hit with Yjs.

**F. Raw Cloudflare Durable Objects without `y-partykit`.** The platform is converging on plain DOs; `y-partykit` is the ergonomic on-ramp. Decision: start with `y-partykit` for DX, knowing we can drop to raw DOs later **without leaving the ecosystem or changing the data model**. No third-party lock-in.

## Consequences

**Enables:**
- Cloud storage + edit access for agents with no desktop app required (data plane is the DO).
- Share-link multiplayer and paste-link-to-agent editing as outputs of the same substrate, not separate features.
- A billable, owner-operated service on infrastructure with no per-third-party data dependency.

**New to build:**
- The **token/auth layer** (issue, scope, expire, revoke; D1/KV grants) — the real work, not the sync.
- A **web canvas client** *iff* browser-based multiplayer is wanted (the desktop renderer is Electron-coupled; live pages degrade to iframes/snapshots in a browser).
- The **asset resolver** (id → local path | R2 key) and upload Worker.
- Generalizing the Y.Doc → runtime observer beyond undo to absorb remote transactions, plus origin tagging for remote/agent edits.

**Costs / accepted trade-offs:**
- **Live page fidelity splits by environment.** Full in the desktop app; degraded (iframe/snapshot) in a browser; requires Browser Rendering or cached snapshots for cloud screenshots. We accept "edit in app / view in browser" as a legitimate shipping shape rather than forcing full page fidelity everywhere.
- **A share link with an edit token is a write capability** — treated like a password, mitigated by scoping/expiry/revocation. Agent tokens are deliberately narrower and separately revocable.
- **Cloudflare's $5 Workers Paid floor** exists even at single-user scale (Durable Objects require it).

**Non-goals:**
- Choosing the exact on-disk encoding of the asset id, the token format, or the wire framing — implementation details, not decisions of record.
- Replacing local-first. Disk `.canvas` files remain a first-class, fully-functional mode; cloud sync is additive. A canvas must remain openable and editable offline with no server.
- Building cloud-side live-page rendering now. It is additive (case-by-case per §5), not a prerequisite for sync or sharing.
- A managed multi-tenant billing/quota system — that's product work downstream of this substrate decision.

## Adoption trigger

This is the target architecture, not a mandate to build it now. The natural first slice is a **spike**: attach a `y-partykit` Durable Object to `getActiveDoc()`, add an R2 upload Worker plus the asset-id resolver, and prove a headless Node agent joining a share link and editing a canvas live. Promote from spike to product when the **auth/token layer and (if browser multiplayer is in scope) the web canvas client** are scheduled — those, not the sync, gate a chargeable service. Until then, the local-first disk path remains the only shipping mode.

When adopted, add a **Cloud sync & sharing** entry to `CONTEXT.md` (doc id, Durable Object per canvas, capability link, asset id, agent-as-peer), document the resolver and token model under `src/main/` once they exist, and note the `.canvas` asset-id encoding in `docs/file-formats.md`.
