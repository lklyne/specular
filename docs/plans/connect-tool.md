# Connect tool — drawing edges as a first-class gesture

> **Status:** planned, not started. One PR covering the tool, routing styles,
> adjustable elbows, and free endpoints.

## Goal

Enter a **Connect** tool and draw edges by dragging, the way FigJam's connector
tool works. Today an edge can only be born from a hover-discovered anchor dot on
an entity; there is no mode in which drawing connections is the primary thing
your pointer does.

Alongside the tool, edges gain **routing styles** (elbow / curved / straight)
with elbow as the default for new edges, and a selected elbow edge gains
**draggable segment handles** so the user can move the crossbar.

## What already exists

The edge system is built. This plan adds a door into it, not the room.

| Piece | Where |
|---|---|
| Data model — `WorkspaceEdge` | `src/shared/types.ts:1529` |
| Drag state machine (create + re-route + snap) | `src/shared/edge-drag-controller.ts` |
| Geometry — bezier only, `autoSides()` | `src/shared/edge-geometry.ts` |
| Rendering — anchors, arrowheads, labels, hit-stroke | `src/renderer/above-view/EdgeLayer.tsx`, `EdgeDragLayer.tsx` |
| Editing UI | `EdgePopup.tsx`, `right-details-panel/components/EdgeEntityPane.tsx` |
| Mutations through the seam, Y.Doc synced | `src/main/workspace-edges.ts`, `runtime/document-commands.ts:656` |
| `.canvas` round-trip, interleaved in `entityOrder` | `runtime/json-canvas-serializer.ts:308` |

## Decisions

### The tool is `connect`, not `add-edge`

`isPlacementTool` and `placementEntityKindForTool` are about creating
**entities**, and edges deliberately are not entities — they don't register in
the entity-kind registry ([ADR 0024](../adr/0024-entity-kind-registry-spans-runtime-and-persistence.md)).
Naming the tool `add-edge` would put it in a family whose machinery it doesn't
use. Its real siblings are `comment` / `draw` / `inspect`: persistent,
non-placement, owning their own gesture.

- Tool kind: `{ kind: 'connect' }`
- Duration: `persistent` — diagramming is bursty, you draw five connectors in a
  row. Escape or `V` exits.
- Keybinding: `X`. `C` is taken by comment.
- UI label: "Connect". Gerund: "connecting".

**Edge** stays the noun in data and docs per CONTEXT.md; "Connect" is the verb
the user performs. The tool creates edges — the two words describe different
things and both are correct.

### Endpoints stay four sides + auto

FigJam offers four side anchors plus whole-object binding, not arbitrary
perimeter points. We already model both: `fromSide` set pins to a side,
`fromSide: undefined` means "bind to the object" and is resolved per-paint by
`autoSides()`.

Arbitrary perimeter attachment would mean normalized `{u,v}` on every endpoint,
perimeter hit-testing instead of four rects, and reworking `autoSides()` — for
behavior FigJam itself doesn't have. Not doing it.

**One behavior change follows from this.** Today `commitEdgeDrag` returns
`{ kind: 'noop' }` when a create-drag ends without an anchor snap
(`edge-drag-controller.ts:161`), so releasing over a target's body creates
nothing. It should instead commit with `side: undefined`. This is the single
change that makes connecting feel like FigJam, and it improves the existing
anchor-drag door for free.

Sides are auto or pinned **per endpoint**, which `WorkspaceEdge` already models
(`fromSide?` / `toSide?`, absent = auto). An auto end keeps rederiving even on
an edge whose crossbar has been dragged; a pinned end never does. `autoSides()`
today resolves both ends from a single `|dx| > |dy|` test and returns a matched
pair — it splits into a per-endpoint resolution so one end can be pinned while
the other tracks.

### A body drag starts with no side

In connect mode the gesture begins on an entity body, so there is no anchor and
no side to start from. `EdgeDragState.create` widens to
`fromSide: EdgeSide | null`, where `null` means auto. While null, the
rubber-band origin resolves per pointer-move to the side of the source facing
the cursor, so the band swings around the entity as you drag past its corners.
On commit the side stays `undefined` — the edge is object-bound and rederives
per paint, the same meaning the field already carries everywhere else.

