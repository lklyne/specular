# ADR 0014 — Canvas stack order and the Notes/Pages sidebar

**Status:** Accepted — integration landed as PR #166. `entityOrder` mutation API (`bringToFront`, `sendToBack`, `bringForward`, `sendBackward`), keyboard shortcuts (`Cmd+[]/]`), context menus, sidebar sort-by-stack-order, and page WCV z-order restack all implemented.
**Date:** 2026-05-11
**Related:** [ADR 0003 — Page as canonical name for live web items](./0003-page-as-canonical-name-for-live-web-items.md), [ADR 0004 — Text affordances and spec extensions](./0004-text-affordances-and-spec-extensions.md).
**Supersedes premise of:** the sidebar's position-based sort (`compareSidebarPositions` in `sidebar-builder.ts`) and the implicit "edges always paint above all entities" rule.
**Origin:** Grilled out from [`docs/canvas-stacking-research.md`](../canvas-stacking-research.md) (branch `claude/research-canvas-stacking-NQ7Lh`) and the grilling session on branch `claude/plan-canvas-stacking-s1th2`.

## Context

Specular has had `entityOrder` — a flat Y.Array of entity ids in back-to-front paint order — since the JSON Canvas serializer landed. The hit-tester already iterates it (`collectBodyTargets` in `src/shared/hit-test.ts:262-291`) to resolve overlapping clicks front-to-back. The persistence layer round-trips it through JSON Canvas v1.0's node array order.

What's missing:

- **No mutation API.** Nothing calls `bringToFront`, `sendToBack`, or any equivalent. `entityOrder` is only ever written by the deserializer at load time.
- **No user-facing surface for stack order.** The left sidebar sorts by canvas position (top-to-bottom, then left-to-right, with a kind-priority tiebreaker), not by `entityOrder`. Users have no way to express "send this sticky behind that one" and no way to see the current stack.
- **Page WCV stacking is creation-order, not z-order.** `page-factory.ts` calls `addChildView(pageView)` at create time. When two pages overlap, the visually frontmost one is whichever was added last, regardless of any logical z-order.
- **Edges have no stack order at all.** They paint above everything else in `EdgeLayer`. Users coming from FigJam expect to send connectors behind shapes; that's not possible today.
- **Groups in `entityOrder` don't enforce contiguity.** A group's members can be scattered, which makes "move the group as a unit" semantically unclear.

Two facts of the rendering architecture are load-bearing throughout this ADR:

1. The canvas has three paint surfaces: `bgView` (bottom, grid/page-borders), pages (middle, multiple `WebContentsView`s), `aboveView` (top, holds *everything else* — text/sticky bodies, files, drawings, shapes, edges, annotations, chrome, marquee).
2. `LAYER_STACK` (`src/main/runtime/layer-stack.ts`) pins `aboveView` above all pages. Overlay entities therefore **always paint above pages**, regardless of any logical z-order we invent.

This means "one global stack order across all kinds" is achievable as a *data* concept but not as a *visual* concept on the current surface model. The decision below honours that.

## Decision

### 1. Data model — Option C (flat `entityOrder` + group contiguity)

`entityOrder` stays a flat Y.Array of ids, back-to-front. Three invariants:

- **Contiguity.** For every group, all of its descendants (recursive) plus the group's own id form a single contiguous run inside `entityOrder`. Within that run, each child group's subtree is also contiguous.
- **Front-of-run.** The group's own id sits at the frontmost slot of its run. `entityOrder.indexOf(groupId)` *is* the group's stack position. Members follow immediately behind.
- **Edges are first-class participants.** Edge ids interleave with entity ids in `entityOrder`. Newly created edges default to the top of the stack.

Alternatives considered:

- **A. Flat, no contiguity rule.** Group children could scatter; "move group" has no meaning.
- **B. Hierarchical (per-group child order array).** Cleanest mental model, but diverges from JSON Canvas v1.0 (flat node list) and round-tripping through other tools loses child order. Schema change to both runtime and persisted format.

Option C wins because it preserves the JSON Canvas round-trip, matches the "group is one stack slot" mental model, and the data shape is already 90% there — only the invariant is new.

