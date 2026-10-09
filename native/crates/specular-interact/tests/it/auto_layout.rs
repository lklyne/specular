//! Auto-layout groups (ADR 0015): a managed group packs its members as a
//! row or a column, and whatever changes them, their order or the group's
//! fields lays them out again in the same undo step.

use specular_doc::{Entity, EntityId, Group, Kind, LayoutMode, Rect};
use specular_interact::{Action, Key, LayoutAxis};
use specular_testkit::{CMD_SHIFT, TestApp, group, inside, shape};

/// The group `g` as a managed row with `gap`.
fn managed(rect: Rect, mode: LayoutMode, gap: Option<f64>) -> Entity {
    Entity {
        kind: Kind::Group(Group {
            managed_layout: Some(true),
            layout_mode: Some(mode),
            layout_gap: gap,
            ..Group::default()
        }),
        ..group("g", rect)
    }
}

/// A row of `a`, `b` and `c`, 100 wide and 40 apart, in the group `g`,
/// with `d` loose below it.
fn row() -> TestApp {
    let mut app = TestApp::with_entities([
        inside("g", shape("a", Rect::new(100.0, 100.0, 100.0, 100.0))),
        inside("g", shape("b", Rect::new(240.0, 100.0, 100.0, 100.0))),
        inside("g", shape("c", Rect::new(380.0, 100.0, 100.0, 100.0))),
        managed(
            Rect::new(76.0, 76.0, 428.0, 148.0),
            LayoutMode::Row,
            Some(40.0),
        ),
        shape("d", Rect::new(100.0, 500.0, 60.0, 60.0)),
    ]);
    app.viewport((1600.0, 1000.0));
    app
}

fn layout(app: &TestApp) -> (Option<bool>, Option<LayoutMode>, Option<f64>) {
    match &app.entity("g").kind {
        Kind::Group(group) => (group.managed_layout, group.layout_mode, group.layout_gap),
        _ => (None, None, None),
    }
}

/// The left edge of each of `ids`.
fn lefts(app: &TestApp, ids: &[&str]) -> Vec<f64> {
    ids.iter().map(|id| app.rect(id).x).collect()
}

fn tops(app: &TestApp, ids: &[&str]) -> Vec<f64> {
    ids.iter().map(|id| app.rect(id).y).collect()
}

