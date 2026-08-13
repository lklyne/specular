# Close the entity field-drift bug class

**One PR.** Extends ADR 0024 §5 from the two Y.Doc legs to every leg a canvas
item's fields travel, and pays off the drift that accumulated while only one
leg was covered.

## The bug class

A canvas item lives in four representations:

```
runtime arrays  ⇄  Y.Doc  ⇄  .canvas on disk
      ↕
  clipboard / duplicate payloads
```

ADR 0024 §5 promised one field list drives persistence, making "forgot to
extend the restore allow-list" structurally impossible. That was delivered for
`runtime ⇄ Y.Doc` (`def.persist` / `def.restore` / `fields`). The disk leg and
the duplication leg still hand-list fields, twice each per kind, held in sync
by nothing.

Adding one persisted field to `shape` today needs ~16 hand edits across 13
files. Exactly one is covered by a test — `entity-kind-persisted-fields.test.ts`
compares `persist()`'s output keys against the declared list. It checks the
**declaration**, never the **copy paths**. That is why every bug below shipped
silently.

### Confirmed data loss in main today

| Symptom | Site |
|---|---|
| Shape text alignment reset to centered on relaunch | `space-restore.ts` shape arm — **fixed in #385**; step 3 subsumes that fix |
| Grouped sticky loses its group on relaunch | `JsonCanvasTextNode` has no `parentGroupId`; `serializeTextToTextNode` never writes it or `label` |
| File entity loses its group on relaunch | `serializeFileToFileNode` omits `parentGroupId` |
| Copy/paste strips shape border + alignment | `shapePayload` / paste arm omit `fillStyle`, `borderStyle`, `borderColor`, `textAlign`, `textVerticalAlign` — verified by running: the pasted shape carries none of the four styling fields the original had |
| Duplicating a group strips the same fields | `workspace-groups.ts` — a third independent copy |
| Page loses `colorScheme` on snapshot hydrate | `hydrateDocFromSnapshot` omits it, though `PAGE_DOC_FIELD_SET` declares it |
| `textStyle`, `label`, `parentGroupId` silently no-op on update | `builtin/text.ts` cast list; same for `shape`, `drawing` |

## Explicitly out of scope

**Declarative JSON Canvas node mapping.** Serialize/deserialize stay
hand-written pairs; step 2 only makes them *complete*. Collapsing them into one
declaration brushes against ADR 0024 §6 (no descriptor framework) and deserves
its own ADR conversation. Accepted debt: the duplication remains, but the net
from step 1 makes it loud instead of silent.

Existing `.canvas` files that already lost a sticky's group membership cannot
be migrated — the data is not in them. This stops the bleeding; it does not
recover.

## Order of work

Each step is one commit. The net is built once, then extended one leg at a
time, each leg landing green alongside the change that fixes it. No step lands
a red test, and no consolidation lands without coverage.

### Step 1 — The drift harness

`tests/unit/entity-field-roundtrip.ts` (helper, not a spec file):

- `sampleEntityFor(kind)` — a fully-populated fixture per kind, every declared
  field set to a distinctive value. Type each `satisfies Required<PersistedShapeEntity>`
  (etc.) so a newly declared field is a **compile error in the fixture**. A
  `fields`-derived runtime builder cannot do this — it has no way to invent a
  valid `strokes` array or `file` path.
- `expectFieldsSurvive(kind, roundTrip)` — asserts the field *set* survives,
  deriving the expectation from `fields` rather than enumerating. Never needs
  editing again.

Wire it to the one already-green leg: `runtime → Y.Doc → runtime` via
`def.persist` / `def.restore`. Zero red.

**Acceptance:** all six kinds pass the Y.Doc leg. Removing a field from any
`*_PERSISTED_FIELD_SET` fails the fixture at compile time.

### Step 2 — Disk leg + targeted format fixes

Add the `runtime → .canvas → runtime` assertion. Red for `text`
(`parentGroupId`, `label`) and `file` (`parentGroupId`).

Fix:
- Add the missing keys to `JsonCanvasTextNode` / `JsonCanvasFileNode` and their
  serializer pairs in `json-canvas-serializer.ts`.