### 2. User-facing terminology

- **"Stack order"** is the domain term. Used in glossary, UI copy, and ADRs.
- **`entityOrder`** stays the code/data-model term. Renaming would churn the JSON Canvas serializer and `.canvas` files for no real win.
- **Actions:** *Bring forward*, *Send backward*, *Bring to front*, *Send to back* (sentence case per CONTEXT.md).
- **Shortcuts:** `Cmd+]`, `Cmd+[`, `Cmd+Shift+]`, `Cmd+Shift+[` (Figma / Sketch convention).
- **Sidebar nav:** `↑` / `↓` move the selected row up / down (= forward / backward in stack).

Rejected alternatives: *Z-order* (code term, not domain), *Layer* / *Layers* (already overloaded — `HIT_LAYER_ORDER`, `LAYER_STACK`, `EdgeLayer`), *Height* (collides with bbox `height`).

### 3. Sidebar — *Notes* and *Pages* sections

The sidebar tree splits canvas items into two labelled sections that mirror the two interactive paint surfaces:

- **Notes** (top section) — text, sticky, document/file, drawing, shape. Rendered in `aboveView`; always paint above pages.
- **Pages** (bottom section) — live web pages. Rendered as `WebContentsView`s.

Stack-order mutations are scoped per section. Cross-section drag has no drop target. The divider surfaces the architectural constraint directly instead of letting one flat ordered list lie about cross-surface stacking.

*Note* is a sidebar grouping label only. It is not a kind in the data model and never appears in `.canvas` files. Drawings, shapes, and files are not literally notes, but they're notational — they exist to call attention to or annotate something on the canvas. "Notes" generalises better than "Sketches" or "Markup" or "Overlays".

Rejected alternatives:

- **One flat sidebar list** (Option α). Honest about `entityOrder` but dishonest about pixels — a sticky behind a page in the sidebar would still visually paint above it.
- **Three sections** (Notes + Pages + Groups). Groups go into the section their contents live in; carving them out is gratuitous structure.
- **`Overlays` / `Pages`** as the section labels. "Overlays" collides with `data-overlay-ui`, `CanvasItemChrome` (overlay UI), and the architectural sense of "overlay surface".

### 4. Mixed groups — split representation

A group's members can paint on both surfaces. A group is still one entity with one contiguous run in `entityOrder` (one stack slot), but the sidebar renders it as **two linked rows** — one in Notes, one in Pages — bearing the same id, name, selection state, and (visually) a matching colour bar or chain glyph.

- Expanding the Notes row reveals only note-children; expanding the Pages row reveals only page-children.
- Operations are unitary: send-backward from either row moves the *whole* group run as one unit, so both rows shift simultaneously within their respective sections.
- Pure-note groups appear only in Notes; pure-page groups only in Pages.
- Dissolve removes both rows; children stay where they were.
- Reparent into a mixed group: same group entity, the child slots into the group's run in its surface's section.

Rejected alternative: "frontmost child wins, group appears in one section." Hides a real cross-surface mix from the user. Violates "the sidebar shows the true landscape".

### 5. Page WCV stacking — re-`addChildView` in `entityOrder` order

On every `entityOrder` mutation, on workspace load, and on undo/redo:

1. Walk `entityOrder` back-to-front.
2. For each page entity, `win.contentView.addChildView(frameView)` then `addChildView(pageView)` — frame immediately behind its page.
3. Re-add `aboveView` (and the rest of `LAYER_STACK` above it) after pages.

This makes page-vs-page visual stacking match `entityOrder` exactly. `bgView` stays pinned at index 0 (as today).

Cost: O(pages) `addChildView` calls per order mutation. Cheap in practice. Already the pattern `applyStack()` uses for singleton overlays.

### 6. Overlay paint-order

Every aboveView body layer (`StickyBodyLayer`, `FileBodyLayer`, `ShapeBodyLayer`, `DrawingsLayer / SavedDrawingEntities`, `EdgeLayer`) iterates entities and edges in `entityOrder` order. Front-most in stack = painted last = visually on top. This is largely already true; the change is to make it a deliberate, tested invariant rather than incidental React iteration order.

