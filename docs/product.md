# Product Philosophy

## Who it's for

Designers and frontend developers who think spatially. People who browse,
collect, arrange, and iterate on web content as part of their design practice.

## What it is

Part browser, part spatial canvas. Not a traditional tab-based browser, not a
traditional design tool. The intersection where you pull live web content onto
a freeform surface and think with it. The browser part lives inside page nodes
on the canvas rather than in a separate tabbed browser mode.

## Core beliefs

### Open, local-first formats

.canvas and JSON files that live on the user's computer. No proprietary blobs,
no server dependency. Files are human-readable, version-controllable, and
editable by agents and other tools. We follow the JSON Canvas v1.0 spec
(created by Obsidian) and extend it transparently.

The file system is the data model. A space is a folder. A canvas is a file.
If you open your data in a text editor, it makes sense.

### Small modular building blocks

Nodes, edges, and canvases are composable primitives, not monolithic features.
A link node is a link node whether it shows a marketing page or a component
library. A text node is the same whether it holds a design note or a code
snippet. The spatial model is universal.

New capabilities should emerge from combining existing primitives, not from
building custom one-off features.

### Interoperability

Data flows in and out. .canvas files work in Obsidian and other tools that
support the spec. Agents can read and write canvas files directly as JSON.
The app is a surface other tools can talk to.

### Spatial thinking over linear workflows

Arrangement, proximity, and grouping carry meaning on the canvas. The canvas
is not just a container — it's a thinking tool. Two pages placed side by side
are being compared. A cluster of references is a research thread. The spatial
layout is part of the work product.

### Browser-native fidelity

Web content renders in real Electron webviews, not screenshots or proxies.
Users can interact with live pages, inspect DOM, navigate, and see responsive
behavior at different viewport presets. Page nodes are real browser surfaces,
even though the app stays organized as one spatial canvas.

### Workflows are emergent

The app provides primitives and tools. Users compose their own workflows.
Common patterns (multi-breakpoint iteration, live annotation, research boards,
wireframing) emerge from how people use the building blocks — they are not
hard-coded as dedicated features.

## What it is not

- Not a full-featured browser (no tabs mode, extensions, bookmarks)
- Not a pixel-perfect design tool (no vector editing, no export to Figma)
- Not a project management tool (no tasks, no timelines)
- Not a collaborative real-time editor (local-first, single user)

It is the tool you reach for when you want to see, arrange, and think about
web content spatially — and then hand off artifacts (annotations, layouts,
references) to the tools that finish the job.
