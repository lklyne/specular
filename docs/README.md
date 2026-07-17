# docs/ map

This is the table of contents for `docs/` and `docs/adr/`. Read the one-line summary here first; open the actual file only when working in that area.

## ADRs

- [0001 — Click-to-enter frame focus replaces rasterization-dependent gate](adr/0001-click-to-enter-frame-focus.md) — aboveView is the single input authority; click enters native focus instead of waiting on bgView rasterization.
- [0002 — Canvas-anchored overlay UI in aboveView](adr/0002-canvas-anchored-overlay-ui.md) — entity chrome/menus render as aboveView overlays anchored to canvas coordinates, not embedded page chrome.
- [0003 — `Page` as the canonical name for live web items](adr/0003-page-as-canonical-name-for-live-web-items.md) — renames `frame` → `page` across types, IPC, components, and docs.
- [0004 — Text affordances and the Specular spec-extension convention](adr/0004-text-affordances-and-spec-extensions.md) — splits Text/Sticky note/Document into one `text` kind with a `textStyle` field, plus the `specular.*` JSON Canvas extension convention.
- [0005 — Unified `Tool` concept](adr/0005-unified-tool-concept.md) — merges pendingPlacement/AnnotationMode/inspect into one `activeTool: Tool` discriminated union.
- [0006 — Unified comment tool (subsumes region-select)](adr/0006-unified-comment-tool.md) — one comment tool handles element/point clicks and region drags; amended by 0029 for page-anchored regions.
- [0008 — Unified canvas-item popup, selection-driven and tool-driven](adr/0008-unified-canvas-item-popup.md) — one `CanvasItemPopup` component replaces per-kind menu implementations.
- [0009 — Tool variants live in popup state, not in the `Tool` union](adr/0009-tool-variants-in-popup-state.md) — `shapeKind`/`brushType` move out of the `Tool` union into popup-managed tool defaults.
- [0010 — Main is the sole shortcut dispatch site](adr/0010-main-as-sole-shortcut-dispatch-site.md) — **Proposed** — consolidates keyboard shortcut handling (currently split across 3 locations) into main.
- [0011 — Page focus respects native shortcuts](adr/0011-page-focus-respects-native-shortcuts.md) — **Proposed** — a keyboard-focused page should see native shortcuts (Cmd+Z etc.) before app-level handlers.
- [0012 — Alignment guides are visual-only](adr/0012-alignment-guides-are-visual-only.md) — FigJam-style alignment guides render but never pull; grid-snap remains the only magnetic drag force.
- [0013 — Popup menus v2: palette, text size, cross-kind morph, toolbar regrouping](adr/0013-popup-menus-v2.md) — **Proposed** — locks down popup visual design decisions left soft by ADR 0008/0009.
- [0014 — Canvas stack order and the Notes/Pages sidebar](adr/0014-canvas-stack-order.md) — **Proposed** — flat `entityOrder` + group-contiguity invariant defines front-to-back stacking; sidebar splits into Notes/Pages sections.
- [0015 — Auto-layout groups](adr/0015-auto-layout-groups.md) — **Proposed** — Figma-style auto-layout: selected group packs children into row/grid with reorder-on-drag.
- [0016 — Tools as a capability registry](adr/0016-tools-as-capability-registry.md) — **Proposed** — unifies per-tool axes (enablement, palette, cursor, popup, bindings) behind one registry, extending 0005's identity unification.
- [0017 — Scroll does not pan the canvas in browser mode](adr/0017-scroll-does-not-pan-the-canvas.md) — plain wheel/trackpad scroll only pans in canvas mode; browser mode scroll stays native to the page.
- [0018 — Cloud sync, canvas sharing, and agents as peers](adr/0018-cloud-sync-and-canvas-sharing.md) — **Proposed**, no code landed — architecture sketch for server-optional cloud access to Y.Doc-backed canvases.
- [0019 — Canvas as a document: verb-primary CLI over one entity-kind registry](adr/0019-canvas-as-document-cli.md) — CLI surface becomes verb-first (create/move/delete) dispatched through one `CanvasEntityKind` registry instead of per-kind commands.
- [0020 — Delete Browser Mode for Focus Selection](adr/0020-delete-browser-mode-for-focus-selection.md) — removes the separate tab-based Browser mode; replaces it with an ephemeral Focus selection camera command.
- [0021 — Focus Session as a First-Class Concept](adr/0021-focus-session-as-first-class-concept.md) — promotes focus selection from a presentation-only camera command to a stateful session concept.
- [0022 — Pages adopt select-first / interact-second interactivity](adr/0022-pages-select-first-interact-second.md) — a single selected page is no longer automatically interactive; selecting and interacting become separate steps.
- [0023 — Markdown note content mirrored into the Y.Doc for undo](adr/0023-note-content-in-ydoc-for-undo.md) — note/document text edits are mirrored into the Y.Doc so Cmd+Z undoes edits instead of destroying the note.
- [0023 — Renderer-owned camera and GPU-composited pan/zoom](adr/0023-renderer-owned-camera-gpu-panzoom.md) — **Rejected** — attempted and abandoned; moving camera/pan-zoom ownership to the renderer gave no felt improvement and measured worse; kept as a record of what not to retry.
- [0024 — The entity-kind registry spans runtime state and persistence](adr/0024-entity-kind-registry-spans-runtime-and-persistence.md) — extends the ADR 0019 registry so `serialize`/`defaultSize` are actually dispatched instead of dead interface surface.
- [0024 — In-process integration suite replaces the Electron smoke suite](adr/0024-in-process-integration-testing.md) — replaces the slow/ungated Electron smoke suite with an in-process integration suite that exercises real IPC handlers.
- [0025 — Single workspace mutation seam](adr/0025-single-workspace-mutation-seam.md) — one owner for the dirty→autosave→layout→undo-boundary ritual instead of it being hand-sequenced at every mutator.
- [0026 — Arrange tidies in place; distribute folds in](adr/0026-arrange-preserves-footprint.md) — splits Arrange (repack to reading order) from Distribute (even gaps in place, preserves layout); retires the standalone Distribute button/verb.
- [0027 — Sync sets decouple navigation sync from groups](adr/0027-sync-sets-decouple-navigation-sync-from-groups.md) — replaces the per-page `linked` boolean (scoped by group) with independent sync sets, so linked pages no longer need to share a group.
- [0028 — Retire the chrome-header slot model](adr/0028-retire-chrome-header-slot-model.md) — removes the unbuilt chrome-header band from entity rects (ADR 0002 §1); entity rect == body rect.
- [0029 — Page-anchored entities (the "hook to a page" utility)](adr/0029-page-anchored-entities.md) — generalizes URL-gated page-anchoring (pioneered by annotations) into a reusable utility for any entity kind tied to a page's content.
- [0030 — Element attachment](adr/0030-element-attachment.md) — page-anchored items carry a derived DOM-element reference and render corrected by its live position, so ink survives page reflow; canvas coords stay authoritative, attachment is outside undo, never a visibility gate.