### 7. Edges — in `entityOrder`, not in sidebar

Edge ids interleave with entity ids in `entityOrder` so edges can be sent forward / backward against entities (FigJam parity). However:

- Edges do **not** appear as rows in the sidebar. The sidebar is the entity / canvas-item view; edges are connective tissue.
- Stack-order mutations on edges happen on the canvas: select edge → `Cmd+[` / `Cmd+]` / right-click menu.
- Sidebar drag-reorder operates only on the entity subsequence of `entityOrder`. Edge indices are untouched by sidebar drags (**rule (i)**). Predictable, two-line implementation.
- Newly created edges default to the top of the stack.

JSON Canvas serialization: the spec keeps `nodes[]` and `edges[]` as separate arrays. The interleaved `entityOrder` (including edge ids) is persisted as a Specular extension — same pattern as `specular.textStyle` (ADR 0004). Other JSON Canvas tools round-trip the data but lose precise stack interleaving for edges. Acceptable.

### 8. Operational defaults

- **New entities** slot at the top of the stack (frontmost). Inside a group context, at the front of the group's run (just behind the group id).
- **Multi-selection mutations** preserve relative order among the selection and move the selection as a block (Figma / Sketch / Illustrator behaviour).
- **New edges** default to the top of the stack.
- **Drag-to-reparent** (sidebar drop into a group) is *out of scope* for this ADR. Initial sidebar drag supports reorder within current parent only. Reparenting is a follow-up.

### 9. Migration

Existing `.canvas` files may have groups whose members are not contiguous in `entityOrder`. On first load after this ADR lands:

1. Read `entityOrder` as today.
2. Run `enforceGroupContiguity` recursively, depth-first frontmost-first. For each group, pin the run to the **frontmost member's current index**; pull the rest backwards behind it (rule **(a₁)**).
3. Mark the workspace dirty; autosave debounce flushes the normalised order.

Deterministic, no data lost — only a reordering. Re-running the migration is a no-op. Users with git-tracked `.canvas` files will see a one-time phantom diff on first open.

## Consequences

**Enables:**

- Single source of truth for canvas stack order, exposed via mutations and reflected in the sidebar.
- Page-vs-page overlap respects user intent.
- FigJam-parity stack mutations for edges.
- A sidebar that honestly surfaces what users can and cannot do across paint surfaces.
- Undo / redo of stack-order mutations for free (Y.Doc transactions).

**Replaces:**

- `compareSidebarPositions` (position-based sort) → `entityOrder` index lookup.
- The implicit "edges paint above everything" rule → edges have a stack position like everything else (and default to the top).
- The sidebar's flat tree → two-section tree with linked split rows for mixed groups.

**Costs:**

- Real refactor of `sidebar-builder.ts` and `SidebarCanvasTree.tsx`.
- New pure module `src/shared/entity-order-math.ts` for the mutation primitives (`bringToFront`, `sendToBack`, `moveForward`, `moveBackward`, `moveBefore(id, anchorId, 'before' | 'after')`) and the `appendAtTop` helper for new-entity / new-edge creation.
- New runtime wrapper `src/main/runtime/entity-order-state.ts` that reads runtime arrays, calls the pure helpers, and writes back. Diff-sync handles Y.Doc per `src/main/runtime/CLAUDE.md`.
- `enforceGroupContiguity` helper (in the pure module) that runs after every order or group-membership mutation.
- Page WCV re-stack on every order mutation (cheap but a new code path).
- aboveView body layers must iterate in `entityOrder` order — verify or fix each one (`StickyBodyLayer`, `FileBodyLayer`, `ShapeBodyLayer`, `DrawingsLayer`, `EdgeLayer`).
- HTTP API surface for the four mutations so agents / smoke tests can drive them.
- One-time migration on legacy canvases.
- The Specular extension for edge stack interleaving — small addition to the JSON Canvas serializer.

**Out of scope:**

