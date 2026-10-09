//! A control activated by its name from the models, with nothing laid out,
//! does what a click on it in the built-in layout does.

#![expect(
    clippy::expect_used,
    reason = "a helper fails the test it is called from"
)]

use specular_doc::Rect;
use specular_interact::{Key, Tool, dock, sidebar, toolbar};
use specular_testkit::{
    CMD, SHIFT, TestApp, doc_snapshot, document, group, inside, page, shape, sticky,
};

const A: Rect = Rect::new(300.0, 300.0, 200.0, 100.0);
const B: Rect = Rect::new(600.0, 340.0, 200.0, 100.0);
const C: Rect = Rect::new(900.0, 300.0, 200.0, 100.0);

/// How a step reaches a control.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Via {
    /// A click at the middle of where the built-in panels lay it out.
    Layout,
    /// By name, from the models alone.
    Models,
}

struct Hands {
    app: TestApp,
    via: Via,
}

impl Hands {
    fn new(app: TestApp, via: Via) -> Self {
        let mut hands = Self { app, via };
        if via == Via::Layout {
            hands.app.with_panels();
        }
        hands
    }

    fn click(&mut self, name: &str) -> &mut Self {
        match self.via {
            Via::Layout => {
                self.app.click_control(name);
            }
            Via::Models => {
                self.app.control(name).expect("the models name it");
            }
        }
        self
    }

    fn right_click(&mut self, at: (f32, f32)) -> &mut Self {
        match self.via {
            Via::Layout => self.app.right_click(at),
            Via::Models => self.app.context_menu(at),
        };
        self
    }

    /// What the two apps must agree on: the document, and everything the
    /// session shows through the models and keeps for the next key.
    fn seen(&self) -> Vec<(&'static str, String)> {
        let (app, session) = (self.app.app(), self.app.session());
        vec![
            ("document", doc_snapshot(self.app.document())),
            ("canvases", format!("{:?}", self.app.canvas_names())),
            ("active", self.app.active_canvas().to_owned()),
            ("selection", format!("{:?}", self.app.selected_ids())),
            ("tool", format!("{:?}", session.tool)),
            ("zoom", format!("{:?}", session.camera.zoom)),
            ("editing", format!("{:?}", self.app.field_edit())),
            ("open list", format!("{:?}", session.panel.open)),
            ("open menu", format!("{:?}", session.panel.menu)),
            ("dock", format!("{:#?}", dock(app))),
            ("toolbar", format!("{:#?}", toolbar(app))),
            ("sidebar", format!("{:#?}", sidebar(app))),
        ]
    }
}

fn shapes() -> TestApp {
    TestApp::with_entities([shape("a", A), shape("b", B), shape("c", C)])
}

fn one_sticky() -> TestApp {
    TestApp::with_entities([sticky("s", A, "note")])
}

fn one_page() -> TestApp {
    TestApp::with_entities([page("p", Rect::new(300.0, 300.0, 400.0, 300.0))])
}

