//! One line of `claude --output-format stream-json` and what it means.

use specular_agent::{Notice, Progress, ProgressKind, parse_line, parse_output};

fn progress(kind: ProgressKind, text: &str, label: Option<&str>) -> Notice {
    Notice::Progress(Progress {
        kind,
        text: text.into(),
        label: label.map(Into::into),
    })
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
    for (line, expected) in cases {
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
