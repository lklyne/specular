# ADR 0024 — The entity-kind registry spans runtime state and persistence

**Status:** Accepted
**Date:** 2026-07-03
**Related:** [ADR 0019 — Canvas as a document](./0019-canvas-as-document-cli.md) (this ADR finishes what 0019 started), [ADR 0016 — Tools as a capability registry](./0016-tools-as-capability-registry.md), [ADR 0025 — Single workspace mutation seam](./0025-single-workspace-mutation-seam.md).
**Origin:** The deepen pass of the architecture audit (`docs/audit/deepen-3673a35.md`, candidate 1). Two independent explorations converged on the same finding: `EntityKindDefinition` is deep on the headless apply path and absent everywhere else.

## Context

ADR 0019 introduced `EntityKindDefinition` (`src/main/entities/contract.ts`) so the CLI/HTTP mutation surface dispatches per-kind behavior through one registry. That adoption stopped at the apply path:

- `serialize` and `defaultSize` are declared on the interface but **never dispatched** — the serializer keeps its own 6-branch if/else (twice: serialize and deserialize) and create-time sizing lives in `footprintForItem`. Dead interface surface.
- Runtime state hand-enumerates the kind list: the `[...textEntities, ...fileEntities, ...drawingEntities, ...shapeEntities]` fan-out is copy-pasted ~8× across scene building, sidebar building, selection, and graph projection.
- Interactive delete (`delete-selection.ts`) hand-buckets into 7 arrays instead of calling `getEntityKind(kind).delete` — a second delete path that can drift from the one agents use.
- Reverse sync (`syncDocToRuntime`) hand-lists both the kinds and each kind's persisted properties; a newly persisted field silently fails to restore on undo unless someone remembers the allow-list.

Measured cost: adding one entity kind touches **~18 sites across ~14 files; exactly one is registry-driven.**

## Decision

Extend `EntityKindDefinition` in place — one registry, one interface — so that runtime state, persistence, and iteration all dispatch through it. Specifically:

1. **One interface, not siblings.** New members go on `EntityKindDefinition`, not a parallel `RuntimeKindDefinition` (two registries keyed by the same names is a new stay-in-lockstep bug class) and not per-concern micro-registries (N registration sites is the disease being cured).
2. **All six kinds register:** page, text, file, group, drawing, shape. Page (WebContentsView-backed) and group (derived bbox) implement the same interface by wrapping their bespoke code; the interface is shaped to allow it (entity access as methods, not raw array fields). Consumers loop registered kinds with zero `if (kind === 'page')` branches.
3. **Edge does not register.** Edges are connective tissue, not canvas items (CONTEXT.md): no width/height, no sidebar row, they reference two entities. Forcing them in waters the interface down to a lowest common denominator. Edge keeps its own serialize/delete path and remains an explicit special case in `entityOrder`.
4. **Reclaim the dead members.** `serializeToJsonCanvas` and the deserialize loop dispatch through the registry's `serialize`/`deserialize`; `footprintForItem` dispatches through `defaultSize`.
5. **One field list drives both persistence directions.** Each kind declares its persisted fields once; persist (runtime → Y.Doc) and restore (Y.Doc → runtime, the undo path) both derive from that declaration. The "forgot to extend the restore allow-list" bug class becomes structurally impossible.
6. **Shared helpers, explicit modules — no descriptor framework.** CRUD consolidation uses plain helpers (`applyPatch(entity, patch, fields)`, default persist/scene projections); each kind's module keeps its genuine divergence as ordinary readable code (file's renderer inference, shape's sizing). We explicitly rejected generating create/update/persist/scene entirely from field metadata: the descriptor language would grow into a mini-framework and debugging would mean decoding a generator.
7. **Renderer-side per-kind capabilities live in an exhaustive shared table**, not the main-process registry. `Record<CanvasEntityKind, { hasChrome, hasAnchors, minSize, aspectMode }>` in `src/shared/`, consumed by hit-test and the pointer router (layer rules forbid renderer → main imports). TypeScript exhaustiveness makes a new kind a compile error there — forced decision, not silent default.
8. **One generic interactive update channel.** `canvas-update-entity { kind, id, patch }` dispatches through the registry's `update` — the same path CLI/HTTP already trusts — replacing the per-kind channel + preload method pairs.

### The false-abstraction line

Not unified, deliberately: the per-kind React components (popups, body layers) and genuinely kind-specific UX config beyond the four capability booleans. That per-kind knowledge is real — a page has URL chrome and no aspect lock, a drawing has no anchors — and flattening it into one interface would be a `Record<kind, config>` that ADR 0016 already rejected for tools. The file-renderer plugin registry (`src/main/plugins/`) is an orthogonal axis (extension → renderer, *within* the file kind) and is untouched.

## Consequences

- "Add a kind" collapses to: one `entities/builtin/<kind>.ts` handler, one shared-capability-table row (compile-enforced), and the kind's React surfaces. The other ~14 sites become registry loops.
- The reverse-sync layer — flagged in `src/main/runtime/CLAUDE.md` as "a bug here can lose user work silently" — gets one owner per kind and a single field-list source of truth.
- Test surface: register a fake kind in a unit test and assert it round-trips through persistence, undo, stack order, and scene projection — replacing setups that wire a 20-field ref bag.
- Interactive and headless delete converge on one path; the interactive-vs-agent drift class closes.
- Implementation plan: `docs/plans/deepen-runtime.md`.