`edgeDragOrigin` and `buildEdgeDragPath` (`edge-drag-controller.ts:186`, `:197`)
both call `getAnchorPoint` with a concrete side and need the same fallback.

### Empty space starts a free-ended edge

Dragging from the background in connect mode creates an edge whose start is a
point in canvas space. The other end binds to a target as usual, and the free
end can later be dragged onto an entity — which is the existing `edit` branch
of the drag controller with a free end as a legal starting state, not a new
gesture.

This pulls free endpoints in from the deferred list. JSON Canvas requires
`fromNode` / `toNode`, so there is no spec-legal dangling edge:

- `fromEntityId` / `toEntityId` become nullable, joined by `fromPoint` /
  `toPoint` in canvas coords.
- Free-ended edges persist in a `specular.freeEdges` array **outside** the
  spec's `edges[]`, so a strict reader sees a fully valid file and simply
  doesn't see them. Same trade already made for `annotations` and
  `specular.entityOrder`.
- `EdgeLayer.tsx:292` drops any edge whose entities are missing, and the delete
  cascade (`workspace-edges.ts:85`) keys off both ids. Both stop treating
  "entity absent" as "edge invalid".
- Drop-in-empty during a **re-route** becomes "detach to a free end" rather
  than "delete". Escape-to-cancel still deletes.

This deviation from the spec gets an ADR.

### Self-edges are rejected

`findClosestAnchorTarget` already skips the source entity, so the anchor door
cannot produce a self-edge. Body-release would slip past that guard: releasing
on the source commits A→A with both sides undefined, and `autoSides` — handed
`dx = dy = 0` — falls through to bottom→top and draws a connector straight
through the entity.

`commitEdgeDrag` returns `noop` when the release target is the source. A
self-loop needs its own route (out one side, around, back into an adjacent one)
that shares nothing with the two-entity builder; it is a separate feature, not
a special case of this one. Deferred.

### A dragged segment persists as a scalar plus its axis

An elbow route is derived from two anchor points and their sides. Dragging a
segment overrides part of that derivation, and the override has to survive both
entities moving.

Three candidate models:

| Model | Verdict |
|---|---|
| **Split scalar + its axis** — normalized 0–1 marking where the crossbar sits, tagged with the axis it was dragged on | **Chosen.** Two fields, moves proportionally with either entity, survives the other endpoint rederiving. |
| Bare scalar, no axis | Rejected. The drag is remembered on the axis it was made on, not reinterpreted onto whichever axis dominates later — without the tag the same number means two different placements. |
| Per-segment offset array | Rejected. Segment indices change as the route reshapes when entities move, so saved offsets silently reattach to the wrong segment. |
| Explicit waypoints in canvas coords | Rejected for now. The draw.io model — most powerful, but waypoints don't follow the entities and it turns the router into a real pathfinder. |

**The axis is load-bearing.** Drag the crossbar of a side-by-side pair out to
85% across, then move the target below the source. The dragged offset holds on
its original axis while the auto endpoint rederives to the target's top — the
connector bulges out to the remembered x, drops, and comes back in. It does not
reinterpret 85% as "85% down." Verified against FigJam.

**The router follows from that.** A crossbar held at a remembered offset while
the far end rederives elsewhere is a **4-segment** route — out, cross, along,
in — not the 3-segment mid-split. The elbow builder honors a stored offset on
one axis and reaches the other end wherever it resolves. Still no pathfinding.

**Stated limitation.** A 2-segment L is fully determined by its endpoints and
has nothing to drag; a stored split is ignored, not cleared, and applies again
if the entities move back. A 5-segment S-route ships **non-adjustable** in this
PR.

**Pinned endpoints draw through entities.** An end pinned to a far side forces
the connector to double back and cross the entity body to reach it. Shipping
as-is — it is what FigJam does, and clamping to the perimeter would mean the
arrowhead no longer lands on the side the user pinned.

### Elbow is the default for new edges only

New edges get the `connect` tool default, which is elbow. Edges with no
`routing` field render as bezier, so **no existing canvas changes appearance**.

## Data model

Two optional fields on `WorkspaceEdge` (`src/shared/types.ts:1529`) and
`JsonCanvasEdge` (`src/shared/json-canvas-types.ts:152`), persisted as
`specular.*` extensions alongside the existing `strokeWidth` / `lineStyle`:

