//! The arguments `claude` is started with.

use specular_agent::{
    ALLOWED_TOOLS, Permissions, RunConfig, RunRequest, SPECULAR_CONTEXT, ThreadId, claude_args,
};

#[test]
fn the_permission_mode_and_resume_pick_the_arguments() {
    let fresh = RunRequest {
        thread: ThreadId("t".into()),
        prompt: "p".into(),
        resume: None,
        images: Vec::new(),
        cwd: None,
    };
    let resume = RunRequest {
        resume: Some("sess".into()),
        ..fresh.clone()
    };
    let config = |permissions| RunConfig {
        model: "opus".into(),
        permissions,
    };

    let mut auto: Vec<String> = [
        "-p",
        "--output-format",
        "stream-json",
        "--verbose",
        "--include-partial-messages",
        "--model",
        "opus",
        "--permission-mode",
        "auto",
        "--allowedTools",
    ]
    .map(String::from)
    .into();
    auto.extend(ALLOWED_TOOLS.map(String::from));
    auto.extend([
        "--append-system-prompt".to_owned(),
        SPECULAR_CONTEXT.to_owned(),
        "--settings".into(),
        r#"{"outputStyle":"Concise"}"#.into(),
    ]);
    assert_eq!(claude_args(&fresh, &config(Permissions::Auto)), auto);

    let accept = claude_args(&fresh, &config(Permissions::AcceptEdits));
    assert_eq!(accept[8], "acceptEdits");
    assert_eq!(accept[9], "--allowedTools");
    assert_eq!(accept.len(), auto.len());

    let open = claude_args(&resume, &config(Permissions::Dangerously));
    assert_eq!(open[7..9], ["--permission-mode", "bypassPermissions"]);
    assert!(!open.contains(&"--allowedTools".to_owned()));
    assert_eq!(open[open.len() - 2..], ["--resume", "sess"]);
    assert!(!claude_args(&fresh, &config(Permissions::Auto)).contains(&"--resume".to_owned()));

    let default = RunConfig::default();
    assert_eq!(
        (default.model.as_str(), default.permissions),
        ("sonnet", Permissions::Auto)
    );
}
