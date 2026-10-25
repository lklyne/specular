# Specular

Part web browser and part canvas, specular is a hybrid design tool for thinking through ideas spatially and iterating on software.

Code is the source of truth for making ideas real, but long chat threads and single browser tabs aren't great for the exploratory, divergent thinking that designing something great requires. A canvas fits that work better — but there's usually friction moving back and forth between canvas and code. Specular sidesteps it with full-featured browser tabs on a canvas: the actual website, product, or prototype lives spatially alongside notes, images, and drawings, designed so people and agents can vibe out designs and research in one place.

<!-- TODO: Add screenshot or GIF demo here -->

## What you can do

- Explore different directions and see them side-by-side
- Annotate live websites and pass the feedback straight back to an agent
- Lay out a page at multiple device breakpoints to check responsiveness
- Ask an agent to share its thinking visually, or ingest a repo's design system
- Open a page or document in its own tab to work on it alone

## Key features

- **Canvas** — Real browser windows on an infinite, zoomable canvas
- **Agent-friendly** — Agents drive the canvas through the `specular` CLI: creating pages, navigating, inspecting the DOM, clicking, typing, screenshotting. An agent chat runs Claude Code beside the canvas
- **Agent presence** — See an agent's live cursor and task status as it works alongside you
- **Annotations** — Comment on any page, usable by people and agents
- **Device frames** — Preview sites at preset device sizes with visual device shells
- **Groups & edges** — Organize items into freeform or auto-layout groups and draw connections between them
- **Local-first** — No sign-in, no account; full undo history, kept per canvas
- **Open formats** — Layout uses [JSON Canvas](https://jsoncanvas.org); text and media are plain `.md`, `.png`, and `.webm` files on disk

## Inspiration and related products
- [Paper](https://paper.design): imo the best full-featured design tool with agent collaboration
- [Agentation](https://agentation.com): inspiration for visual edits and commenting
- [Polypane](https://polypane.app): for viewing a webpage across multiple breakpoints in one place
- [Obsidian](https://obsidian.md): inspiration from local storage format and its lightweight canvas
- [agent-browser](https://agent-browser.dev): inspiration for cli based web automation. Wrapped in specular skill for browser automation.

## System requirements

- macOS 12+ (Apple Silicon or Intel)
- Windows and Linux builds aren't currently planned

## Installation

Download the latest release from the [GitHub Releases](https://github.com/lklyne/specular/releases) page.

The app updates itself with [Sparkle](https://sparkle-project.org) from the same releases.

## Using with AI agents

The `specular` CLI is the main interface for agents — composable commands that fit an agent's working loop:

```bash
specular canvas                          # inspect the current canvas
specular create page <url>               # pull a live page onto the canvas
specular snapshot -i -f <pageId>         # get element refs for a page
specular annotate "<feedback>"           # leave a comment for a human or agent
```

A Claude Code skill ships with the app — see [`resources/skills/specular/SKILL.md`](resources/skills/specular/SKILL.md) for the full surface. The app's setup dialog puts the command on your `PATH` and installs the skill.

## Building from source

The app is Rust, under [`native/`](native/). See [`native/README.md`](native/README.md) to build and run it, and [`CLAUDE.md`](CLAUDE.md) for the gate.

## Security

To report a security vulnerability, see [SECURITY.md](SECURITY.md).

## License

Licensed under [PolyForm Shield 1.0.0](LICENSE.md). Copyright © 2026 Lyle Klyne.
