# ADR 0019 — Canvas as a document: verb-primary CLI over one entity-kind registry

**Status:** Accepted
**Date:** 2026-06-22
**Related:** [ADR 0003 — Page as canonical name for live web items](./0003-page-as-canonical-name-for-live-web-items.md), [ADR 0016 — Tools as a capability registry](./0016-tools-as-capability-registry.md). Architectural precedent: the entity-renderer registry (`src/main/plugins/registry.ts`, `src/main/plugins/CLAUDE.md`). Surface precedent: the `agent-browser` skill (action verbs) and the Figma MCP (a few broad tools, node-type variety inside the payload).
**Origin:** Surfaced auditing the `specular` CLI surface (`src/main/cli-commands.ts`, `src/main/shared/entity-ops.ts`, `src/main/routes/`) for simplicity against the agent-browser and Figma reference CLIs. The data model is already unified (`CanvasEntityKind` is one union); only the *surface* and the *mutation plumbing* are fragmented per kind.

## Context

The domain has exactly one user-facing thing on the canvas — a **canvas item**, the **entity** in runtime terms (CONTEXT.md: "one entity ⇔ one canvas item"). It is a single discriminated union keyed on `kind`:

```ts
// src/shared/types.ts
type CanvasEntityKind = 'page' | 'text' | 'file' | 'group' | 'edge' | 'drawing' | 'shape'
```

The model is unified. The **CLI surface** and the **HTTP/runtime mutation path** are not — they fragment per kind, and every limitation in the `specular` skill's "Known CLI limitations" list is a symptom of that fragmentation rather than an independent bug:

| Fragmentation | Where | Symptom |
|---|---|---|
| Per-kind create commands | `create page` / `create note` subverbs (`cli-commands.ts:101-144`) | `drawing` and `shape` have no create path at all |
| Per-kind create routes | `/pages/create`, `/text-entities/create`, `/file-entities/create`, `/note-entities/create`, `/drawing-entities/create` | five payload shapes for one union; `upsertEntities` re-implements the routing on top |
| Kind recovered by id prefix | `kindFromId()` (`cli-commands.ts:168-173`) | `update`/`delete` guess kind via `id.startsWith('page_')`; unknown prefixes silently bucket to `file`; `delete --json` with a string array crashes on `.startsWith` of `undefined` |
| Kind-blind generic verbs that lie | generic `delete` accepts annotation ids | returns `deleted: true` but never calls the annotation route — the annotation stays |
| Per-kind update field support | `update` flag handling | `--url`, `--width`, `--label` return `updated: [id]` while applying nothing |
| JSON as the power surface | `upsert --json` is the only generic door | the declarative patch became the *primary* batch interface, when it was meant as a fallback |

This is the same class of problem ADR 0016 identified for tools — a concept that already has a clean *identity* (the union) but whose *behavior* is smeared across N call sites with no single owner — and the codebase has already solved the equivalent for **entity renderers**: a renderer is one claim file under `plugins/builtin/` + one `RendererSwitch` case, and every render surface is a projection of `pickRenderer`. The *mutation* side of an entity never received that treatment. Adding a kind today means editing the CLI verb set, adding a create route, adding an update route, teaching `kindFromId` the new prefix, and adding a bucket to `upsertEntities` — five edits, none enforced by the type system.

Two reference CLIs show the target shape:

- **agent-browser** exposes *action verbs* (`click`, `fill`, `snapshot`, `screenshot`). Nouns never multiply; you act on the live page with verbs that take one or two positional args. No JSON.
- **Figma MCP** exposes a handful of broad tools (`use_figma`, `generate_figma_design`, `get_design_context`); the node-type variety lives *inside* the payload, not in the tool list.

Specular's surface should be the union of those two: **ergonomic action verbs as the primary surface, with one declarative JSON door kept as a batch fallback** — not the reverse.

## Decision

Treat the canvas as a document with one organizing idea: **read it, patch it, act on it.** The CLI surface is **verbs**; the patch is the **transport** every verb compiles to, not a surface users assemble by hand. Underneath, one **entity-kind registry** owns create/update/delete/serialize for every kind, mirroring the entity-renderer registry.

Crucially, verb-ergonomics and single-spine plumbing are **not a tradeoff**: every verb builds the same internal patch and hits the same apply path, so the surface reads like agent-browser while the mutation path stays singular.

### 1. The surface — verbs primary

**Read** — what's there
```
specular canvas             # canvas state as JSON Canvas (entities, edges, groups)
specular snapshot [-f id]    # live DOM accessibility tree of a page (refs for interaction)
specular screenshot [-f id]  # pixels
```

