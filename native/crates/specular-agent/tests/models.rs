//! The helper's list of models.

use specular_agent::{Model, parse_models};

#[test]
fn the_helpers_models_line_is_read_and_other_lines_are_not() {
    let line = r#"{"type":"helper","event":"models","models":[
        {"value":"default","resolvedModel":"claude-opus-5-5","displayName":"Default (recommended)","description":"Opus 5.5"},
        {"value":"haiku","displayName":"Haiku 4.5"},
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
            model("default", "Default (recommended)", "Opus 5.5"),
            model("haiku", "Haiku 4.5", ""),
        ])
    );
    assert_eq!(
        parse_models(r#"{"type":"helper","event":"error","message":"no"}"#),
        None
    );
    assert_eq!(parse_models(r#"{"type":"system","subtype":"init"}"#), None);
    assert_eq!(parse_models("not json"), None);
}