- **Drag-to-reparent** in the sidebar — separate ADR / PR.
- **Cross-surface visual stacking** (sticky behind page, page over drawing). Architecturally impossible on the current surface model. Future ADR can revisit by re-architecting `aboveView` content into per-z-band layers, but not in this change.
- **Z-order for annotations / comments** — they stay above all entities, as today.
- **Per-tab stack order semantics** beyond what the existing tab-switch mechanism already provides (the active tab's `entityOrder` is the one being mutated; tab switches restore the prior tab's order via the Y.Doc tab-state mechanism).
- **Bring-forward / send-backward keyboard shortcut conflicts with browser-mode reload / forward**. Resolved by binding to canvas mode only.

## Migration plan

Implementation slices, each independently shippable:

1. **Pure math module + invariant helper.** ✅ Landed. Pure helpers live in `src/shared/entity-order-math.ts` (`bringToFront`, `sendToBack`, `moveForward`, `moveBackward`, `moveBefore(id, anchorId, 'before' | 'after')`, `enforceGroupContiguity(order, groups)`, `appendAtTop`). The runtime wrapper `src/main/runtime/entity-order-state.ts` (and call sites for user-driven reorders) remains a follow-up; today `entityOrder` is regenerated deterministically by `syncEntityOrder` in `workspace-doc.ts` on every diff-sync.
2. **Sidebar sort by `entityOrder`.** ✅ Landed. Position-based sort is gone. `LeftSidebarData.sections: { notes, pages }` replaces the flat `items` array. The pure partition lives in `src/shared/sidebar-partition.ts` (unit-tested); `sidebar-builder.ts` decorates the partition with UI payloads. Mixed groups emit two rows sharing the group id, each exposing only the children on their surface. Initial render only — no drag yet.
3. **Page WCV re-stack.** ✅ Landed. `applyStack()` in `layer-stack.ts` now walks `entityOrder` between re-adding `bgView` (index 0) and the above-pages cluster, reattaching each page's `frameView` followed by `pageView`. `removePageAtIndex` marks `'stack'` dirty so the restack runs after deletes too.
4. **aboveView paint-iteration verification.** ✅ Landed. `buildCanvasLayoutData` sorts `entities` by `entityOrder` rank (stable on ties) before broadcasting. Body layers (`StickyBodyLayer`, `FileBodyLayer`, `ShapeBodyLayer`, `DrawingsLayer`) consume the same array, so React's iteration order is now `entityOrder`-derived rather than incidental. `EdgeLayer` still iterates its own array — interleaving edges into `entityOrder` is slice 7.
5. **Sidebar drag-to-reorder.** Drag handle on each row; drop zones between rows; section divider is a forbidden drop. Multi-select block move. No reparenting.
6. **Context-menu and keyboard shortcuts.** *Bring forward / Send backward / Bring to front / Send to back* on canvas right-click and `Cmd+[ ]` / `Cmd+Shift+[ ]` shortcuts (canvas mode only).
7. **Edges in `entityOrder`.** Extend `entityOrder` to accept edge ids; new edges go to the top; stack mutations on a selected edge work. Persist via Specular extension in the JSON Canvas serializer.
8. **Migration.** On `workspace-restore.ts`, run `enforceGroupContiguity` after deserialisation; mark dirty; autosave flushes.
9. **HTTP API.** Routes for the four mutations under `src/main/routes/`.

## Tests

- **Unit:** every mutation preserves the contiguity invariant (recursive across nested groups). Migration normalises scattered groups deterministically. Multi-selection block move preserves relative order. New entities and new edges land at the top.
- **Unit:** sidebar partition algorithm — pure groups go in one section, mixed groups produce two linked rows, dissolve removes both, reparent updates both.
- **Smoke:** two overlapping pages — send-backward on the front, click region resolves correctly; visually back one now on top via `addChildView`. Same test for two overlapping stickies. Same for edge-behind-sticky.
- **Smoke:** sidebar drag reorders entities; edges keep absolute indices in `entityOrder`. Verified by reading `entityOrder` before and after.
- **Smoke:** undo / redo restores stack order across tab switches.
- **Agent:** HTTP routes for the four mutations drive reorder; assertions on `entityOrder` and on sidebar broadcast.
