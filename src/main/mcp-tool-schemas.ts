// ---------------------------------------------------------------------------
// MCP tool schema definitions
// ---------------------------------------------------------------------------

export const toolSchemas = [
  {
    name: 'get_workspace',
    description: 'Return the current Specular workspace graph, selection, and occupied regions. Text entities include a preview of the first 80 characters — use get_text_entities for full content.',
    inputSchema: {
      type: 'object',
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: 'get_selection',
    description: 'Return the current page or group selection.',
    inputSchema: {
      type: 'object',
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: 'find_placement',
    description: 'Find a non-overlapping placement for a proposed cluster.',
    inputSchema: {
      type: 'object',
      properties: {
        width: { type: 'number' },
        height: { type: 'number' },
        anchor: {
          type: 'string',
          enum: ['selection_or_empty_region', 'empty_region'],
        },
      },
      required: ['width', 'height'],
      additionalProperties: false,
    },
  },
  {
    name: 'apply_task_layout',
    description: 'Create a breakpoint cluster for a URL using mobile, tablet, and desktop presets.',
    inputSchema: {
      type: 'object',
      properties: {
        task_kind: { type: 'string', enum: ['breakpoint_map'] },
        url: { type: 'string' },
        presets: {
          type: 'array',
          items: { type: 'string' },
        },
        anchor: {
          type: 'string',
          enum: ['selection_or_empty_region', 'empty_region'],
        },
        focus: { type: 'boolean' },
        label: { type: 'string' },
      },
      required: ['url'],
      additionalProperties: false,
    },
  },
  {
    name: 'upsert_entities',
    description: `Create or update canvas entities (pages, text notes, file attachments) in a single call.
If id matches an existing entity → update. No id → create.

Kind-specific fields:
  page — url, presetIndex, canvasX, canvasY, orientation, showDeviceFrame (default true), groupId
  text  — text (Markdown), color (hex "#RRGGBB" or preset 1-6: red/orange/yellow/green/cyan/purple), canvasX, canvasY, width, height
  file  — file (absolute path), subpath, canvasX, canvasY, width, height
         Files ending in .wireframe.json render as interactive wireframe editors (schema: the specular skill's references/wireframes.md).

Page presets (presetIndex → device):
  0: iPhone SE (375×667, mobile)     3: iPad Mini (744×1133)       6: Laptop (1280×800)
  1: iPhone 14 Pro (393×852, mobile) 4: iPad Pro 11 (834×1194)     7: Desktop (1440×900)
  2: iPhone 14 Pro Max (430×932)     5: iPad Pro 12.9 (1024×1366)  8: Desktop XL (1920×1080)
Portrait dimensions for phones/tablets. Use orientation: "landscape" to swap.`,
    inputSchema: {
      type: 'object',
      properties: {
        items: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              kind: { type: 'string', enum: ['page', 'text', 'file'], description: 'Entity type.' },
              id: { type: 'string', description: 'Entity ID. Present = update, absent = create.' },
              canvasX: { type: 'number' },
              canvasY: { type: 'number' },
              width: { type: 'number' },
              height: { type: 'number' },
              // Page
              url: { type: 'string' },
              presetIndex: { type: 'number', description: 'Device preset index (0-8).' },
              orientation: { type: 'string', enum: ['portrait', 'landscape'] },
              showDeviceFrame: { type: 'boolean', description: 'Show device bezel. Default true for new pages.' },
              groupId: { type: 'string' },
              // Text
              text: { type: 'string', description: 'Markdown content (text entity).' },
              color: { type: 'string', description: 'Background color (text entity).' },
              // File
              file: { type: 'string', description: 'Absolute file path (file entity).' },
              subpath: { type: 'string', description: 'Subpath within file (file entity).' },
            },
            required: ['kind'],
            additionalProperties: true,
          },
        },
      },
      required: ['items'],
      additionalProperties: false,
    },
  },
  {
    name: 'link_pages',
    description: 'Create edges (connections) between any canvas entities (pages, text blocks, file blocks).',
    inputSchema: {
      type: 'object',
      properties: {
        edges: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              id: { type: 'string' },
              fromEntityId: { type: 'string' },
              toEntityId: { type: 'string' },
              kind: { type: 'string', enum: ['breakpoint_variant'] },
            },
            required: ['fromEntityId', 'toEntityId', 'kind'],
            additionalProperties: true,
          },
        },
      },
      required: ['edges'],
      additionalProperties: false,
    },
  },
  {
    name: 'unlink_pages',
    description: 'Delete semantic links by edge ID.',
    inputSchema: {
      type: 'object',
      properties: {
        edgeIds: {
          type: 'array',
          items: { type: 'string' },
        },
      },
      required: ['edgeIds'],
      additionalProperties: false,
    },
  },
  {
    name: 'focus_pages',
    description: 'Focus the canvas camera on pages, groups, or explicit bounds.',
    inputSchema: {
      type: 'object',
      properties: {
        pageIds: {
          type: 'array',
          items: { type: 'string' },
        },
        groupIds: {
          type: 'array',
          items: { type: 'string' },
        },
        bounds: {
          type: 'object',
          properties: {
            x: { type: 'number' },
            y: { type: 'number' },
            width: { type: 'number' },
            height: { type: 'number' },
          },
          required: ['x', 'y', 'width', 'height'],
          additionalProperties: false,
        },
      },
      additionalProperties: false,
    },
  },
  {
    name: 'create_group',
    description: 'Create a group from existing canvas entities.',
    inputSchema: {
      type: 'object',
      properties: {
        entity_ids: {
          type: 'array',
          items: { type: 'string' },
        },
        label: { type: 'string' },
      },
      required: ['entity_ids'],
      additionalProperties: false,
    },
  },
  {
    name: 'ungroup_group',
    description: 'Ungroup a group and return its freed entity IDs.',
    inputSchema: {
      type: 'object',
      properties: {
        group_id: { type: 'string' },
      },
      required: ['group_id'],
      additionalProperties: false,
    },
  },
  {
    name: 'delete_groups',
    description: 'Delete groups and optionally all member pages.',
    inputSchema: {
      type: 'object',
      properties: {
        group_ids: {
          type: 'array',
          items: { type: 'string' },
        },
        delete_member_pages: { type: 'boolean' },
        focus_after: { type: 'boolean' },
      },
      required: ['group_ids'],
      additionalProperties: false,
    },
  },
  {
    name: 'register_design_system',
    description:
      'Register a design system manifest with component and token definitions.',
    inputSchema: {
      type: 'object',
      properties: {
        manifest: {
          type: 'object',
          additionalProperties: true,
        },
      },
      required: ['manifest'],
      additionalProperties: false,
    },
  },
  {
    name: 'get_design_system',
    description: 'Return the currently registered design system manifest.',
    inputSchema: {
      type: 'object',
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: 'layout_component_states',
    description:
      'Create a page grid showing different states for a design system component.',
    inputSchema: {
      type: 'object',
      properties: {
        component: { type: 'string' },
        url: { type: 'string' },
        vary: {
          type: 'array',
          items: { type: 'string' },
        },
        values: {
          type: 'object',
          additionalProperties: true,
        },
        states: {
          type: 'array',
          items: { type: 'string' },
        },
        tokens: {
          type: 'object',
          additionalProperties: { type: 'string' },
        },
        selector: { type: 'string' },
        anchor: {
          type: 'string',
          enum: ['selection_or_empty_region', 'empty_region'],
        },
        focus: { type: 'boolean' },
        label: { type: 'string' },
      },
      required: ['component', 'url', 'vary'],
      additionalProperties: false,
    },
  },
  {
    name: 'browse',
    description:
      'Run an agent-browser command inside a page. Handles CDP connection, presence animation, and per-page serialization automatically.\n\nCommon commands:\n- "snapshot" / "snapshot -i" — accessibility tree with element refs (@eN)\n- "snapshot -s \\"#main\\"" — scope snapshot to a CSS selector\n- "click @eN" — click an element\n- "fill @eN text" — fill an input\n- "type @eN text" — type into an element\n- "select @eN value" — select a dropdown option\n- "scroll down" / "scroll up" — scroll the page\n- "wait --load networkidle" — wait for page to settle\n- "get text" / "get url" — read page content\n- "screenshot" — capture a PNG image\n- "screenshot --annotate" — labeled screenshot with ref overlay (snapshot + screenshot in one)\n- "diff snapshot" — show changes since last snapshot (+/- format)\n- "find text \\"Sign In\\" click" — semantic locators (no refs needed)\n- "console" / "errors" — page diagnostics\n- "query-elements selector" — find elements by CSS selector\n\nCommand chaining: use "cmd1 && cmd2 && cmd3" to run multiple commands in a single call with shared element refs. Example: "snapshot -i && click @e3 && get url".\n\nMutation commands (click, fill, type, select) automatically return the current URL.\n\nAfter mutations, re-snapshot to get fresh refs — element refs are per-snapshot and become stale after DOM changes.',
    inputSchema: {
      type: 'object',
      properties: {
        page_id: {
          type: 'string',
          description: 'Page to interact with. Defaults to the selected page.',
        },
        command: {
          type: 'string',
          description: 'The agent-browser command to run (e.g. "snapshot -i", "click @e5", "fill @e12 hello").',
        },
        echo: {
          type: 'boolean',
          description: 'After a successful mutation, append a fresh interactive snapshot to the result.',
        },
      },
      required: ['command'],
      additionalProperties: false,
    },
  },
  {
    name: 'create_annotation',
    description:
      'Create a new user-visible comment thread on the canvas or an element. The anchor type discriminates element / canvas / page / region (ADR 0006).',
    inputSchema: {
      type: 'object',
      properties: {
        text: { type: 'string' },
        anchor: {
          type: 'object',
          additionalProperties: true,
          description:
            "Annotation anchor. Examples: { type: 'canvas', canvasX, canvasY }, { type: 'page', pageId, offsetX, offsetY }, { type: 'element', pageId, selector, elementPath?, boundingBox? }, { type: 'region', canvasRect: { x, y, width, height } }. Always pass a region as canvasRect; a marquee that grabbed page content is stored page-relative internally so it scroll-follows.",
        },
        metadata: {
          type: 'object',
          additionalProperties: true,
        },
      },
      required: ['text', 'anchor'],
      additionalProperties: false,
    },
  },
  {
    name: 'get_annotations',
    description:
      'Return lightweight annotation stubs (comments, feedback) left by the user on the canvas. Includes summaries but not full metadata — use get_annotation_detail to fetch screenshots, DOM elements, and inspect context for a specific annotation.',
    inputSchema: {
      type: 'object',
      properties: {
        status: {
          type: 'string',
          enum: ['pending', 'acknowledged', 'resolved', 'dismissed'],
          description: 'Filter by annotation status. Omit to get all.',
        },
        url: {
          type: 'string',
          description: 'Filter annotations by canonical page URL.',
        },
        page_id: {
          type: 'string',
          description: 'Filter annotations by page id.',
        },
      },
      additionalProperties: false,
    },
  },
  {
    name: 'get_annotation_detail',
    description:
      'Get full detail for a single annotation including screenshot, DOM elements, computed styles, and component context. Use after get_annotations to drill into a specific annotation.',
    inputSchema: {
      type: 'object',
      properties: {
        annotation_id: {
          type: 'string',
          description: 'The annotation ID to fetch detail for.',
        },
        include_screenshot: {
          type: 'boolean',
          description:
            'Whether to include the region screenshot image. Defaults to true.',
        },
      },
      required: ['annotation_id'],
      additionalProperties: false,
    },
  },
  {
    name: 'acknowledge_annotation',
    description:
      'Mark an annotation as acknowledged — you have seen and understood the feedback.',
    inputSchema: {
      type: 'object',
      properties: {
        annotation_id: { type: 'string' },
      },
      required: ['annotation_id'],
      additionalProperties: false,
    },
  },
  {
    name: 'resolve_annotation',
    description:
      'Mark an annotation as resolved — the requested change has been made.',
    inputSchema: {
      type: 'object',
      properties: {
        annotation_id: { type: 'string' },
      },
      required: ['annotation_id'],
      additionalProperties: false,
    },
  },
  {
    name: 'dismiss_annotation',
    description:
      'Dismiss an annotation with a reason — you have decided not to address this feedback.',
    inputSchema: {
      type: 'object',
      properties: {
        annotation_id: { type: 'string' },
        reason: { type: 'string' },
      },
      required: ['annotation_id'],
      additionalProperties: false,
    },
  },
  {
    name: 'reply_to_annotation',
    description:
      'Add a reply to an annotation thread — ask a clarifying question or provide an update.',
    inputSchema: {
      type: 'object',
      properties: {
        annotation_id: { type: 'string' },
        text: { type: 'string' },
      },
      required: ['annotation_id', 'text'],
      additionalProperties: false,
    },
  },
  {
    name: 'get_text_entities',
    description: 'Return all text entities on the canvas.',
    inputSchema: {
      type: 'object',
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: 'get_file_entities',
    description: 'Return all file entities (images, attachments, wireframes) on the canvas. Files ending in .wireframe.json are rendered as interactive wireframe editors.',
    inputSchema: {
      type: 'object',
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: 'start_recording',
    description:
      'Start a composite video recording of a page. Captures the page content with agent cursor overlay composited on top. Output is a VP9 WebM file.',
    inputSchema: {
      type: 'object',
      properties: {
        page_id: { type: 'string', description: 'Page to record.' },
        output_path: { type: 'string', description: 'Optional output file path. Defaults to a temp directory.' },
        fps: { type: 'number', description: 'Capture page rate (default 30, max 60).' },
        quality: {
          type: 'string',
          enum: ['high', 'medium', 'compact'],
          description: 'Quality preset. high=60fps/crf20, medium=30fps/crf30, compact=30fps/crf40.',
        },
      },
      required: ['page_id'],
      additionalProperties: false,
    },
  },
  {
    name: 'stop_recording',
    description:
      'Stop the current composite video recording. Returns the output path, duration, page count, and activity segments.',
    inputSchema: {
      type: 'object',
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: 'get_recording_status',
    description:
      'Get the current recording state (idle or recording), including elapsed time and page count.',
    inputSchema: {
      type: 'object',
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: 'trim_recording',
    description:
      'Trim idle segments from a recorded video. Uses activity segments to remove or speed up periods where the agent was idle.',
    inputSchema: {
      type: 'object',
      properties: {
        input_path: { type: 'string', description: 'Path to the recorded WebM file.' },
        output_path: { type: 'string', description: 'Optional output path for the trimmed file.' },
        min_idle_ms: { type: 'number', description: 'Minimum idle duration (ms) before trimming. Default: 3000.' },
        idle_speed_factor: { type: 'number', description: 'Speed multiplier for idle segments (e.g. 4 = 4x speed). Default: 4.' },
      },
      required: ['input_path'],
      additionalProperties: false,
    },
  },
  {
    name: 'delete_entities',
    description:
      'Delete a batch of mixed canvas entities (pages, text notes, file attachments) in a single call. Items are removed sequentially with animated cursor movement.',
    inputSchema: {
      type: 'object',
      properties: {
        items: {
          type: 'array',
          description: 'Array of entities to delete. Each must include `kind` and `id`.',
          items: {
            type: 'object',
            properties: {
              kind: { type: 'string', enum: ['page', 'text', 'file'], description: 'Entity type' },
              id: { type: 'string', description: 'Entity ID to delete' },
            },
            required: ['kind', 'id'],
          },
        },
      },
      required: ['items'],
      additionalProperties: false,
    },
  },
] as const
