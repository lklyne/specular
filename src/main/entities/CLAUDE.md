# Entity-kind registry

This folder is the **headless mutation spine** for canvas entities (ADR 0019). One self-describing handler per `CanvasEntityKind` owns that kind's `create` / `update` / `serialize` / `defaultSize` / `fields`. It is the mutation-side mirror of the entity-renderer registry in `src/main/plugins/` — a renderer is one claim file, an entity kind is one handler file.

## How dispatch works

Every canvas mutation compiles to a **patch** and hits one route:

```
verb / upsertEntities  →  { entities: [ {kind,…}, {id,…} ] }   (a patch)
                       →  POST /canvas/apply                     [routes/canvas.ts]
                       →  getEntityKind(item.kind)               [contract.ts]
                       →  handler.create(item) | handler.update(id, item)
                       →  all inside ONE commitAsOneTransaction  → one undo step
```

- **No `id`** on an item → `create`. **`id` present** → `update`.
- A long / structured `text` create is re-routed to the `file` kind as a `.md` note (`builtin/file.ts#claimsAsNote`). This is the auto-route that used to live in `shared/entity-ops.ts`.
- Positions are resolved by the caller (placement / layout directive) before the patch reaches the route. `defaultSize` supplies a footprint when a create omits one.

`update` honors only the keys a kind lists in `fields`; the underlying state mutators ignore unknown/`undefined` keys, which is what structurally kills the "update silently ignores `--url`/`--width`" class of bug.

## How to add an entity kind

1. **Add the literal** to `CanvasEntityKind` in `src/shared/types.ts` (and a `Persisted…Entity` type if the kind persists new fields).
2. **Create the handler** under `builtin/your-kind.ts` as an `EntityKindDefinition<'your-kind'>`. Delegate `serialize` to the matching `serialize…Node` export in `runtime/json-canvas-serializer.ts` and reuse the existing `document-commands` mutators for `create`/`update` — don't write a second mutation path.
3. **Register it** in `index.ts` by adding it to the `builtIns` array.

That's the whole tax. No new route and no new CLI verb — the `apply` path and the verb surface already cover the new kind, and kind is resolved from the doc by id (no prefix sniffing to teach).

## Layer rules

- The registry lives in `src/main/`; **renderers never import it** (same rule as `plugins/registry.ts`). Renderer-side dispatch is by the string `rendererTag` broadcast over IPC, unrelated to this registry.
- Handlers are the *headless* path. Interactive creation (toolbar placement, drawing capture) keeps its own gesture homes; both ultimately call the same `document-commands` mutators a handler calls.
- One `commitAsOneTransaction` per applied patch keeps the forward/reverse-sync contract intact (one Y.Doc transaction per mutation).

## Scope today (ADR 0019, slice 4)

`POST /canvas/apply` handles the whole patch — **create + update + delete + edges** — in one transaction. Delete is generic and id-resolved: `apply` looks up each id's kind from the doc (via `entityKindById`) and calls that handler's `delete`, or the edge store when the id names an edge. No id prefix sniffing. `GET /canvas` is the JSON Canvas read shape (`specular workspace` reads it).

The verb surface (ADR 0019 §1) is `add page|note|file`, `update`, `delete`, `arrange row|column|grid`, `group`/`ungroup`, `auto-layout`, `distribute`, `focus`, `link`/`unlink`, and `apply` (the JSON door). `move`/`resize` fold into `update --at`/`--size` flags. Every verb is a thin shim that builds a patch and calls `apply`: `add`/`update`/`upsert` via `upsertEntities`, and `delete`/`arrange`/`link`/`group`/`apply` via `applyPatch` (`src/main/shared/entity-ops.ts`). The placement pre-pass (`resolveEntityPlacements`) resolves create positions before the single `apply` post. Drawing/shape have no ergonomic verb (interactive-creation kinds) — they are created via `apply` with a patch.

The per-kind create/update/delete routes are **gone** (slice 4): `/pages/create|update|delete`, `/text-entities/create|update|delete`, `/file-entities/*`, `/note-entities/create`, `/drawing-entities/create|update|delete`, `/entities/create|delete`, and `GET /workspace` (the read shape moved to `GET /canvas`). The per-kind GET list routes (`/text-entities`, `/file-entities`, `/drawing-entities` in `routes/entities.ts`) remain for inspection. `kindFromId` is gone (slice 3): `update`/`delete`/`apply` all resolve an entity's kind from the doc by id via `entityKindById`, so the CLI never sniffs id prefixes.
