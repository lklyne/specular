use super::*;
use crate::geometry::PixelSize;

fn frames(events: &[PageEvent]) -> Vec<&FrameEvent> {
    events
        .iter()
        .filter_map(|event| match event {
            PageEvent::Frame(frame) => Some(frame),
            _ => None,
        })
        .collect()
}

fn source_with_page(scale: f32) -> (SyntheticPageSource, PageId) {
    let mut source = SyntheticPageSource::new();
    let mut spec = PageSpec::new("https://example.com/", CssSize::new(40, 20));
    spec.texture_scale = scale;
    let id = source.create_page(&spec).unwrap();
    (source, id)
}

#[test]
fn pump_paints_frame_at_texture_scaled_size() {
    let (mut source, _) = source_with_page(0.5);
    source.pump_at(Instant::now());
    let mut events = Vec::new();
    source.drain_events(&mut events);
    assert_eq!(frames(&events)[0].frame.size(), PixelSize::new(20, 10));

    {
        let (mut source, _) = source_with_page(1.0);
        let start = Instant::now();
        source.pump_at(start);
        source.pump_at(start + Duration::from_millis(1));
        let mut events = Vec::new();
        source.drain_events(&mut events);
        assert_eq!(frames(&events).len(), 1);
    }

    {
        let (mut source, id) = source_with_page(1.0);
        source.set_painting(id, false).unwrap();
        source.pump_at(Instant::now());
        let mut events = Vec::new();
        source.drain_events(&mut events);
        assert!(frames(&events).is_empty());
    }
}

#[test]
fn closed_page_reports_unknown_page() {
    let (mut source, id) = source_with_page(1.0);
    source.close_page(id).unwrap();
    assert!(matches!(
        source.set_frame_rate(id, 30),
        Err(PageSourceError::UnknownPage(_))
    ));

    {
        let mut source = SyntheticPageSource::new();
        let spec = PageSpec::new("https://example.com/", CssSize::new(0, 10));
        assert!(matches!(
            source.create_page(&spec),
            Err(PageSourceError::InvalidSpec(_))
        ));
    }
}

#[test]
fn element_at_is_the_grid_cell_holding_the_point() {
    let element = synthetic_element_at(CssSize::new(400, 300), Vec2::new(170.0, 100.0));
    assert_eq!(
        element,
        Some(PageElement {
            selector: "div.cell[data-col=\"1\"][data-row=\"2\"]".to_owned(),
            element_path: Some("body > div.cell".to_owned()),
            bounding_box: PixelRect::new(160, 96, 160, 48),
        })
    );

    {
        let element = synthetic_element_at(CssSize::new(400, 300), Vec2::new(399.0, 299.0));
        assert_eq!(
            element.map(|element| element.bounding_box),
            Some(PixelRect::new(320, 288, 80, 12))
        );
    }

    {
        let viewport = CssSize::new(400, 300);
        for point in [
            Vec2::new(-1.0, 10.0),
            Vec2::new(400.0, 10.0),
            Vec2::new(10.0, 300.0),
        ] {
            assert_eq!(synthetic_element_at(viewport, point), None, "{point}");
        }
    }
}

fn drained(source: &mut SyntheticPageSource) -> Vec<PageEvent> {
    let mut events = Vec::new();
    source.drain_events(&mut events);
    events
}

/// The loading flags a page last reported: `(loading, back, forward)`.
fn loading(events: &[PageEvent]) -> Option<(bool, bool, bool)> {
    events.iter().rev().find_map(|event| match event {
        PageEvent::Loading {
            loading,
            can_go_back,
            can_go_forward,
            ..
        } => Some((*loading, *can_go_back, *can_go_forward)),
        _ => None,
    })
}

fn urls(events: &[PageEvent]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|event| match event {
            PageEvent::Url { url, .. } => Some(url.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_hosted_page_answers_an_element_question_and_a_closed_one_refuses() {
    let (mut source, id) = source_with_page(1.0);
    drained(&mut source);
    source.query_element(id, Vec2::new(10.0, 10.0), 7).unwrap();
    let events = drained(&mut source);
    assert!(matches!(
        events.as_slice(),
        [PageEvent::ElementAt { request: 7, element: Some(element), .. }]
            if element.bounding_box == PixelRect::new(0, 0, 40, 48)
    ));
    source.close_page(id).unwrap();
    assert!(source.query_element(id, Vec2::ZERO, 8).is_err());
}

#[test]
fn a_new_page_reports_its_title_address_and_an_ended_load() {
    let (mut source, _) = source_with_page(1.0);
    let events = drained(&mut source);
    assert_eq!(urls(&events), ["https://example.com/"]);
    assert!(events.iter().any(|event| matches!(
        event,
        PageEvent::Title { title, .. } if title == "Synthetic example.com"
    )));
    assert_eq!(loading(&events), Some((false, false, false)));
}

#[test]
fn navigation_walks_a_history_and_a_new_address_cuts_what_was_ahead() {
    let (mut source, id) = source_with_page(1.0);
    let to = |url: &str| PageNav::To(url.to_owned());
    source.navigate(id, &to("https://a.test/")).unwrap();
    source.navigate(id, &to("https://b.test/")).unwrap();
    drained(&mut source);

    source.navigate(id, &PageNav::Back).unwrap();
    let events = drained(&mut source);
    assert_eq!(urls(&events), ["https://a.test/"]);
    assert_eq!(loading(&events), Some((false, true, true)));

    source.navigate(id, &to("https://c.test/")).unwrap();
    assert_eq!(loading(&drained(&mut source)), Some((false, true, false)));
    source.navigate(id, &PageNav::Forward).unwrap();
    assert!(drained(&mut source).is_empty());
}

#[test]
fn a_wheel_scrolls_the_document_and_stops_at_its_ends() {
    let (mut source, id) = source_with_page(1.0);
    drained(&mut source);
    let wheel = |dy: f32| {
        InputEvent::Wheel(crate::input::WheelEvent {
            position: Vec2::ZERO,
            delta: Vec2::new(0.0, dy),
            modifiers: crate::input::Modifiers::default(),
        })
    };
    source.send_input(id, &wheel(-30.0)).unwrap();
    source.send_input(id, &wheel(-30.0)).unwrap();
    source.send_input(id, &wheel(100.0)).unwrap();
    source.send_input(id, &wheel(100.0)).unwrap();
    let offsets: Vec<f32> = drained(&mut source)
        .iter()
        .filter_map(|event| match event {
            PageEvent::Scrolled { offset, .. } => Some(offset.y),
            _ => None,
        })
        .collect();
    assert_eq!(offsets, [30.0, 40.0, 0.0]);
}

#[test]
fn the_grid_moves_with_the_scroll_and_a_region_grabs_whole_cells() {
    let viewport = CssSize::new(400, 300);
    let scroll = Vec2::new(0.0, 24.0);
    let element = element_scrolled(viewport, scroll, Vec2::new(10.0, 30.0)).unwrap();
    assert_eq!(element.bounding_box, PixelRect::new(0, 24, 160, 48));
    let grabbed = |rect| synthetic_elements_in(viewport, Vec2::ZERO, rect);
    assert_eq!(grabbed(CssRect::new(0.0, 0.0, 330.0, 100.0)), 4);
    assert_eq!(grabbed(CssRect::new(10.0, 10.0, 150.0, 40.0)), 0);
}
