# ADR 0048: A flow's routes are stored on their edges and drawn only while they meet their ends

**Status:** Proposed. Built this way in `native/`. The user has not reviewed this ADR.
**Date:** 2026-10-28
**Related:** [ADR 0026](./0026-arrange-preserves-footprint.md), the other arrange modes. [ADR 0041](./0041-typed-document-inverse-command-undo.md), why a flow is one command.

## Context

An agent screenshots a set of screens and wants them on the canvas as a state diagram: each screen placed by what leads to it, and the edges between them drawn as right-angle lines that run round the screens. Specular drew every edge as one bezier between two side midpoints, worked out from where its entities are on each frame.

A right-angle route depends on every screen of the flow, not only on the edge's two ends: it turns in the gaps between ranks, on tracks shared out between the edges that cross the same gap. It cannot be worked out per edge per frame, so it has to be computed by a layout and stored. Then the question is what a stored route means once someone moves a screen by hand, since the route no longer meets it.

The layout is adapted from [Pele](https://github.com/obsidianmd/pele)'s layered layout (MIT), which renders Mermaid diagrams for Obsidian. Pele only returns an SVG, so its algorithm is ported to Rust rather than called.

## Decision

- A flow (`specular_interact::flow_plan`) places the entities and gives each edge between them a `route`: its corners in canvas units, from where it leaves its first entity to where it reaches the second, stored on the edge with the sides it meets. Edges from an entity of the flow to one outside it lose their route.
- An edge's route is drawn only while its first point sits on the side it names of its first entity, and its last on the second's (`App::edge_path`). Otherwise the edge is drawn, hit and labelled as the curve it always was. The check is made where the edge is drawn, against where its entities are seen.
- Moving or resizing an entity does not touch its edges' routes. A moved screen's edges become curves, and the route stays in the file until the next flow replaces it. Undoing the move draws the route again.
- Dragging an edge's end to another entity clears its route.

## Consequences

- No move, resize, group or page-anchoring path needs to know about routes, and undo needs nothing new. Every way an entity can move is covered by the one check where the edge is drawn.
- A stale route is kept in the file, invisible. It is small, and `route` is ignored by other JSON Canvas tools.
- After a hand move the flow shows mixed lines, routes and curves, until `specular arrange flow` is run again. A flow group that lays itself out again on every change is left for later.
- A route can pass through an entity that was moved into its way after the layout, since only the ends are checked.
