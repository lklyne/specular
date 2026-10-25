//! What a paste is, decided from what the clipboard holds. What a paste
//! then makes is in `fixtures/scenarios/clipboard-items.txt`.

use specular_interact::{AssetBytes, ClipboardContent, ClipboardImage, Paste};

fn text(text: &str) -> ClipboardContent {
    ClipboardContent {
        text: Some(text.to_owned()),
        image: None,
    }
}

fn image() -> ClipboardImage {
    ClipboardImage {
        width: 640,
        height: 480,
        png: AssetBytes::from(vec![0x89, b'P', b'N', b'G']),
    }
}

#[test]
fn a_paste_decides_items_then_image_then_url_then_text() {
    let items = concat!(
        r#"specular:canvas:{"nodes":[{"id":"c","type":"shape","x":0,"y":300,"#,
        r#""width":100,"height":100,"shapeKind":"rectangle","text":""}],"edges":[]}"#
    )
    .to_owned();
    let both = ClipboardContent {
        text: Some(items),
        image: Some(image()),
    };
    assert!(matches!(Paste::of(both), Paste::Items(_)));
    let image_and_url = ClipboardContent {
        text: Some("example.com".to_owned()),
        image: Some(image()),
    };
    assert_eq!(Paste::of(image_and_url), Paste::Image(image()));
    assert_eq!(
        Paste::of(text("localhost:4321/garden")),
        Paste::Page("http://localhost:4321/garden".to_owned())
    );
    // A URL on one of several lines is prose.
    assert_eq!(
        Paste::of(text("see\nexample.com")),
        Paste::Text("see\nexample.com".to_owned())
    );
    // The prefix with something else after it is text like any other.
    assert_eq!(
        Paste::of(text("specular:canvas:nope")),
        Paste::Text("specular:canvas:nope".to_owned())
    );
    // Copied canvas data with nothing in it is text too.
    assert_eq!(
        Paste::of(text("specular:canvas:{}")),
        Paste::Text("specular:canvas:{}".to_owned())
    );
    assert_eq!(Paste::of(ClipboardContent::default()), Paste::Nothing);
}
