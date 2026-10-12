//! The helper's list of models.

use specular_agent::{Model, parse_models};

#[test]
fn the_helpers_models_line_keeps_the_families_in_order_and_other_lines_are_not_read() {
    let line = r#"{"type":"helper","event":"models","models":[
        {"value":"default","resolvedModel":"claude-opus-5-5","displayName":"Default (recommended)","description":"Opus 5.5"},
        {"value":"opus","displayName":"Opus 5.5","description":"For complex work"},
        {"value":"haiku","displayName":"Haiku 5.5"},
        {"value":"claude-sonnet-5","displayName":"Sonnet 5"},
        {"value":"fable","displayName":"Fable 5.1"},
        {"displayName":"No value"}],"account":{}}"#
        .replace('\n', "");
    let model = |value: &str, name: &str, description: &str| Model {
        value: value.into(),
        name: name.into(),
        description: description.into(),
    };
    assert_eq!(
        parse_models(&line),
        Some(vec![
            model("fable", "Fable 5.1", ""),
            model("opus", "Opus 5.5", "For complex work"),
            model("haiku", "Haiku 5.5", ""),
        ])
    );
    assert_eq!(
        parse_models(r#"{"type":"helper","event":"error","message":"no"}"#),
        None
    );
    assert_eq!(parse_models(r#"{"type":"system","subtype":"init"}"#), None);
    assert_eq!(parse_models("not json"), None);
}