fn grouped() -> TestApp {
    TestApp::with_space([
        (
            "Home",
            document([
                group("g", Rect::new(280.0, 280.0, 540.0, 180.0)),
                inside("g", shape("a", A)),
                inside("g", shape("b", B)),
                shape("c", C),
            ]),
        ),
        ("Notes", document([shape("n", A)])),
    ])
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "a table with a case for each kind of control"
)]
fn a_control_named_from_the_models_does_what_a_click_on_it_in_the_layout_does() {
    type Case = (&'static str, fn() -> TestApp, fn(&mut Hands), bool);
    let cases: [Case; 12] = [
        (
            "a tool button arms its tool",
            shapes,
            |hands| {
                hands.click("tool.shape");
            },
            false,
        ),
        (
            "a list of the toolbar puts the tool down, and its option closes it",
            shapes,
            |hands| {
                hands.app.tool(Tool::AddSticky);
                hands.click("zoom").click("zoom.50");
            },
            false,
        ),
        (
            "a swatch in a list, which Escape then closes before anything else",
            one_sticky,
            |hands| {
                hands.app.select(&["s"]);
                hands.click("text.color").click("text.color.swatches.green");
                hands.app.key(Key::Escape);
            },
            true,
        ),
        (
            "an option, a stepper's half, and a control outside the open list",
            one_sticky,
            |hands| {
                hands.app.select(&["s"]);
                hands.click("text.size").click("text.size.32");
                hands.click("text.size").click("text.size.custom.inc");
                hands.click("text.color");
            },
            true,
        ),
        (
            "a toggle",
            one_sticky,
            |hands| {
                hands
                    .app
                    .double_click((400.0, 350.0))
                    .chord(CMD, Key::Char('a'));
                hands.click("format.bold");
                hands.app.key(Key::Escape);
            },
            true,
        ),
        (
            "a button",
            shapes,
            |hands| {
                hands.app.select(&["a", "b", "c"]);
                hands.click("item.arrange.row");
            },
            true,
        ),
        (
            "a field takes the keys, and Enter submits what was typed",
            one_page,
            |hands| {
                hands.app.select(&["p"]);
                hands.click("page.url");
                (hands.app.chord(CMD, Key::Char('a')))
                    .type_text("example.org/docs")
                    .key(Key::Enter);
            },
            true,
        ),
        (
            "a field being typed in is kept by a click on another control",
            one_page,
            |hands| {
                hands.app.select(&["p"]);
                hands.click("page.url");
                (hands.app.chord(CMD, Key::Char('a'))).type_text("example.org/docs");
                hands.click("page.reload");
            },
            true,
        ),
        (
            "the sidebar's add button and a head",
            grouped,
            |hands| {
                hands.app.show_sidebar(true);
                hands.click("sidebar.add");
                hands.click("sidebar.head.notes");
            },
            false,
        ),
        (
            "a canvas row, a row's fold, and rows picked with keys held",
            grouped,
            |hands| {
                hands.app.show_sidebar(true);
                hands
                    .click("sidebar.canvas.tab_2")
                    .click("sidebar.canvas.tab_1");
                hands
                    .click("sidebar.notes.g.toggle")
                    .click("sidebar.notes.a");
                hands.app.hold(SHIFT);
                hands.click("sidebar.notes.c");
                hands.app.let_go();
            },
            false,
        ),
        (
            "an item of the menu a right press opens on what it selects",
            shapes,
            |hands| {
                hands.app.select(&["b"]);
                hands.right_click((350.0, 350.0)).click("menu.duplicate");
            },
            true,
        ),
        (
            "a menu left open is closed by a control outside it, which does nothing",
            shapes,
            |hands| {
                hands.right_click((350.0, 350.0)).click("tool.shape");
            },
            false,
        ),
    ];
    for (name, start, steps, undoable) in cases {
        let mut laid_out = Hands::new(start(), Via::Layout);
        let mut named = Hands::new(start(), Via::Models);
        let before = named.seen();
        steps(&mut laid_out);
        steps(&mut named);
        for ((part, named), (_, laid_out)) in named.seen().into_iter().zip(laid_out.seen()) {
            assert!(
                named == laid_out,
                "{name}: the {part} by name is\n{named}\nand through the layout\n{laid_out}"
            );
        }
        assert_ne!(named.seen(), before, "{name}: the steps did something");
        if undoable {
            named.app.assert_undo_returns_to_start();
        }
    }
}

#[test]
fn a_name_no_control_has_is_an_error_that_lists_the_names_there_are() {
    let mut app = one_sticky();
    app.select(&["s"]);
    let error = app
        .control("text.colour")
        .map(|_| ())
        .expect_err("no such control");
    assert_eq!(error.name, "text.colour");
    for name in [
        "tool.select",
        "zoom.50",
        "text.color",
        "text.color.swatches.green",
        "menu.duplicate",
    ] {
        assert!(error.shown.iter().any(|shown| shown == name), "{name}");
    }
    assert!(
        !error
            .shown
            .iter()
            .any(|shown| shown.starts_with("sidebar.canvas")),
        "the sidebar is hidden"
    );
    assert!(
        error.to_string().starts_with(
            "no control `text.colour` is shown; these are: view.canvas tool.select tool.draw"
        ),
        "{error}"
    );
}
