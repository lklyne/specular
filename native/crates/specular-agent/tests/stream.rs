//! One line of `claude --output-format stream-json` and what it means.

use serde_json::{Value, json};
use specular_agent::{Notice, Progress, ProgressKind, parse_line, parse_output};

fn progress(kind: ProgressKind, text: &str, label: Option<&str>) -> Notice {
    Notice::Progress(Progress {
        kind,
        text: text.into(),
        label: label.map(Into::into),
    })
}

type Case = (String, Vec<Notice>);

fn assistant_line(blocks: &Value) -> String {
    json!({ "type": "assistant", "message": { "content": blocks } }).to_string()
}

/// A `tool_use` line and the one progress line it becomes.
fn tool(name: &str, input: &Value, text: &str, label: &str) -> Case {
    let line = assistant_line(&json!([{ "type": "tool_use", "name": name, "input": input }]));
    (
        line,
        vec![progress(ProgressKind::ToolUse, text, Some(label))],
    )
}

/// A `tool_result` user line and the progress line it becomes.
fn tool_results(results: &Value, text: &str) -> Case {
    let blocks: Vec<Value> = results
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| json!({ "type": "tool_result", "content": r["content"], "is_error": r["is_error"] }))
        .collect();
    let line = json!({ "type": "user", "message": { "content": blocks } }).to_string();
    (line, vec![progress(ProgressKind::ToolResult, text, None)])
}

fn cut(text: &str, max: usize) -> String {
    format!("{}…", text.chars().take(max - 1).collect::<String>())
}

