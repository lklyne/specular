# PR 1 — Close the entity field-drift bug class

**Branch:** `worktree/field-drift-net` (two commits landed, three steps to go)
**Related:** ADR 0024 §5, ADR 0019, `src/main/runtime/CLAUDE.md`
**Sibling:** [json-canvas-spec-compliance.md](./json-canvas-spec-compliance.md) — do that one separately

## What this is really about

A canvas item's fields travel through several code paths: forward sync to the
Y.Doc, save and load from `.canvas`, snapshot hydrate, tab switching,
copy/paste, duplicate. ADR 0024 §5 promised that one declared field list would
drive all of persistence, so that "I forgot to add the field to the other list"
stops being possible. That promise was kept for the two Y.Doc directions and
nowhere else.

Every other path still keeps its own hand-written list of which fields to
carry. Those lists fall behind, and when they do the failure is **silent and
destructive**: the value loads as undefined, the next autosave writes the file
back without it, and the data is gone from disk. Three separate user-visible
bugs came from this one shape:

- shape text alignment reverting to centered on relaunch (fixed, #385)
- stickies and file cards losing their group on relaunch (fixed, this branch)
- copy/paste and duplicate stripping shape styling (still open, step 3 below)

**The goal is not to fix a list of fields.** It is to remove the *category* —
to get to a place where adding a persisted field to an entity kind means
editing one declaration, and every path that copies entities picks it up
without being told. Judge a change by whether it makes the next field-drop
impossible, not by whether it fixes today's symptom.

## The idea to hold onto

There is already a registry (`src/main/entities/`) where each kind declares its
persisted fields once and both Y.Doc directions derive from it. That pattern
works. The work is extending its reach — bringing the paths that hand-list
fields under the same declaration — rather than inventing a new mechanism.

Prefer deleting a hand-written list over updating it. If a path can be
expressed as "persist the entity, then restore it," that is almost always the
better shape than a bespoke field copy, because it inherits correctness instead
of restating it.

## What has landed

**Commit 1 — the net** (`tests/integration/entity-field-fixtures.ts`,
`entity-field-roundtrip.test.ts`)

A fully-populated sample of each kind, plus a reusable assertion that a sample
survives a given path with every declared field intact. Samples are pinned at
runtime against each kind's declared field list, so a newly declared field
fails the suite until its sample sets it.

Note for anyone extending it: the samples carry `satisfies
Required<Persisted…Entity>` for editor feedback, but `tests/` sits outside both
typecheck projects, so that is *not* gate-enforced. The runtime assertion is
what actually holds. (Bringing `tests/` under typecheck is worthwhile and would
make this compile-enforced — it surfaces ~85 pre-existing type errors in the
existing suite, so it is its own piece of work.)

**Commit 2 — grouping survives a relaunch**

`text` and `file` nodes gained `specular.parentGroupId` (and `specular.label`
for text). The net grew a save-and-reload path, which is what caught it.

## What remains

Each step below adds its path to the net first — watch it fail, then make it
pass. That ordering matters more than any particular implementation: it is what
proves the net is really covering the path rather than passing vacuously.

### Step A — one intake door

Four places rebuild runtime entities from persisted data; only two derive their
fields from the registry. `space-restore.ts` hand-lists the page arm twice and
the group arm once, `space-tabs.ts` has five copy-pasted per-kind loops, and the
page snapshot shape is restated four times across `space-persistence.ts` and
`space-tabs.ts`.

Collapse them into one. Page is genuinely different — it is
`WebContentsView`-backed and its live view must survive rather than be rebuilt —
so it keeps its own handler behind the same door; ADR 0024 §2 shaped the
interface to allow exactly that. Group is derived-bbox and similar.

This is the step that turns "we fixed the shape fields and the grouping fields"
into "this cannot happen again," so it is the heart of the PR.

**Treat this as the risky one.** It is the layer `src/main/runtime/CLAUDE.md`
flags as able to lose user work silently. It wants integration coverage with a
named mutation verification, and a manual smoke — create a few items, group
them, quit, reopen — before moving on. If it starts sprawling, stop and hand
back rather than pushing through.

### Step B — duplication reuses persistence

Copy, paste, and group-duplicate each restate every kind's create arguments, and
all three have drifted: copying a styled shape today loses its border style,
fill, and text alignment (verified by running, not just reading). The clipboard
payload type is a fourth parallel family of per-kind field lists.

Reframe duplication as persist → re-id → offset → restore, on top of the door
from Step A. The payload should be the persisted record plus a placement delta,
so it *cannot* carry a different field set than persistence does. Expect to
delete a lot more than you add.

Page-anchor re-targeting on paste is placement logic, not field copying — leave
that behavior as it is.

Do this after Step A; its whole premise is reusing that door.

### Step C — fields that silently ignore updates

Some fields persist and load correctly but do nothing when an agent or the
details panel sets them, because each registry handler casts patch fields one by
one: `textStyle`, `label`, `parentGroupId` on text; `label`, `parentGroupId`,
`pageAnchor` on shape; `label` on drawing.

A different flavor of the same disease — a hand-written list, this time on the
mutation side. Independent of A and B, lowest risk, good last step.

## How to know you are done

- Adding a persisted field to a kind requires touching the declaration and the
  sample, and nothing else, to have it flow through every path.
- The net covers: Y.Doc round trip, save/reload, snapshot hydrate, tab switch,
  copy/paste, duplicate, and update.
- Copying a styled shape produces an identical shape.
- Grouping, styling, and labels survive quit-and-reopen for every kind.

## Traps worth knowing

- **`.canvas` writes back what it loaded.** A lossy load is not cosmetic; the
  next autosave destroys the file's data. Any test that only checks in-memory
  state after a load will miss the whole bug class — check the file too.
- **Tab switching and undo already work.** They derive from the registry. If a
  bug reproduces on relaunch but not on tab switch, that asymmetry is the
  fingerprint of a hand-listed path.
- **Data already lost stays lost.** These fixes stop the bleeding; they cannot
  recover fields that previous saves erased.
- **Gates:** `pnpm typecheck` and `pnpm test:unit` per step, `pnpm
  test:integration` for anything touching runtime, IPC, or persistence. Name the
  mutation verification in each new test's docstring per `tests/README.md`.

## Deliberately not here

- Making the JSON Canvas serializer derive from the field list rather than
  hand-written serialize/deserialize pairs. It brushes against ADR 0024 §6 (no
  descriptor framework) and deserves its own ADR conversation.
- `persistGroupEntity` omits `pageIds` / `entityIds` that its own field set
  declares. Extending the net to group will surface it; decide then whether
  those fields are genuinely derived (and should leave the declaration) or
  genuinely persisted (and the projection is wrong).
- Scene, panel, and graph projections hand-list fields too, but they cause
  stale UI rather than data loss — a lower tier of the same problem.
