//! Port of `tests/unit/locator-kernel.test.ts`: the confidence policy behind
//! interaction sync (ADR 0030). Weakening it (dropping the identity-key
//! short-circuit, or lowering the floor or margin so ambiguous same-text
//! buttons resolve confident) fails these.

use specular_core::{LocatorBundle, LocatorCandidate, LocatorRect};

fn rect(x: f64, y: f64, width: f64, height: f64) -> LocatorRect {
    LocatorRect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn wire_json_reads_with_omitted_or_null_fields() {
    let b: LocatorBundle = serde_json::from_str(
        r#"{"testId":"x","tag":null,"elementPath":"a > b","fullPath":"body > a > b","offsetX":0.25,"offsetY":0.75,"name":null}"#,
    )
    .unwrap();
    assert_eq!(b.test_id.as_deref(), Some("x"));
    assert_eq!((b.tag.as_str(), b.name), ("", None));
    assert_eq!(b.element_path, "a > b");

    let c: LocatorCandidate = serde_json::from_str(
        r#"{"elementPath":"a","fullPath":null,"interactive":true,"rect":{"x":1,"y":2,"width":3,"height":4}}"#,
    )
    .unwrap();
    assert_eq!(c.element_path.as_deref(), Some("a"));
    assert!(c.interactive && c.full_path.is_none());
    assert_eq!(c.rect, rect(1.0, 2.0, 3.0, 4.0));
}