/// How each tool, text, result and limit reads on the run bar.
#[expect(clippy::too_many_lines, reason = "one table of described lines")]
fn described_cases() -> Vec<Case> {
    let a = |n: usize| "a".repeat(n);
    let two_blocks = assistant_line(&json!([
        { "type": "text", "text": " one " },
        { "type": "text", "text": "   " },
        { "type": "text", "text": "two" },
    ]));
    let wide = assistant_line(&json!([
        { "type": "text", "text": a(200) },
        { "type": "text", "text": a(200) },
    ]));
    let sentence = |text: &str| assistant_line(&json!([{ "type": "text", "text": text }]));
    let label_of = |text: &str, label: &str| -> Case {
        let trimmed = text.trim();
        (
            sentence(text),
            vec![
                progress(ProgressKind::Text, trimmed, Some(label)),
                Notice::Text(trimmed.into()),
            ],
        )
    };
    vec![
        (
            two_blocks,
            vec![
                progress(ProgressKind::Text, "one | two", Some("two")),
                Notice::Text("one\n\ntwo".into()),
            ],
        ),
        (
            wide,
            vec![
                progress(
                    ProgressKind::Text,
                    &format!("{} | {}…", a(200), a(116)),
                    Some(&cut(&a(200), 80)),
                ),
                Notice::Text(format!("{}\n\n{}", a(200), a(200))),
            ],
        ),
        (
            sentence(&a(241)),
            vec![
                progress(
                    ProgressKind::Text,
                    &cut(&a(241), 240),
                    Some(&cut(&a(241), 80)),
                ),
                Notice::Text(a(241)),
            ],
        ),
        label_of("Hi! Then more", "Hi!"),
        label_of("Wait? ok", "Wait?"),
        label_of("v1.2 is out", "v1.2 is out"),
        label_of("  \nSecond line. x", "Second line."),
        (
            assistant_line(&json!([{ "type": "thinking", "thinking": "t".repeat(200) }])),
            vec![progress(
                ProgressKind::Text,
                &format!("(thinking) {}", cut(&"t".repeat(200), 180)),
                Some("Thinking"),
            )],
        ),
        (
            r#"{"type":"system","subtype":"init"}"#.into(),
            vec![progress(ProgressKind::System, "init session", None)],
        ),
        (
            r#"{"type":"result"}"#.into(),
            vec![
                progress(ProgressKind::Result, "done", Some("Wrapping up")),
                Notice::Failed {
                    error: "error".into(),
                },
            ],
        ),
        (
            json!({ "type": "result", "subtype": "e".repeat(250) }).to_string(),
            vec![
                progress(ProgressKind::Result, &"e".repeat(250), Some("Wrapping up")),
                Notice::Failed {
                    error: cut(&"e".repeat(250), 200),
                },
            ],
        ),
        (
            json!({ "type": "result", "subtype": "success", "result": "z".repeat(250) })
                .to_string(),
            vec![
                progress(
                    ProgressKind::Result,
                    &cut(&"z".repeat(250), 200),
                    Some("Wrapping up"),
                ),
                Notice::Finished {
                    text: "z".repeat(250),
                },
            ],
        ),
        (
            json!({ "type": "stream_event", "event": { "type": "content_block_delta",
                "delta": { "type": "thinking_delta", "text": "hm" } } })
            .to_string(),
            vec![],
        ),
        tool(
            "MultiEdit",
            &json!({ "file_path": "/a/x.rs" }),
            "MultiEdit /a/x.rs",
            "Editing x.rs",
        ),
        tool(
            "NotebookEdit",
            &json!({ "notebook_path": "/a/n.ipynb" }),
            "NotebookEdit",
            "Editing n.ipynb",
        ),
        tool(
            "Write",
            &json!({ "file_path": "/a/w.md" }),
            "Write /a/w.md",
            "Writing w.md",
        ),
        tool(
            "Read",
            &json!({ "file_path": "C:\\a\\b.rs" }),
            "Read C:\\a\\b.rs",
            "Reading b.rs",
        ),
        tool("Read", &json!({}), "Read", "Reading"),
        tool(
            "Read",
            &json!({ "file_path": "  ", "path": "/a/y.rs" }),
            "Read /a/y.rs",
            "Reading y.rs",
        ),
        tool(
            "Grep",
            &json!({ "pattern": "foo" }),
            "Grep foo",
            "Searching for “foo”",
        ),
        tool("Grep", &json!({}), "Grep", "Searching"),
        tool(
            "Glob",
            &json!({ "pattern": "**/*.rs" }),
            "Glob **/*.rs",
            "Finding files",
        ),
        tool(
            "WebSearch",
            &json!({ "query": "css grid" }),
            "WebSearch css grid",
            "Searching the web for “css grid”",
        ),
        tool("WebSearch", &json!({}), "WebSearch", "Searching the web"),
        tool("WebFetch", &json!({}), "WebFetch", "Reading the web"),
        tool(
            "WebFetch",
            &json!({ "url": "http://[::1]:3000/x" }),
            "WebFetch http://[::1]:3000/x",
            "Reading [::1]",
        ),
        tool(
            "WebFetch",
            &json!({ "url": "https://me:pw@Host.com/x" }),
            "WebFetch https://me:pw@Host.com/x",
            "Reading host.com",
        ),
        tool(
            "WebFetch",
            &json!({ "url": "x".repeat(50) }),
            &format!("WebFetch {}", "x".repeat(50)),
            &format!("Reading {}", cut(&"x".repeat(50), 40)),
        ),
        tool(
            "Task",
            &json!({ "description": "d".repeat(100) }),
            "Task",
            &cut(&"d".repeat(100), 80),
        ),
        tool("Task", &json!({}), "Task", "Delegating to a subagent"),
        tool("Agent", &json!({}), "Agent", "Delegating to a subagent"),
        tool("TodoWrite", &json!({}), "TodoWrite", "Planning"),
        tool("Bash", &json!({}), "Bash", "Running a command"),
        tool(
            "Bash",
            &json!({ "cmd": "ls" }),
            "Bash ls",
            "Running a command",
        ),
        tool(
            "Bash",
            &json!({ "command": "c".repeat(50) }),
            &format!("Bash {}", "c".repeat(50)),
            &format!("Running {}", cut(&"c".repeat(50), 40)),
        ),
        tool(
            "Bash",
            &json!({ "command": "ls", "description": "d".repeat(100) }),
            "Bash ls",
            &cut(&"d".repeat(100), 80),
        ),
        tool("Foo", &json!({ "query": "q" }), "Foo q", "Using Foo"),
        tool(
            "Foo",
            &json!({ "pattern": "p", "query": "q" }),
            "Foo p",
            "Using Foo",
        ),
        tool(
            "mcp__srv__do_it",
            &json!({}),
            "mcp__srv__do_it",
            "Using do it",
        ),
        tool_results(&json!([{ "content": "a" }, { "content": "b" }]), "a | b"),
        tool_results(&json!([{ "content": "  " }]), "(empty output)"),
        tool_results(&json!([{ "content": "", "is_error": true }]), "tool error"),
        tool_results(&json!([{ "content": [] }]), "tool result"),
        tool_results(&json!([{ "content": [], "is_error": true }]), "tool error"),
        tool_results(
            &json!([{ "content": [{ "type": "audio" }] }]),
            "tool result",
        ),
        tool_results(&json!([{ "content": null }]), "tool result"),
        tool_results(
            &json!([{ "content": [{ "type": "image", "mimeType": "image/jpeg" }] }]),
            "image (image/jpeg)",
        ),
        tool_results(
            &json!([{ "content": [{ "type": "image" }] }]),
            "image (image)",
        ),
        tool_results(
            &json!([{ "content": [{ "type": "text", "text": "w".repeat(250) }] }]),
            &cut(&"w".repeat(250), 200),
        ),
        tool_results(
            &json!([{ "content": [{ "type": "text", "text": " " }, { "type": "text", "text": "kept" }] }]),
            "kept",
        ),
    ]
}

