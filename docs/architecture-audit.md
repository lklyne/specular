# Architecture audit guide

How to run a holistic simplify/deslop/deepen pass over this codebase without
gutting the load-bearing nuance. Drive the expensive passes in a Fable session;
`fallow` runs in CI and supplies the free ground truth.

## Invariant preamble

Paste this into every audit run. It keeps a delete-biased audit from
simplifying away the layered WYSIWYG canvas with live inline web pages.

```
Before proposing any deletion or simplification, read and treat as load-bearing:
- docs/interaction-layer.md §6 "Load-bearing invariants"
- CONTEXT.md (glossary — canvas items, entities, tools, edges, focus session)
- docs/adr/ — accepted decisions; anything an ADR ratifies is intentional, not slop.
  Especially: 0021 focus-session, 0022 pages select-first, 0023 renderer-owned GPU
  pan/zoom, 0014 canvas stack order, 0016 tools-as-capability-registry.
- CLAUDE.md two-layer state model (Y.Doc = truth, runtime = ephemeral) and layer rules.

Preserve: the layered WYSIWYG canvas with live inline web pages, renderer-owned
camera, and the forward/reverse Yjs sync. If a simplification touches these, flag it
as a tradeoff for human review — do not present it as free deletion.
```

## Run sequence — delete before you deepen

Multi-run, not one holistic pass: ~74k LOC across clean seams, so a single
"audit everything" prompt goes wide and shallow. Scope to the seams.

0. **Ground truth (no model).** Pull the latest `fallow` report from CI. Paste
   its unused-code / duplication / circular-dep / complexity-hotspot sections
   into runs 1 and 2 so the model reasons over evidence, not guesses.

1. **`/ponytail-audit`** — whole-repo cut list, one run. Prepend the preamble +
   fallow findings. Strips dead weight first so later passes reason over a
   smaller, truer surface. Reports only; applies nothing.

2. **`/improve-codebase-architecture`** — deepen & consolidate, one run. Reads
   CONTEXT.md + ADRs itself. Likely targets: `src/shared/types.ts` (split by
   domain), the preload + `src/main/ipc` bridge surface (consolidate),
   oversized `App.tsx` files (keep thin).

3. **`deep-audit` per seam** — depth layer; emits tracer-bullet issues. Run
   heaviest-leverage first, same preamble each time:
   ```
   deep-audit src/main/runtime         # two-layer Yjs/runtime state
   deep-audit src/renderer             # above-view App + pointer router
   deep-audit src/main/ipc src/preload # the IPC/bridge surface
   deep-audit src/main/plugins         # entity-renderer registry
   ```

## Why this shape

- **Facts before opinions** — fallow is free and already ran; feeding it in
  stops the model inventing targets.
- **Cut → deepen → depth** — ponytail removes, improve-architecture consolidates
  the remainder, deep-audit turns survivors into actionable issues.
