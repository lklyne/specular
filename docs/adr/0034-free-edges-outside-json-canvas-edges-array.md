# ADR 0034 — Free-ended edges live in `specular.freeEdges`, outside the spec's `edges[]`

**Status:** Accepted
**Date:** 2026-08-14
**Related:** [connect-tool.md](../plans/connect-tool.md), [connect-tool-build.md](../plans/connect-tool-build.md), [ADR 0024 — entity-kind registry](./0024-entity-kind-registry-spans-runtime-and-persistence.md) (edges are connective tissue, not entities).

## Context

The connect tool (step 2, free endpoints) lets an edge's endpoint be a bare
canvas-space point instead of an entity — dragging from empty space starts an
edge with nothing to bind to yet, and deleting the entity at one end of an
existing edge should detach that end rather than delete the whole edge.

JSON Canvas v1.0 requires `fromNode` and `toNode` on every edge:

```json
{ "id": "...", "fromNode": "...", "toNode": "..." }
```

There is no spec-legal way to write a dangling edge into `edges[]`. Putting a
sentinel id there (`""`, `null` as a string, a synthetic "void" node) would
make every `edges[]` entry look spec-valid while actually being malformed —
worse than omitting it, because a strict reader has no way to know the id is
fake without out-of-band knowledge.

## Decision

A `WorkspaceEdge` with a null `fromEntityId` or `toEntityId` is not written
into `edges[]` at all. It is serialized into a separate
`specular.freeEdges` array instead, with `fromNode`/`toNode` optional and a
matching `fromPoint`/`toPoint` (canvas-space `{x, y}`) carrying the free
end's position. On load, `specular.freeEdges` is deserialized and merged back
into the same `workspaceEdges` runtime array `edges[]`-sourced edges land in —
there is exactly one edge collection at runtime; the split only exists in the
serialized form.

Re-binding a free end to an entity is a field write (`fromEntityId` set,
`fromPoint` cleared), which moves the edge from `specular.freeEdges` back into
`edges[]` on the next save — not a move between two different runtime
collections, because there was only ever one.

This is the same trade already made twice in this file format:

- **`annotations`** — comment/drawing annotations live entirely outside
  `nodes[]`/`edges[]` because JSON Canvas has no node type for them.
- **`specular.entityOrder`** (documented in CONTEXT.md's **Edges** entry) —
  the interleaved z-order spanning nodes and edges has no home in two
  separate spec arrays, so it's reconstructed from a Specular extension on
  load; a foreign reader loses the interleaving but keeps a fully valid file.

In each case: a strict JSON Canvas reader sees `nodes[]` and `edges[]` that
are completely spec-valid and simply doesn't see the Specular-only data.
Specular reconstructs full fidelity from the extension fields it wrote.

## Consequences

- A `.canvas` file opened by a generic JSON Canvas tool never shows a
  half-drawn or dangling connector — free-ended edges are invisible to it,
  not corrupted-looking.
- `serializeToJsonCanvas` / `deserializeFromJsonCanvas` gain one extra
  partition-and-merge step (split on write by whether both ends are bound;
  concatenate on read), mirroring the existing `entityOrder` reconstruction.
- Every edge mutator that isn't serialization-aware (`workspace-edges.ts`,
  `EdgeLayer.tsx`, `edge-drag-controller.ts`, the delete cascade) treats free
  and bound edges as the same kind of row — a free edge is just one whose
  `fromEntityId` happens to be null — so this ADR's cost is paid once, at the
  file boundary, not at every call site.
- A future strict-mode "flatten to pure JSON Canvas" export would drop
  `specular.freeEdges` (and `annotations`, and stack interleaving) the same
  way it already has to reckon with the other two extensions — no new kind of
  loss, just one more field on the list.