- Add `colorScheme` to `hydrateDocFromSnapshot` (`space-doc.ts:121`).
- Confirm `deserializeLinkNodeToPage` sets `parentGroupId`, not only `groupId`.

Do this before the consolidations: it is the only user-visible data loss still
shipping, and steps 3–4 both need a green disk leg to test against.

**Acceptance:** disk leg green for all six kinds. Update the golden
`rich-workspace.canvas` snapshot — the fixture must include a grouped text and
a grouped file so the new keys actually appear.

### Step 3 — One intake door (candidate 1)

Four modules rehydrate entities into runtime arrays; only two go through
`def.restore`. `space-restore.ts` hand-lists the page arm twice and the group
arm once; `space-tabs.ts` hardcodes five copy-pasted per-kind loops; the page
snapshot shape is restated four times across `space-persistence.ts` and
`space-tabs.ts`.

Consolidate into one deep module owning "snapshot → runtime", dispatching
`def.restore` for **every** registered kind including page and group. Page's
`WebContentsView` creation is the genuine divergence and stays behind its own
handler — ADR 0024 §2 shaped the interface to allow exactly this. The spread
currently in `space-restore.ts` collapses into that door.

**Acceptance:** snapshot-hydrate and tab-switch legs added to the net, green.
The `restores every persisted field of a styled shape from disk` test in
`persistence.test.ts` still passes unchanged.

**Risk:** highest of the set — `space-*.ts` is the layer `src/main/runtime/CLAUDE.md`
flags as able to lose user work silently. Per the test contract this needs
integration coverage with a named mutation verification, and a forward-sync
"one mutation → one Y.Doc transaction" assertion if the sync path shifts.

### Step 4 — Duplication as persist-and-restore (candidate 3)

Copy, paste, and group-duplicate each restate every kind's create arguments,
and all three have drifted. `ClipboardEntityPayload` is a flat union
hand-listing every kind's copied fields — a fourth parallel type family.

Rewrite duplication as `def.persist` → re-id → offset → `def.restore`, reusing
the intake door from step 3. The clipboard payload becomes the persisted record
plus a placement delta, so it cannot carry a different field set than
persistence does.

Deletes: `ClipboardEntityPayload`'s per-kind arms, both paste tables, and the
per-kind arms of `workspace-groups.ts#duplicateGroup`.

**Acceptance:** copy→paste and duplicate legs added to the net, green.
Page-anchor re-targeting on paste (`payloadAnchor`) keeps its current behavior —
it is placement, not field copying.

**Ordering:** must follow step 3. Its premise is reusing the intake door; built
first, it wires duplication into four doors and gets rewritten again.

### Step 5 — Update patch map (candidate 5)

Each `builtin/*.ts` handler casts patch fields one by one, so a field can be
persisted and serialized correctly yet silently no-op when an agent or the
details panel sets it.

Add a mutation-leg assertion: every declared field is settable through
`def.update`. Red for `text` (`textStyle`, `label`, `parentGroupId`), `shape`
(`label`, `parentGroupId`, `pageAnchor`), `drawing` (`label`). Fix the casts and
the corresponding `EntityUpdatePatchMap` entries in `shared/types.ts`.

Last because it is independent of steps 2–4 and lowest blast radius.

## Gates

Per `CLAUDE.md`: `pnpm typecheck` + `pnpm test:unit` after every step;
`pnpm test:integration` after steps 2, 3, and 4 (runtime, IPC, persistence all
move). Each new test names its mutation verification in the file docstring, per
`tests/README.md`.

## Follow-ups this PR deliberately leaves open

- Declarative JSON Canvas node mapping (the skipped candidate) — wants an ADR
  first, and a decision on whether ADR 0024 §6's "no descriptor framework" line
  covers a format mapping whose two directions are provably inverses.
- `persistGroupEntity` omits `pageIds` / `entityIds` that
  `WORKSPACE_GROUP_PERSISTED_FIELD_SET` declares, and no test compares them.
  Step 1's harness will surface this; decide then whether those fields are
  genuinely derived (and should leave the field set) or genuinely persisted.
- The scene / panel / graph projections (`canvas-layout-data.ts`,
  `inspect-session.ts`, `workspace-entities.ts`) hand-list fields too. They
  cause stale UI, not data loss, so they are a lower tier.