> **Note (ADR 0033, v0.6.0):** `specular workspace` was renamed to `specular canvas` as part of the vocabulary cleanup that retired "workspace" (space = folder, canvas = document). The old verb is preserved as a hidden alias so existing agent skills don't break mid-transition.

**Add** — kind is the subcommand (like `git remote add`)
```
specular add page <url> [--at x,y] [--preset N]
specular add note <text> [--at x,y] [--color 3]
specular add file <path>           # md / wireframe / html / image — kind inferred from extension
```

**Edit**
```
specular update <id> [--at x,y] [--size w,h] [--color] [--text] [--url] [--preset]
specular delete <id…>
```
`move`/`resize` fold into `update` flags — no separate verbs. The registry makes each flag actually take effect per kind, which is what kills the "update silently ignores `--url`/`--width`" class of bug.

**Arrange** — today's `layout` directive, surfaced *as a verb*
```
specular arrange row|column|grid <id…> [--gap m]   # rearrange existing entities
specular group <id…> / ungroup <id>
specular auto-layout <id…> / distribute <id…>
specular focus <id…>
```

**Connect**
```
specular link <a> <b> / unlink <id…>
```

**Comment** — its own lifecycle, stays verbs (see Non-goals)
```
specular comment <text> [--on id] / resolve / ack / dismiss / reply <id>
```

**Drive a page** — agent-browser passthrough, unchanged
```
specular click / fill / type / scroll / back / reload
```

**Fallback — the JSON door, documented last**
```
specular apply < patch.json    # batch create/update/delete + layout in one shot
```
A patch is the one shape every verb compiles to:
```jsonc
{
  "entities": [ {"kind":"page","url":"…"}, {"id":"text_7","text":"updated"} ],
  "edges":    [ {"from":"a","to":"b"} ],
  "delete":   ["text_3", "edge_9"],
  "layout":   {"kind":"grid","cols":3,"gap":"m","near":"page_1"}
}
```
No `id` → create. `id` present → update. `id` in `delete` → remove. `apply` is the *only* place JSON appears on the surface. It exists for the genuinely batch case ("create 6 pages in a 3×2 grid") where verbs would be six calls. Power users and programmatic callers reach for it; nobody needs it for the common path.

### 2. The spine — one entity-kind registry

Mirror `plugins/registry.ts`. Each kind registers one self-describing handler:

```ts
// src/main/entities/contract.ts
export interface EntityKindDefinition<K extends CanvasEntityKind = CanvasEntityKind> {
  kind: K
  create(input: EntityCreateInput, ctx: MutationContext): EntityId
  update(id: EntityId, patch: EntityPatch, ctx: MutationContext): void
  serialize(entity: PersistedCanvasEntity): JsonCanvasNode
  defaultSize(input: EntityCreateInput): { width: number; height: number }
  fields: readonly string[]   // the update flags this kind actually honors
}

registerEntityKind(def): void        // dup-id guard, like registerEntityRenderer
getEntityKind(kind): EntityKindDefinition
```

Built-in kinds live in `src/main/entities/builtin/` (`page.ts`, `text.ts`, `file.ts`, `group.ts`, `drawing.ts`, `shape.ts`) and register at boot via `registerBuiltInEntityKinds()`, exactly like `registerBuiltInPlugins()`.

### 3. The transport — one apply route

```
GET  /canvas            # serialize the doc (replaces /workspace read shape)
POST /canvas/apply      # the single create/update/delete/arrange path
```

`POST /canvas/apply` walks the patch, looks up each item's handler by `kind`, and runs the whole patch inside **one Y.Doc transaction** — which is what the test contract already wants ("one Y.Doc transaction per mutation"). The five `*/create` routes, the per-kind update routes, and the per-kind delete logic collapse into this one route. `upsertEntities` is reimplemented as a thin client of it; the per-kind buckets in `entity-ops.ts` move into the registry handlers.

### 4. Kind is data, never a command and never an id prefix

`kindFromId()` is deleted. `update`/`delete`/`apply` operate on the doc by id; the runtime resolves the entity and its kind from the doc. The CLI never sniffs prefixes. This structurally removes the `delete --json` crash, the "delete annotation lies" bug, and the "unknown prefix buckets to file" mis-routing — there is one delete path and it always knows the kind.

## Alternatives considered

**A. Do nothing.** The per-kind sprawl is tolerable today only because the kind set is small and rarely grows. Every new kind re-pays the five-edit tax, and the limitations list keeps accreting one symptom per fragmentation. The `.canvas`/agent direction in CLAUDE.md ("editable by agents and other tools") makes a clean, generic mutation surface a growing need, not a shrinking one. Rejected as the end-state.