```ts
export type EdgeRouting = 'bezier' | 'elbow' | 'straight'

routing?: EdgeRouting        // absent → 'bezier' (back-compat)
elbowSplit?: number          // normalized 0–1; only with routing === 'elbow'
elbowSplitAxis?: 'x' | 'y'   // axis the split was dragged on
```

`updateEdge` (`runtime/document-commands.ts:656`) patches a fixed key list that
today excludes `kind` and `metadata`. All three new fields go into that list.

A `connect` scope joins `ToolDefaults` (`src/shared/tool-defaults.ts:17`)
holding `routing`, `color`, `strokeWidth`, `toEnd`. **Both creation doors read
it** — the connect tool and the existing anchor drag — so an edge is identical
regardless of how it was born. Mirror in `src/main/runtime/tool-defaults.ts`
and the scope allowlist at `src/main/ipc/register-toolbar-ipc.ts:65`.

## Interaction

### Connect mode

- `src/shared/canvas-pointer-owner.ts` — connect takes `'tool-gesture'`, the
  same branch placement uses. Without this a connector drag across a page is
  swallowed by the live web content.
- `src/shared/canvas-pointer-actions.ts` — with connect active, a **body** hit
  yields `begin-edge-drag` instead of the entity-drag actions. Anchor hits
  already yield it. A **background** hit yields a free-start edge drag.
  `runEdgeDrag` (`useCanvasPointerRouter.ts:850`) then does the rest unchanged.
- A click that never drags is a no-op and the tool stays active. Marquee
  selection and click-to-select are unavailable while connect owns the pointer;
  `V` and `Escape` are the exits.

### Dragging a segment

A new `routing-edge` interaction mode carrying the live split value. This
mirrors `resizing-gap` precisely — live value ticks in the broadcast, renderer
previews from it, one doc write at commit, one undo step. Same shape in both
interaction unions (`src/shared/interaction-types.ts:5`,
`src/shared/types.ts:92`) plus `snapshotMode` / `TryEnterInput` / `tryEnter` in
`runtime/interaction-controller.ts` and a `begin*` in `interaction-state.ts`.

Cursor: `col-resize` / `row-resize` by segment axis.

### Hit-testing — note the asymmetry

Entity anchors go through the shared hit-test (`HIT_LAYER_ORDER`, the
`'anchors'` layer). **Edge bodies do not** — `EdgeLayer` selects via an
invisible transparent stroke with `pointerEvents: 'stroke'` and a
`data-edge-id` attribute.

Segment handles and endpoint handles follow the DOM pattern the rest of the
edge already uses, rather than entering the shared hit-test. Cheaper, and it
keeps edges consistent with themselves.

## Rendering

- **Elbow and straight path builders** in `src/shared/edge-geometry.ts`, beside
  `buildBezierPath`. Mid-split when there is no stored offset; 4-segment when a
  stored offset sits outside the direct span. **No obstacle avoidance** —
  FigJam doesn't do it either, and a pinned end deliberately draws through the
  entity it points at.
- **Per-endpoint side resolution** replacing `autoSides()`'s matched pair, so a
  pinned end and an auto end coexist on one edge.
- **Rounded corners** on elbow joins, radius clamped to half the shorter
  adjacent segment.
- **Endpoint handles** on a selected edge. A genuine discoverability win:
  today re-routing means finding the *entity's* anchor dot, which is a
  different affordance than the selected-edge state implies.
- **Segment handles** — midpoint grab on each adjustable segment, shown only
  when the edge is selected and only for elbow routing.
- **Routing dropdown** in `EdgePopup.tsx` (selection) and a new
  `ConnectToolPopup` in the `TOOL_POPUPS` table
  (`above-view/canvasItemPopupTable.ts:80`), mirroring `ShapeToolPopup` /
  `ShapeDropdown`.

## Toolbar

- `src/renderer/shared/icons/toolbar/{,dark/}connect.svg` — new glyph pair.
  There is no arrow/connector icon in the app today; the shape catalog
  (`src/shared/shapes.ts:61`) is 10 geometric shapes and the toolbar's 12 icons
  contain nothing connector-shaped.
- `CustomIcons.tsx` — `ConnectToolIcon`
- `toolbarSections.tsx` — button in the create group, beside add-shape.