#[test]
#[expect(clippy::too_many_lines, reason = "one table of literal stream lines")]
fn stream_lines_become_notices() {
    let long = "x".repeat(300);
    let long_line = format!(
        r#"{{"type":"user","message":{{"content":[{{"type":"tool_result","content":"{long}"}}]}}}}"#
    );
    let capped = format!("{}…", "x".repeat(199));
    let cases: Vec<(String, Vec<Notice>)> = vec![
        (
            r#"{"type":"system","subtype":"init","session_id":"s1","model":"sonnet"}"#.into(),
            vec![Notice::Session("s1".into()), progress(ProgressKind::System, "init sonnet", None)],
        ),
        (r#"{"type":"system","subtype":"status","session_id":"s1"}"#.into(), vec![]),
        (
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Looking at it now. Then more."}]}}"#.into(),
            vec![
                progress(ProgressKind::Text, "Looking at it now. Then more.", Some("Looking at it now.")),
                Notice::Text("Looking at it now. Then more.".into()),
            ],
        ),
        (
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Reading."},{"type":"tool_use","name":"Read","input":{"file_path":"/a/b/c.ts"}}]}}"#.into(),
            vec![
                progress(ProgressKind::Text, "Reading. | Read /a/b/c.ts", Some("Reading c.ts")),
                Notice::Text("Reading.".into()),
            ],
        ),
        (
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"  pnpm typecheck --all"}}]}}"#.into(),
            vec![progress(ProgressKind::ToolUse, "Bash   pnpm typecheck --all", Some("Running pnpm"))],
        ),
        (
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"ls","description":"List files"}}]}}"#.into(),
            vec![progress(ProgressKind::ToolUse, "Bash ls", Some("List files"))],
        ),
        (
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"mcp__specular__create_page","input":{}}]}}"#.into(),
            vec![progress(ProgressKind::ToolUse, "mcp__specular__create_page", Some("Using create page"))],
        ),
        (
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"WebFetch","input":{"url":"https://Docs.Example.com:8080/x?y=1"}}]}}"#.into(),
            vec![progress(ProgressKind::ToolUse, "WebFetch https://Docs.Example.com:8080/x?y=1", Some("Reading docs.example.com"))],
        ),
        (
            r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"hmm"}]}}"#.into(),
            vec![progress(ProgressKind::Text, "(thinking) hmm", Some("Thinking"))],
        ),
        (
            r#"{"type":"assistant","parent_tool_use_id":"tu1","message":{"content":[{"type":"text","text":"sub"}]}}"#.into(),
            vec![progress(ProgressKind::Text, "sub", Some("sub"))],
        ),
        (
            r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"first line\nsecond"}]}}"#.into(),
            vec![progress(ProgressKind::ToolResult, "first line", None)],
        ),
        (
            r#"{"type":"user","message":{"content":[{"type":"tool_result","is_error":true,"content":"boom\nmore"}]}}"#.into(),
            vec![progress(ProgressKind::ToolResult, "tool error: boom", None)],
        ),
        (
            r#"{"type":"user","message":{"content":[{"type":"tool_result","content":[{"type":"text","text":"ok\nrest"},{"type":"image","source":{"media_type":"image/png"}}]}]}}"#.into(),
            vec![progress(ProgressKind::ToolResult, "ok · image (image/png)", None)],
        ),
        (r#"{"type":"user","message":{"content":[{"type":"text","text":"typed"}]}}"#.into(), vec![]),
        (long_line, vec![progress(ProgressKind::ToolResult, &capped, None)]),
        (
            r#"{"type":"stream_event","event":{"type":"message_start"}}"#.into(),
            vec![Notice::Text(String::new())],
        ),
        (
            r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hel"}}}"#.into(),
            vec![Notice::TextDelta("Hel".into())],
        ),
        (
            r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"{"}}}"#.into(),
            vec![],
        ),
        (
            r#"{"type":"stream_event","parent_tool_use_id":"tu1","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"sub"}}}"#.into(),
            vec![],
        ),
        (
            r#"{"type":"result","subtype":"success","is_error":false,"result":"Done up top.\nLast words.\n<<RESOLVE>>"}"#.into(),
            vec![
                progress(ProgressKind::Result, "<<RESOLVE>>", Some("Wrapping up")),
                Notice::Finished { text: "Done up top.\nLast words.\n<<RESOLVE>>".into() },
            ],
        ),
        (
            r#"{"type":"result","subtype":"error_during_execution","errors":["a","b"]}"#.into(),
            vec![
                progress(ProgressKind::Result, "error_during_execution", Some("Wrapping up")),
                Notice::Failed { error: "a; b".into() },
            ],
        ),
        (
            r#"{"type":"result","subtype":"success","is_error":true,"result":"Not logged in"}"#.into(),
            vec![
                progress(ProgressKind::Result, "Not logged in", Some("Wrapping up")),
                Notice::Failed { error: "Not logged in".into() },
            ],
        ),
        (
            r#"{"type":"result","subtype":"error_max_turns"}"#.into(),
            vec![
                progress(ProgressKind::Result, "error_max_turns", Some("Wrapping up")),
                Notice::Failed { error: "error_max_turns".into() },
            ],
        ),
        (r#"{"type":"rate_limit_event"}"#.into(), vec![]),
        ("not json".into(), vec![]),
        (String::new(), vec![]),
    ];
    for (line, expected) in cases.into_iter().chain(described_cases()) {
        assert_eq!(parse_line(&line), expected, "{line}");
    }
}

#[test]
fn the_last_marker_decides_and_is_cut_from_the_reply() {
    let cases = [
        ("Fixed it.\n<<RESOLVE>>", "Fixed it.", true),
        ("Which page?\n<<WAITING>>", "Which page?", false),
        ("a <<WAITING>> b <<RESOLVE>>", "a <<WAITING>> b", true),
        ("a <<RESOLVE>> b <<WAITING>>", "a <<RESOLVE>> b", false),
        ("  plain answer  ", "plain answer", false),
        ("<<RESOLVE>>", "(no summary)", true),
        ("   ", "(no output)", false),
    ];
    for (text, summary, resolve) in cases {
        let reply = parse_output(text);
        assert_eq!(
            (reply.summary.as_str(), reply.should_resolve),
            (summary, resolve),
            "{text}"
        );
    }
    assert_eq!(
        parse_output(&"y".repeat(60_000)).summary.chars().count(),
        50_000
    );
}