**B. Generic create only (collapse the CLI, keep the routes).** Add `specular create <kind>` over the existing `upsertEntities` without unifying the HTTP/runtime layer. Cheap, but cosmetic — `upsert --json` already *is* that generic door, so it buys little, and it leaves `kindFromId`, the five create routes, and the per-kind update gaps in place. Right-sized as an incremental first step (see Adoption trigger), wrong as the architecture.

**C. JSON patch as the primary surface.** The first draft of this design led with `apply`/`get` and demoted verbs to sugar. Rejected: the patch is the right *transport* but the wrong *surface*. Leading with JSON makes the common path ("add a page") ceremony-heavy and reads nothing like agent-browser; the declarative door was always meant as a batch fallback. Verbs primary, patch as transport, one `apply` escape hatch.

**D. Verb-primary surface over one entity-kind registry (this decision).** Amortizes across all per-kind axes at once — create, update, delete, serialize, default size, honored fields — the same move already made for entity renderers, so the pattern, layer discipline, and "how to add one" docs already exist to copy. The surface matches the two reference CLIs; the spine matches the codebase's own precedent. Accepted.

## Consequences

**Collapses / removes:**
- CLI: `create page` / `create note` subverbs → `add <kind>`; `kindFromId()` → deleted; `update` flag handling → registry `fields`; `upsert --json` → demoted to `apply`, documented last.
- Routes: `/pages/create`, `/text-entities/create`, `/file-entities/create`, `/note-entities/create`, `/drawing-entities/create`, the per-kind `*/update` and `*/delete` routes → `POST /canvas/apply`. The `/workspace` read shape → `GET /canvas`.
- `entity-ops.ts`: the single-pass per-kind bucketer (`pageCreates`/`textCreates`/`fileCreates`/`noteCreates`) and the long-text→file auto-route → registry handlers (`file`'s create claims long/structured text; `page`'s create owns device-metadata prep).
- Most of the skill's "Known CLI limitations" list (the delete-lies bug, the `delete --json` crash, the silent-no-op update flags, the per-kind create asymmetry) → structurally unconstructible.

**Invariants preserved:**
- The data model is unchanged: `CanvasEntityKind` / `PersistedCanvasEntity` stay the flat, JSON-diffable unions persisted to `.canvas`. The registry wraps behavior *beside* the persisted value, exactly as the entity-renderer registry sits beside the persisted file entity.
- Layer rule: the registry lives in `src/main/`; renderers never import it. The CLI talks to main only via the HTTP API (`callApp`), as today.
- Main remains the single owner of workspace state; one Y.Doc transaction per applied patch (forward/reverse sync contract intact).
- JSON Canvas on disk is unchanged; `serialize` per kind is where the runtime entity meets the disk `node` (CONTEXT.md's "node" layer).

**Non-goals:**
- **Annotations stay their own resource.** Comment threads have a genuine lifecycle (replies, `ack`/`resolve`/`dismiss`) that JSON Canvas does not model and that does not fit the geometric entity patch. They keep their verbs and routes; they are not folded into `apply`.
- **Browser/page-drive verbs are unchanged** — they act on the live DOM, not the doc, and already match the agent-browser model. `apply` does not touch them.
- **Camera/`focus`** stays a verb against runtime (viewport is ephemeral, not doc state).
- Gesture/runtime handlers for interactive creation (toolbar placement, drawing capture) keep their homes; the registry is the *headless* mutation path the CLI/API and interactive code both call.

**Enables:** adding a kind becomes one `registerEntityKind` call — zero new routes, zero new CLI verbs, zero skill edits (the verb surface and `apply` already cover it). Agent- and plugin-authored entity kinds become consistent with the `.canvas`/agent direction in CLAUDE.md, the same way entity renderers and `.canvas` files already are.

## Adoption trigger

This is the end-state, not a mandate to rewrite a working CLI today. The pragmatic incremental path is registry-shaped and lands in slices that never break current agents:

1. Add the entity-kind registry + `POST /canvas/apply` behind the scenes; reimplement `upsertEntities` on top. Pure internal unification, covered by the existing smoke contract — no surface change.
2. Add `specular apply` + `specular get`, and make today's `create`/`update`/`delete`/`link`/`group` thin shims over `apply`. Old commands keep working.
3. Delete `kindFromId` once everything routes through the registry.
4. Add the `add`/`arrange` verbs; deprecate then remove the per-kind routes.
5. Rewrite the `specular` skill to the verb-primary surface — **this rides step 4**, never before, so the skill never documents a command that does not yet exist.

When adopted, add `src/main/entities/CLAUDE.md` documenting the contract and the "how to add an entity kind" steps, mirroring `src/main/plugins/CLAUDE.md`, and update the **Canvas data model** entry in `CONTEXT.md` to link this ADR.
