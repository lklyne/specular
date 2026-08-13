# PR 2 — Put Specular's extensions back inside the spec's lines

**Related:** ADR 0004 §2 (the `specular: {}` convention), `docs/file-formats.md`
**Sibling:** [entity-field-drift.md](./entity-field-drift.md) — land that first;
this one touches the same serializer

## Why this matters

`.canvas` files are the product, not an implementation detail. The file format
principles in `CLAUDE.md` say data lives in open formats that other tools and
agents can read and edit. That promise only holds if a Specular canvas is a
*valid JSON Canvas* canvas — one another tool opens without choking, where our
additions are visibly ours and safely ignorable.

ADR 0004 §2 already decided how: **Specular-only fields live under a namespaced
`specular: {}` object; nothing else goes at the top level.** The reasoning is
worth re-reading before starting — it is about other tools reading our files and
our reader falling back to documented defaults on theirs.

Practice has drifted from that decision. Several spec node types carry Specular
fields at their top level, mixed in with spec fields, where another tool cannot
tell ours from the standard's. This PR closes that gap.

## The one distinction that governs everything

ADR 0004 draws a line that is easy to miss and decides most cases:

- **Spec node types** — `text`, `file`, `link`, `group` — are JSON Canvas's.
  Our fields on them must be namespaced under `specular: {}`.
- **Specular node types** — `shape`, `drawing` — are ours outright. The spec
  has no such types and no fallback rendering would be meaningful, so their
  top-level fields are fine. **Leave them alone.**

Getting this backwards is the likeliest way to make things worse. When in
doubt: is this node type in the JSON Canvas spec? If yes, namespace.

Useful grounding — JSON Canvas 1.0 defines exactly four node types, and node
attributes are only `id`, `type`, `x`, `y`, `width`, `height`, `color`, plus a
handful per type (`text`; `file`, `subpath`; `url`; `label`, `background`,
`backgroundStyle`). Anything else on those types is ours.

## Scope

Fields currently at the top level of spec node types, which should move under
`specular: {}`:

| Node | Spec allows | Ours, currently top-level |
|---|---|---|
| `link` | `url` | `presetIndex`, `syncId`, `label`, `source`, `groupId`, `parentGroupId`, `metadata`, `colorScheme` |
| `file` | `file`, `subpath` | `objectFit`, `presetIndex`, `metadata` |
| `group` | `label`, `background`, `backgroundStyle` | `layoutMode`, `layoutGap`, `pageIds`, `entityIds`, `parentGroupId`, `managedLayout`, `sourceTaskId` |
| edges | `fromNode`/`toNode`, sides, ends, `color`, `label` | `strokeWidth`, `lineStyle`, and others |

Two things worth fixing while in here, both cases of a spec field being
shadowed rather than merely joined:

- The group node writes `groupColor` and `groupMetadata` instead of using the
  spec's own `color`. A spec field exists; use it.
- The `link` node writes the page's name as top-level `label`, which is not a
  spec attribute of `link` (only `group` has `label`). It should be namespaced
  like the rest.

Also worth confirming as you go: `deserializeLinkNodeToPage` reads `groupId`
but not `parentGroupId`, which is a latent inconsistency in the same area.

## Approach

**Readers accept both spellings; writers emit only the namespaced one.** Files
migrate silently as they are re-saved, and nothing breaks in the meantime. A
hard cutover with a one-time migration is the wrong trade for a format users
already have on disk — including canvases inside other people's spaces we never
see.

Keep the tolerance narrow and obvious rather than building a general migration
layer: read the new location, fall back to the old, done. Leave a comment
saying why the fallback exists, since a dead-looking branch invites deletion.

## Ripple effects to handle in the same PR

- `GET /canvas` and `specular canvas` return this exact shape, so an agent
  reading `node.presetIndex` will need `node.specular.presetIndex`. Update the
  skill files — both `resources/skills/specular/SKILL.md` and
  `.claude/skills/specular/SKILL.md`, they must stay in sync — and
  `docs/file-formats.md`.
- The golden `.canvas` snapshot will churn heavily. That diff is the review
  surface for this PR; read it as the primary artifact rather than noise.
- Ask the user whether anything outside this repo reads `.canvas` files
  directly. Only they know, and it changes how loud the release note needs to be.

## How to know you are done

- A Specular `.canvas` file has no non-spec keys at the top level of any
  `text` / `file` / `link` / `group` node, or on any edge.
- `shape` and `drawing` nodes are untouched.
- Opening a canvas saved by the previous version loses nothing.
- The format doc and both skill files describe what the file actually contains.

## Judgment calls left open

- Whether `pageIds` / `entityIds` on group nodes should be persisted at all, or
  derived from children's `parentGroupId`. They are already suspected redundant
  (see the sibling plan). If they turn out to be derived, deleting beats moving.
- Whether to record this as an ADR amendment. ADR 0004 already decided the
  convention, so this is enforcement rather than a new decision — but if the
  reader-tolerance window is meant to close eventually, that is a decision worth
  writing down.
