//! The page popup's history row and address, and the page tool's preset
//! list, as models: which controls exist, their state and their actions.

use specular_doc::{Kind, Rect};
use specular_interact::{Action, PageNotice, Tool, ToolDefaultPatch};
use specular_testkit::insta::assert_snapshot;
use specular_testkit::{TestApp, page};

const PAGE: Rect = Rect::new(100.0, 100.0, 375.0, 667.0);

/// The lines of the popup that match `keep`, in order.
fn lines(app: &TestApp, keep: impl Fn(&str) -> bool) -> String {
    let all = app.popup_snapshot();
    all.lines()
        .filter(|line| keep(line.trim_start()))
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_loading_page_with_history_offers_back_and_stop() {
    let mut app = TestApp::with_entities([page("p", PAGE)]);
    app.select(&["p"]);
    app.page_reports("p", PageNotice::Url("https://example.com/live".to_owned()))
        .page_reports(
            "p",
            PageNotice::Loading {
                loading: true,
                can_go_back: true,
                can_go_forward: false,
            },
        );
    let row = lines(&app, |line| {
        [
            "button page.back",
            "button page.forward",
            "button page.reload",
            "field page.url",
        ]
        .iter()
        .any(|prefix| line.starts_with(prefix))
    });
    assert_snapshot!(row, @r#"
    button page.back "Back" icon=ChevronLeft chord=cmd+[ -> PageBack
    button page.forward "Forward" icon=ChevronRight chord=cmd+] disabled -> PageForward
    button page.reload "Stop loading" icon=Stop chord=cmd+. -> PageStop
    field page.url "Page address" value="https://example.com/live" placeholder="Type a URL" Wide submit=PageUrl
    "#);
}

#[test]
fn a_blank_page_has_an_empty_address_to_type_into() {
    let mut blank = page("p", PAGE);
    if let Kind::Page(inner) = &mut blank.kind {
        inner.url = "about:blank".to_owned();
    }
    let mut app = TestApp::with_entities([blank]);
    app.select(&["p"]);
    assert_snapshot!(lines(&app, |line| line.starts_with("field page.url")), @r#"
    field page.url "Page address" value="" placeholder="Type a URL" Wide submit=PageUrl
    "#);
}

#[test]
fn the_page_tool_marks_the_preset_in_use_and_then_custom() {
    let mut app = TestApp::empty();
    app.act(Action::SetToolDefault(ToolDefaultPatch::PagePreset(7)))
        .tool(Tool::AddPage);
    let marked = |app: &TestApp| lines(app, |line| line.starts_with("option [x]"));
    assert_snapshot!(marked(&app), @r#"
    option [x] page.preset.7 "Add Desktop" text="Desktop" trailing="1440×900" -> SetToolDefault(PagePreset(7))
    "#);
    app.act(Action::SetToolDefault(ToolDefaultPatch::PageCustom));
    assert_snapshot!(marked(&app), @r#"
    option [x] page.preset.custom "Add custom" text="Custom" -> SetToolDefault(PageCustom)
    "#);
}
