# Cloud sync spike — implementation plan

Executes the first slice of [ADR 0018](../adr/0018-cloud-sync-and-canvas-sharing.md): the HTML prototyping loop end-to-end, built account-shaped-without-accounts, verified entirely against local Workers emulation (miniflare / `wrangler dev`). No Cloudflare account, deploy, web client, or snapshot work is in scope.

## Recorded implementation choices

- **The synced unit in the spike is the workspace doc, not one doc per canvas.** The runtime owns exactly one `Y.Doc` per process and tabs are a data slice inside it (`workspace-doc.ts`), so the spike maps one Durable Object to one *workspace*; every tab's `.canvas` carries the same `specular.server` docId. ADR 0018's one-DO-per-canvas granularity requires splitting the runtime doc per tab — a recorded follow-up, not spike work.

These are the owner's calls plus their consequences; steps below assume them.

- **`server/` workspace package.** All cloud code — Worker, `YServer` Durable Object, better-auth setup, D1 schema/migrations, grant + redemption endpoints — lives in a `server/` folder as a pnpm workspace package in this repo. Protocol types shared with the desktop app go in `src/shared/` (which stays side-effect-free; `server/` may import from `src/shared/`, never the reverse).
- **`y-partyserver`, not `y-partykit`.** PartyKit was acquired by Cloudflare; `y-partyserver` (in the `cloudflare/partykit` monorepo) is the maintained successor and extends `DurableObject` directly. Hibernation works as of 2.1.0. Doc persistence is **not** automatic: wire `onLoad`/`onSave` to DO storage in the same step the DO is introduced. Client side uses its `YProvider`.
- **Cloud binding lives in the `.canvas` file as a top-level `specular.server` block** — `{ docId, url }`, written when the canvas is first published. In-file (rather than a sidecar) keeps the file-system-is-the-data-model principle: the binding is diffable, portable, and agent-readable. Tokens and device credentials are **never** in the file (token = security boundary; docId is not a secret). Consequence to handle: a duplicated `.canvas` file would carry the same docId, so main keeps a docId → file-path registry; opening a second path claiming a known docId prompts to **fork** (mint a fresh docId) rather than silently double-syncing one DO. Document the block in `docs/file-formats.md` when it lands.
- **First-attach and reconnect follow CRDT no-data-loss practice.** Never wholesale-replace state in either direction. First publish: push the local doc's full state as an update into the empty DO. Every subsequent connect (including after offline edits on multiple devices): standard Yjs sync protocol state-vector diff exchange — both sides end with the union; Yjs merge is idempotent and commutative. The DO is the rendezvous point, not an authority that overwrites; deletion of cloud state is only ever an explicit owner action. Cover the divergent-offline-edits case with an integration test.
- **Auth skeleton is the first PR.** better-auth on Workers/D1 (Drizzle adapter, migrations) with the `anonymous` and `apiKey` plugins — the fiddliest dependency, so everything else builds on a working skeleton.

## Steps (one PR each, tracer-bullet order)

Each step lands with its own tests and passes `pnpm typecheck` + `pnpm test:unit` (+ `pnpm test:integration` when it touches `src/main`). Server-side tests run the Worker under miniflare (`@cloudflare/vitest-pool-workers`).

1. **`server/` scaffold + better-auth skeleton + harness.** Workspace package, wrangler config (DO, D1, R2 bindings), better-auth with anonymous + apiKey plugins, Drizzle migrations, and a `bootServerHarness()` (miniflare) mirroring `bootWorkspaceHarness()`. Test: anonymous sign-up issues a principal; apiKey mint/verify round-trips.
2. **`YServer` DO per canvas with persistence.** Doc-id-keyed DO, `onLoad`/`onSave` wired to DO storage, hibernation on. Test: two headless Yjs clients converge; all clients disconnect and a fresh client still loads the persisted doc.
3. **Grants + capability links + redemption.** D1 grant rows owned by the principal (`{grantId, docId, scope, expiresAt, createdBy}`), one durable link per scope, reset/revoke endpoints, link redemption → short-TTL connection token, and token check in the DO's WebSocket upgrade. Test: scope enforcement (view can't write), revoke and reset cut off live/derived access.
4. **Desktop provider attach.** `YProvider` on `getActiveDoc()`; generalize the Y.Doc → runtime observer beyond undo so remote transactions patch runtime arrays + `requestLayout()`; origin-tag remote edits out of the local undo stack; publish flow writes the `specular.server` block + docId registry with the fork-on-duplicate guard. Integration tests: remote edit round-trip, undo isolation, divergent-offline merge, duplicate-file fork prompt.
5. **Asset upload + resolver.** Content-addressed R2 upload endpoint (auth: connection token), and the asset resolver (asset id → local `assets/` path | R2 URL) as the single seam per ADR 0018 §3. Test: same bytes dedupe to one key; desktop and server resolve the same reference.
6. **Headless agent client + HTML loop.** `specular connect <link>` CLI verb: redeem link, join DO headless, write an HTML file entity (upload bytes, repoint content hash). End-to-end test: agent writes → connected peer's doc carries the new hash → (desktop) iframe reload path fires.
7. **Share popover (dev-flagged).** Toolbar-anchored per ADR 0018 §4b: Copy link + scope dropdown (default comment), first-copy publish moment with "Syncing…" state, active-link list with reset/revoke, shared-state indicator. Flag off by default.
8. **Docs + integration PR.** `docs/file-formats.md` (`specular.server` block), `docs/architecture.md` (server layer), CONTEXT.md deltas, and the final integration PR into the feature branch.

## Verification bar

- Steps 2–6 each prove their piece under miniflare with no network; step 4's coverage satisfies the test-contract rule for new runtime mutators (forward/reverse sync, one transaction per mutation, clean undo round-trip).
- The end state demonstrable locally: `wrangler dev` + desktop app + `specular connect` in three processes, HTML loop round-tripping live.
- Deploy to a real Cloudflare account is deliberately absent; it is a manual follow-on once reviewed.