#[test]
fn making_auto_layout_wraps_the_selection_and_packs_it_in_the_order_it_reads() {
    // Spread along x, out of stack order, uneven and not level.
    let mut app = TestApp::with_entities([
        shape("c", Rect::new(520.0, 130.0, 100.0, 100.0)),
        shape("a", Rect::new(103.0, 100.0, 100.0, 100.0)),
        shape("b", Rect::new(260.0, 110.0, 100.0, 100.0)),
    ]);
    app.select(&["a", "b", "c"])
        .chord(CMD_SHIFT, Key::Char('a'));
    let made = app.selected().map(str::to_owned).unwrap_or_default();
    let Kind::Group(group) = &app.entity(&made).kind else {
        panic!("the selection is the new group");
    };
    assert_eq!(
        (group.managed_layout, group.layout_mode, group.layout_gap),
        (Some(true), Some(LayoutMode::Row), None)
    );
    assert_eq!(app.entity(&made).label.as_deref(), Some("Auto-layout"));
    // Packed 80 apart from the least corner, on the grid, all level with it.
    assert_eq!(lefts(&app, &["a", "b", "c"]), [100.0, 280.0, 460.0]);
    assert_eq!(tops(&app, &["a", "b", "c"]), [100.0, 100.0, 100.0]);
    assert_eq!(app.rect(&made), Rect::new(76.0, 76.0, 508.0, 148.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_lone_group_is_managed_along_the_axis_its_members_are_spread() {
    let mut app = TestApp::with_entities([
        inside("g", shape("low", Rect::new(100.0, 400.0, 100.0, 100.0))),
        inside("g", shape("high", Rect::new(120.0, 100.0, 100.0, 100.0))),
        group("g", Rect::new(76.0, 76.0, 168.0, 448.0)),
    ]);
    app.select(&["g"]).act(Action::AutoLayout);
    assert_eq!(layout(&app).1, Some(LayoutMode::Column));
    assert_eq!(app.rect("high"), Rect::new(100.0, 100.0, 100.0, 100.0));
    assert_eq!(app.rect("low"), Rect::new(100.0, 280.0, 100.0, 100.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn whatever_changes_a_managed_row_lays_it_out_again_in_the_same_step() {
    type Change = fn(&mut TestApp);
    // (what is done, the lefts of a, b and c after, the group's rect).
    let rows: [(&str, Change, [f64; 3], Rect); 6] = [
        (
            "the gap is set",
            |app| {
                app.select(&["g"]).act(Action::GroupGap(20.0));
            },
            [100.0, 220.0, 340.0],
            Rect::new(76.0, 76.0, 388.0, 148.0),
        ),
        (
            "a member is resized by its right handle",
            |app| {
                app.select(&["a"]);
                app.press((200.0, 150.0)).drag_to((260.0, 150.0)).release();
            },
            [100.0, 300.0, 440.0],
            Rect::new(76.0, 76.0, 488.0, 148.0),
        ),
        (
            "a member is deleted",
            |app| {
                app.select(&["a"]).key(Key::Delete);
            },
            [f64::NAN, 240.0, 380.0],
            Rect::new(216.0, 76.0, 288.0, 148.0),
        ),
        (
            "a member is sent to the back, so it leads",
            |app| {
                app.select(&["c"]).act(Action::SendToBack);
            },
            [240.0, 380.0, 100.0],
            Rect::new(76.0, 76.0, 428.0, 148.0),
        ),
        (
            "it is turned into a column",
            |app| {
                app.select(&["g"])
                    .act(Action::GroupLayout(Some(LayoutAxis::Y)));
            },
            [100.0, 100.0, 100.0],
            Rect::new(76.0, 76.0, 148.0, 428.0),
        ),
        (
            "an item is dropped into it",
            |app| {
                app.press((130.0, 530.0)).drag_to((270.0, 170.0)).release();
            },
            [100.0, 240.0, 380.0],
            Rect::new(76.0, 76.0, 528.0, 148.0),
        ),
    ];
    for (name, change, want, group) in rows {
        let mut app = row();
        change(&mut app);
        let found: Vec<f64> = ["a", "b", "c"]
            .iter()
            .zip(want)
            .filter(|(_, want)| !want.is_nan())
            .map(|(id, _)| app.rect(id).x)
            .collect();
        let want: Vec<f64> = want.into_iter().filter(|x| !x.is_nan()).collect();
        assert_eq!(found, want, "{name}");
        assert_eq!(app.rect("g"), group, "{name}");
        app.assert_undo_returns_to_start();
    }
}

#[test]
fn an_item_dropped_into_a_row_takes_the_slot_its_stack_position_gives() {
    // The layout sequence is the group's run of the stack (ADR 0015 D2): an
    // item in front of everything joins last, one behind everything first.
    let slots = [(None, 520.0), (Some(Action::SendToBack), 100.0)];
    for (send, want_x) in slots {
        let mut app = row();
        if let Some(send) = send {
            app.select(&["d"]).act(send);
        }
        app.press((130.0, 530.0)).drag_to((270.0, 170.0)).release();
        assert_eq!(app.rect("d"), Rect::new(want_x, 100.0, 60.0, 60.0));
        app.assert_undo_returns_to_start();
    }
}

#[test]
fn dragging_a_members_body_moves_the_whole_group() {
    let mut app = row();
    app.select(&["b"]);
    // Off the dot at its middle, which would reorder it.
    app.press((310.0, 120.0)).drag_to((350.0, 220.0)).release();
    assert_eq!(lefts(&app, &["a", "b", "c"]), [140.0, 280.0, 420.0]);
    assert_eq!(tops(&app, &["a", "b", "c"]), [200.0, 200.0, 200.0]);
    assert_eq!(app.rect("g"), Rect::new(116.0, 176.0, 428.0, 148.0));
    assert_eq!(
        app.entity("b").parent.as_ref().map(EntityId::as_str),
        Some("g")
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn turning_the_layout_off_leaves_the_members_where_they_are() {
    let mut app = row();
    app.select(&["g"]).act(Action::GroupLayout(None));
    assert_eq!(
        layout(&app),
        (Some(false), Some(LayoutMode::Freeform), Some(40.0))
    );
    assert_eq!(lefts(&app, &["a", "b", "c"]), [100.0, 240.0, 380.0]);
    // Now a member moves freely and nothing follows it.
    app.select(&["a"]);
    app.press((150.0, 150.0)).drag_to((150.0, 250.0)).release();
    assert_eq!(app.rect("a").y, 200.0);
    assert_eq!(lefts(&app, &["b", "c"]), [240.0, 380.0]);
}

#[test]
fn a_group_inside_a_row_travels_with_everything_in_it() {
    let mut app = TestApp::with_entities([
        inside("g", shape("a", Rect::new(100.0, 100.0, 100.0, 100.0))),
        inside("inner", shape("m", Rect::new(424.0, 124.0, 50.0, 50.0))),
        inside("g", group("inner", Rect::new(400.0, 100.0, 98.0, 98.0))),
        managed(
            Rect::new(76.0, 76.0, 446.0, 148.0),
            LayoutMode::Row,
            Some(40.0),
        ),
    ]);
    app.select(&["g"]).act(Action::GroupGap(0.0));
    assert_eq!(app.rect("inner"), Rect::new(200.0, 100.0, 98.0, 98.0));
    assert_eq!(app.rect("m"), Rect::new(224.0, 124.0, 50.0, 50.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn the_group_dock_turns_the_layout_round_and_steps_its_gap() {
    let mut app = row();
    app.with_panels();
    app.select(&["g"]);
    // The direction that is on turns the layout off; the other turns it
    // round. The gap steps by a grid square while there is a layout.
    app.click_control("group.gap.inc");
    assert_eq!(layout(&app).2, Some(60.0));
    assert_eq!(lefts(&app, &["a", "b", "c"]), [100.0, 260.0, 420.0]);
    app.click_control("group.gap.dec");
    assert_eq!(layout(&app).2, Some(40.0));
    assert_eq!(lefts(&app, &["a", "b", "c"]), [100.0, 240.0, 380.0]);
    app.click_control("group.gap.inc");
    app.click_control("group.layout.column");
    assert_eq!(layout(&app).1, Some(LayoutMode::Column));
    assert_eq!(tops(&app, &["a", "b", "c"]), [100.0, 260.0, 420.0]);
    app.click_control("group.layout.column");
    assert_eq!(layout(&app).0, Some(false));
    let shown: Vec<String> = (app.panel_layout().controls())
        .map(ToString::to_string)
        .collect();
    assert!(
        !shown.iter().any(|id| id.starts_with("group.gap")),
        "no gap without a layout: {shown:?}"
    );
    app.undo().undo().undo().undo().undo();
    assert_eq!(lefts(&app, &["a", "b", "c"]), [100.0, 240.0, 380.0]);
    assert!(!app.app().can_undo());
}
