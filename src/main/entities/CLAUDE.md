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

That's the whole tax. No new route, no new CLI verb, no `kindFromId` edit — the `apply` path and the verb surface already cover the new kind.

## Layer rules

- The registry lives in `src/main/`; **renderers never import it** (same rule as `plugins/registry.ts`). Renderer-side dispatch is by the string `rendererTag` broadcast over IPC, unrelated to this registry.
- Handlers are the *headless* path. Interactive creation (toolbar placement, drawing capture) keeps its own gesture homes; both ultimately call the same `document-commands` mutators a handler calls.
- One `commitAsOneTransaction` per applied patch keeps the forward/reverse-sync contract intact (one Y.Doc transaction per mutation).

## Scope today (ADR 0019, slice 1)

`POST /canvas/apply` handles **create + update**; `upsertEntities` is a thin client of it. Delete still lives on `/entities/delete` and folds into the registry (with a generic, id-resolved delete) in a later slice. The per-kind create/update routes in `routes/entities.ts` and `routes/pages.ts` still exist for the smoke contract and are removed once every caller routes through `apply`.