## Docs

- [architecture.md](architecture.md) — process model, IPC/HTTP surfaces, and the two-layer (Y.Doc + runtime) state model.
- [architecture-audit.md](architecture-audit.md) — how to run a simplify/deepen audit pass without gutting load-bearing nuance; invariant preamble to paste into audit runs.
- [canvas-motion-research.md](canvas-motion-research.md) — research note on canvas-anchored overlay animation jitter and how to make motion smooth without per-frame IPC.
- [canvas-stacking-research.md](canvas-stacking-research.md) — historical research note that seeded ADR 0014 (canvas stack order / sidebar tree); read the ADR instead unless doing archaeology.
- [cli-architecture-review.md](cli-architecture-review.md) — tradeoff analysis of the MCP tool-schema approach vs a CLI-first agent interface.
- [development.md](development.md) — prerequisites, dev commands, env vars, and release process.
- [divergence-input-authority.md](divergence-input-authority.md) — running status/phase table tracking where the input-authority refactor implementation diverged from its plan.
- [file-formats.md](file-formats.md) — .canvas (JSON Canvas v1.0) spec, spaces/vault model, and persistence details.
- [input-authority-audit.md](input-authority-audit.md) — live classification table of every canvas gesture path (router-owned vs native vs visual-only) post input-authority refactor.
- [interaction-layer.md](interaction-layer.md) — architecture spec for gestures/overlays/focus handoffs; §6 load-bearing invariants must be read before touching input.
- [offscreen-rendering-research.md](offscreen-rendering-research.md) — research into GPU offscreen rendering (e.g. Ultralight) as a strategy for scaling many live web frames.
- [pan-zoom-perf-unknowns.md](pan-zoom-perf-unknowns.md) — *(not on this branch, present on main)* — deep research sweep of Chromium/Electron internals for pan/zoom perf options beyond bitmap-freeze/WCV-fold; corrects part of the ADR 0023 postmortem.
- [perf-tracing.md](perf-tracing.md) — *(not on this branch, present on main)* — how to record and read Chromium performance traces of Specular via UI or HTTP API.
- [product.md](product.md) — product philosophy: who Specular is for, what it is, and the open/local-first core beliefs.

Subdirectories (see files within for detail, not indexed individually here):

- `adr/` — architecture decision records; see ADRs section above.
- `agents/` — agent-facing process docs (issue tracker usage, triage labels, domain-doc conventions).
- `plans/` — in-flight and historical implementation plans for specific features/refactors.
- `explorations/` — early-stage design explorations not yet promoted to a plan or ADR.
- `audit/` — output of architecture/codebase audit passes (fallow, ponytail, deepen reports).
- `internal/` — internal research/planning notes not meant for the ADR/plan pipeline.
- `orchestrator/` — journal and operating rules for the AFK/orchestrator agent workflow.
- `superpowers/` — plans related to the superpowers skill/workflow.