## Touch list

1. `src/shared/tool.ts` — union arm, `toolDuration`, `toolGerund`, `toolHasPopup`
2. `src/shared/bindings.ts` + `runtime/binding-handlers.ts` — `tool-connect` on `X`
3. `src/shared/types.ts` + `json-canvas-types.ts` — `routing`, `elbowSplit`,
   `elbowSplitAxis`; nullable `fromEntityId` / `toEntityId` + `fromPoint` /
   `toPoint`
4. `src/shared/tool-defaults.ts` + `main/runtime/tool-defaults.ts` + `register-toolbar-ipc.ts` — `connect` scope
5. `src/shared/canvas-pointer-owner.ts`, `canvas-pointer-actions.ts` — connect-mode branches
6. `src/shared/edge-drag-controller.ts` — `fromSide: EdgeSide | null` in the
   create state, cursor-facing rubber-band origin, body release commits with
   `side: undefined`
7. `src/shared/edge-geometry.ts` — elbow + straight builders, 4-segment offset
   case, per-endpoint side resolution, corner rounding
8. `src/shared/interaction-types.ts`, `types.ts`, `runtime/interaction-controller.ts`, `interaction-state.ts` — `routing-edge` mode
9. `runtime/document-commands.ts:656` — patch key list
10. `runtime/json-canvas-serializer.ts` — the routing fields; free edges to and
    from `specular.freeEdges`
11. `src/main/workspace-edges.ts` — free-end edges survive the delete cascade
12. `above-view/EdgeLayer.tsx` — endpoint + segment handles, routing dispatch,
    render edges with a free end instead of dropping them
13. `above-view/EdgePopup.tsx`, new `ConnectToolPopup.tsx`, `canvasItemPopupTable.ts`
14. `renderer/shared/CustomIcons.tsx`, `icons/toolbar/`, `toolbar/toolbarSections.tsx`
15. `docs/adr/` — the `specular.freeEdges` spec deviation

## Tests

Per the test contract in `CLAUDE.md` / `tests/README.md`:

- **Integration** — `routing` + `elbowSplit` + `elbowSplitAxis` persistence and
  undo round-trip through `.canvas`. Merging the routing work into this PR means
  this can't ride along with a later change.
- **Integration** — the `routing-edge` mutator: one Y.Doc transaction per
  commit, undo round-trips cleanly.
- **Unit** — `edge-geometry` elbow path for the 3-segment, 2-segment L, and
  5-segment S cases; corner-radius clamping at short segments.
- **Unit** — a stored `elbowSplit` holds its axis when the far endpoint
  rederives to a perpendicular side, producing the 4-segment route; a degenerate
  2-segment route ignores the split without clearing it.
- **Unit** — per-endpoint side resolution: one pinned end, one auto end.
- **Unit** — `edge-drag-controller` body-release commits with `side: undefined`
  rather than `noop`; a null-side create drag resolves its origin from the
  cursor; a release on the source entity commits `noop`.
- **Integration** — a free-ended edge round-trips through `specular.freeEdges`
  and survives deletion of the entity at its bound end; re-binding the free end
  moves it back into the spec's `edges[]`.

## Docs

- CONTEXT.md **Edge** entry — the connect tool, the auto-attach rule, the
  per-endpoint auto/pinned distinction, and `routing` / `elbowSplit`.
- `docs/file-formats.md` §Edges — the three new `specular.*` extension fields.
- **ADR** for `specular.freeEdges` — edges living outside the spec's `edges[]`
  is a deviation a future reader will otherwise have to reverse-engineer. The
  routing fields need no ADR; they are additive extensions of a kind the format
  already documents.

## Deferred

**Self-edges.** An edge from an entity to itself needs its own route — out one
side, around, back into an adjacent one — sharing nothing with the two-entity
builder.

**Edge-to-edge attachment.** An endpoint bound to another edge rather than an
entity, at some position along it. No JSON Canvas representation, and it needs
cascade rules for when the host edge is deleted or re-routed.

**Drop-in-empty offers to create a shape or sticky**, FigJam-style. Purely
additive.

**S-route adjustability** — the second crossbar, per the stated limitation.

**Obstacle-avoiding elbow routing.** A different project. Not planned.
