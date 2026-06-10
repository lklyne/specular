# CLI workflows to grade

The fixed set of tasks the **discovery phase** runs against the *real running app*
to surface friction (see `probes.md` → "Two phases"). A doer agent performs one
workflow as a first-time user would; an independent judge grades the trace against
the charter's friction signals.

Rules for grading these:

- **Run them, don't invent.** A made-up task grades a flattering thing. These are
  fixed so the measurement is honest across rounds.
- **Behave like a real agent.** Discover flags from `specular --help` and the
  skill (`.claude/skills/specular/SKILL.md`), never from reading `src/`. If you had
  to guess a flag or dive into source, that *is* the friction — record it.
- **Count every call.** One logical intent should cost ~one command. The "ideal
  calls" below is the target an agent-friendly CLI should hit, not a hard limit.
- **Two of these (W5, W6) need the full app** — real rendering, screenshot capture,
  agent-browser. They are the reason discovery runs against the live app and not
  the headless smoke instance.

Each workflow records (in `<trace>.meta`): its id, whether the **acceptance**
check was observably met, and the **call count** the doer actually used.

---

## W1 — Build a small canvas

**Goal.** Put two pages at breakpoints and one note on the canvas, then confirm the
edits are real by reading the workspace back.

**Path.** Create a page at `http://localhost:4321` (desktop), a second at a phone
breakpoint, add a sticky note, then `workspace`.

**Ideal calls.** ≤ 4.

**Acceptance.** `workspace` shows both pages and the note with finite geometry; the
note text round-trips exactly.

**Friction to watch.** Does `create page` accept the URL without complaint? Is each
result parseable JSON with no `--format`? Did adding the note take one call?

---

## W2 — Annotate then resolve

**Goal.** Leave an annotation on a live page, then resolve it.

**Path.** Create a page, annotate it with a short comment, list annotations, resolve
that annotation, confirm it reads as resolved.

**Ideal calls.** ≤ 4.

**Acceptance.** The annotation appears after creation and reads resolved after the
resolve call — both observable on read-back, not inferred from exit codes.

**Friction to watch.** Could you find the annotation's id from the create output, or
did you need an extra list call? Is the resolve target named the same way the create
output named it (id stability)?

---

## W3 — Group, auto-layout, focus

**Goal.** Take several scattered pages, group them, auto-lay-out the group, and
focus it.

**Path.** Create 3 pages, group them, run auto-layout on the group, focus the group.

**Ideal calls.** ≤ 6.

**Acceptance.** `workspace` shows one group containing all three; their positions
are non-overlapping after auto-layout; the camera/selection reflects the focus.

**Friction to watch.** Did grouping return a group id you could feed straight into
auto-layout and focus, or did you re-query between steps? Any step need a flag that
wasn't in the skill?

---

## W4 — Grid at scale

**Goal.** Lay a dozen-plus items out as an even grid in one shot and confirm none of
them overlap.

**Path.** `upsert --json` with a `{ kind: grid }` directive and 12+ text items;
read the workspace back.

**Ideal calls.** ≤ 2 (one upsert, one read-back).

**Acceptance.** All items present with finite geometry; no two AABBs overlap; the
result is a real grid (distinct rows and columns), not a pile or a single column.

**Friction to watch.** Is the directive grammar discoverable from `--help`/skill, or
did you guess `cols`/`gap`/`originX`? Does `upsert` report ids that match what
`workspace` reports (round-trip)? — this is the race the first heal fixed; keep
watching it. Overlap only shows at scale, so always use 12+.

---

## W5 — Pull a live page and capture it  *(full app)*

**Goal.** Bring a real URL onto the canvas and capture a screenshot of the rendered
page.

**Path.** Create a page for a real site, wait for it to load, take a screenshot,
confirm the capture is a non-empty image.

**Ideal calls.** ≤ 3.

**Acceptance.** The screenshot call returns a non-empty PNG (base64 length well
above an empty/transparent frame). A blank or error capture is a failure worth a
backlog item — it is the headless-render gap, and against the live app it should
*work*.

**Friction to watch.** Is there a single command to capture a page by id? Does it
tell you *where* the image landed or hand you the bytes? Did you have to poll for
load yourself, or does capture wait?

---

## W6 — Inspect and drive a page via agent-browser  *(full app)*

**Goal.** Snapshot a live page's DOM, find an element, and interact with it.

**Path.** Create a page for a real site, `snapshot` it, locate a link or button by
semantic name from the snapshot, click or query it.

**Ideal calls.** ≤ 4.

**Acceptance.** The snapshot returns structured, parseable elements with refs; the
located element resolves and the interaction reports success — all observable.

**Friction to watch.** Is `snapshot` output parseable without a flag? Do refs from
the snapshot feed straight into the next command, or did you have to translate them?
Are agent-browser errors (binary missing, page not loaded) actionable on stderr?

---

## Adding a workflow

Keep them small, fixed, and observable. A good workflow is one logical agent intent
with a read-back that proves it happened. Prefer tasks that an agent will actually
run often. When a workflow's friction gets fixed and locked into a probe, the
workflow stays — it is the realistic exercise; the probe is the regression guard.
