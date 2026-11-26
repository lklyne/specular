//! The page bridge goes to a file inside the space folder and to nothing
//! else (ADR 0055, "Trust").

use std::path::Path;

use specular_interact::bridge_allowed;

#[test]
fn only_a_file_inside_the_space_folder_gets_the_bridge() {
    let root = Path::new("/Users/me/space");
    let rows: [(&str, bool); 15] = [
        ("file:///Users/me/space/widget.html", true),
        ("file:///Users/me/space/widgets/clock.html?v=2#top", true),
        ("file:///Users/me/space/my%20widget.html", true),
        ("local-file:///Users/me/space/widget.html?t=1", true),
        ("file:///Users/me/space/./widgets/../widget.html", true),
        ("file:///Users/me/space/../secrets.html", false),
        ("file:///Users/me/space/%2E%2E/secrets.html", false),
        ("file:///Users/me/space", false),
        ("file:///Users/me/space-other/widget.html", false),
        ("file:///Users/me/elsewhere/widget.html", false),
        ("file://localhost/Users/me/space/widget.html", false),
        ("http://localhost/Users/me/space/widget.html", false),
        ("https://example.com/", false),
        ("about:blank", false),
        ("data:text/html,<script>specular</script>", false),
    ];
    for (url, allowed) in rows {
        assert_eq!(bridge_allowed(url, root), allowed, "{url}");
    }
    assert!(
        !bridge_allowed("file:///anything.html", Path::new("relative/space")),
        "a space folder that is not absolute holds nothing"
    );
}
