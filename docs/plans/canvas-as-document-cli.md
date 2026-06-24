# Canvas as a document: verb-primary CLI over one entity-kind registry

**Source:** [ADR 0019 — Canvas as a document](../adr/0019-canvas-as-document-cli.md)

Read the ADR first — it is the spec. This plan is the build order: the ADR's
"Adoption trigger" slices, one per phase, ordered so no phase breaks current
agents. Every verb compiles to one internal patch and hits one apply path; the
spine mirrors the entity-renderer registry (`src/main/plugins/registry.ts`,
`src/main/plugins/CLAUDE.md`).

## Phase 1 — Entity-kind registry + apply route (internal, no surface change)

Build the spine behind the existing CLI. Nothing user-facing changes.

- Add `src/main/entities/contract.ts` with `EntityKindDefinition`,
  `registerEntityKind` (dup-id guard like `registerEntityRenderer`), and
  `getEntityKind`.
- Add `src/main/entities/builtin/` with one handler per kind (`page.ts`,
  `text.ts`, `file.ts`, `group.ts`, `drawing.ts`, `shape.ts`) implementing
  `create` / `update` / `serialize` / `defaultSize` / `fields`. Register all at
  boot via `registerBuiltInEntityKinds()`, mirroring `registerBuiltInPlugins()`.
  Move the per-kind buckets from `src/main/shared/entity-ops.ts` (the
  `pageCreates` / `textCreates` / `fileCreates` / `noteCreates` pass, the
  long-text→file auto-route, device-metadata prep) into the handlers.
- Add `POST /canvas/apply`: walk the patch, look up each item's handler by
  `kind`, run the whole patch in **one Y.Doc transaction**. Reimplement
  `upsertEntities` as a thin client of this route.
- Layer rule: registry lives in `src/main/`; renderers never import it.

Done when: `upsertEntities` routes through the registry, existing smoke tests
pass unchanged, and adding a kind needs only a new `builtin/` handler.

## Phase 2 — `specular apply` + `specular get`; shim the existing verbs

- Add `GET /canvas` (serialize the doc; this is the new read shape that
  `/workspace` read becomes) and wire `specular get` / `specular workspace` to it.
- Add `specular apply < patch.json` — the one declarative door. Patch shape:
  no `id` → create, `id` present → update, `id` in `delete` → remove, plus
  `edges` and `layout`. Documented as the batch fallback, not the primary path.
- Make today's `create` / `update` / `delete` / `link` / `group` thin shims that
  build a patch and call `apply`. Old commands keep working identically.

Done when: every existing CLI command produces the same result via the apply
spine; `specular apply`/`get` work end-to-end with smoke coverage.

## Phase 3 — Delete `kindFromId`

Now that `update` / `delete` / `apply` resolve kind from the doc by id, remove
the prefix sniffing.

- Delete `kindFromId()` from `src/main/cli-commands.ts`.
- The runtime resolves each entity's kind from the doc. This structurally kills
  the `delete --json` crash, the "delete annotation lies" bug, and the
  "unknown prefix buckets to file" mis-routing.

Done when: no call site sniffs id prefixes; the three listed bugs are
unreproducible; smoke covers delete-by-id and the former crash case.

## Phase 4 — Add `add` / `arrange` verbs; remove per-kind routes

- Add `specular add page|note|file` (kind as subcommand, like `git remote add`;
  file kind inferred from extension). `add` builds a patch and calls `apply`.
- Surface `arrange row|column|grid`, `group`/`ungroup`, `auto-layout`,
  `distribute`, `focus`, `link`/`unlink` as verbs over the apply spine.
- `move`/`resize` fold into `update` flags — no separate verbs.
- Deprecate then remove the per-kind routes: `/pages/create`,
  `/text-entities/create`, `/file-entities/create`, `/note-entities/create`,
  `/drawing-entities/create`, and the per-kind `*/update` and `*/delete` routes.
  `/workspace` read → `GET /canvas`.

Done when: the verb surface in ADR §1 exists, the per-kind routes are gone, and
smoke covers `add` for every kind including `drawing`/`shape` (which had no
create path before).

## Phase 5 — Rewrite the `specular` skill + housekeeping

Rides Phase 4 — never document a command before it exists.

- Rewrite `resources/skills/specular/SKILL.md` and
  `.claude/skills/specular/SKILL.md` to the verb-primary surface (same commit).
  Drop the "Known CLI limitations" entries that are now structurally
  unconstructible.
- Add `src/main/entities/CLAUDE.md` documenting the `EntityKindDefinition`
  contract and the "how to add an entity kind" steps, mirroring
  `src/main/plugins/CLAUDE.md`.
- Update the **Canvas data model** entry in `CONTEXT.md` to link ADR 0019; flip
  the ADR status from Proposed to Accepted.

Done when: the skill documents only shipping commands, `entities/CLAUDE.md`
exists, and CONTEXT.md links the ADR.
