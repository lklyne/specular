//! Port of `tests/unit/locator-kernel.test.ts`: the confidence policy behind
//! interaction sync (ADR 0030). Weakening it (dropping the identity-key
//! short-circuit, or lowering the floor or margin so ambiguous same-text
//! buttons resolve confident) fails these.

use specular_core::{
    LOCATOR_CONFIDENCE_FLOOR, LOCATOR_RUNNER_UP_MARGIN, LocatorBundle, LocatorCandidate,
    LocatorRect, LocatorResolution, dispatch_point, resolve_locator,
};

fn rect(x: f64, y: f64, width: f64, height: f64) -> LocatorRect {
    LocatorRect {
        x,
        y,
        width,
        height,
    }
}

fn s<T: From<String>>(value: &str) -> T {
    T::from(value.to_string())
}

fn bundle(f: impl FnOnce(&mut LocatorBundle)) -> LocatorBundle {
    let mut b = LocatorBundle {
        tag: "div".into(),
        offset_x: 0.5,
        offset_y: 0.5,
        ..LocatorBundle::default()
    };
    f(&mut b);
    b
}

fn candidate(f: impl FnOnce(&mut LocatorCandidate)) -> LocatorCandidate {
    let mut c = LocatorCandidate {
        rect: rect(0.0, 0.0, 10.0, 10.0),
        ..LocatorCandidate::default()
    };
    f(&mut c);
    c
}

/// Bounds far from the origin so the top-left proximity tiebreak contributes 0,
/// keeping scores exact for the margin and floor boundary cases below.
fn far() -> LocatorRect {
    rect(10_000.0, 10_000.0, 10.0, 10.0)
}

/// What a row expects: `Some(index)` for a confident winner, else the kind.
#[derive(Debug)]
enum Want {
    Winner(usize),
    Ambiguous,
    None,
}

#[test]
#[expect(clippy::too_many_lines, reason = "one table, one row per case")]
fn resolve_locator_policy() {
    let button = |text: &str, role: &str, tag: &str| {
        candidate(|c| {
            c.text = s(text);
            c.interactive = true;
            c.role = s(role);
            c.tag = s(tag);
            c.rect = far();
        })
    };
    let named = |name: &str| {
        candidate(|c| {
            c.name = s(name);
            c.interactive = true;
            c.rect = far();
        })
    };
    let same_text_button = candidate(|c| {
        c.text = s("Add to cart");
        c.interactive = true;
        c.tag = s("button");
    });

    let rows: Vec<(&str, LocatorBundle, Vec<LocatorCandidate>, Want)> = vec![
        (
            "exact id wins",
            bundle(|b| b.id = s("submit-btn")),
            vec![
                candidate(|c| {
                    c.id = s("submit-btn");
                    c.rect = rect(10.0, 20.0, 40.0, 20.0);
                }),
                candidate(|c| c.id = s("cancel-btn")),
            ],
            Want::Winner(0),
        ),
        (
            "unique test id wins even when text is shared",
            bundle(|b| {
                b.test_id = s("hero-cta");
                b.text = s("Buy now");
            }),
            vec![
                candidate(|c| {
                    c.test_id = s("footer-cta");
                    c.text = s("Buy now");
                    c.interactive = true;
                }),
                candidate(|c| {
                    c.test_id = s("hero-cta");
                    c.text = s("Buy now");
                    c.interactive = true;
                }),
            ],
            Want::Winner(1),
        ),
        (
            // The bundle's id and paths describe the old DOM; only the id still
            // exists. A decoy that keeps the old paths and name scores higher
            // structurally, but the identity key must win.
            "identity beats structural drift",
            bundle(|b| {
                b.id = s("go");
                b.name = s("Go");
                b.element_path = "main > form > button#go".into();
            }),
            vec![
                candidate(|c| {
                    c.name = s("Go");
                    c.text = s("Go");
                    c.element_path = s("main > form > button#go");
                    c.interactive = true;
                }),
                candidate(|c| {
                    c.id = s("go");
                    c.name = s("Go");
                    c.element_path = s("div > section > button");
                }),
            ],
            Want::Winner(1),
        ),
        (
            "restructured DOM with paths differing and identity holding",
            bundle(|b| {
                b.id = s("nav-home");
                b.element_path = "header > nav > a".into();
                b.full_path = "body > header > nav > a".into();
            }),
            vec![candidate(|c| {
                c.id = s("nav-home");
                c.element_path = s("div.wrapper > ul > li > a");
                c.full_path = s("body > div > ul > li > a");
            })],
            Want::Winner(0),
        ),
        (
            "duplicated identity key refuses rather than guessing",
            bundle(|b| b.id = s("dup")),
            vec![
                candidate(|c| c.id = s("dup")),
                candidate(|c| c.id = s("dup")),
            ],
            Want::Ambiguous,
        ),
        (
            "two same-text buttons are ambiguous",
            bundle(|b| {
                b.text = s("Add to cart");
                b.tag = "button".into();
            }),
            vec![same_text_button.clone(), same_text_button],
            Want::Ambiguous,
        ),
        (
            // Same text (+320) and interactive (+50) on both, far so proximity
            // is 0: a tie without role and tag. The bundle is a button; the
            // matching control agrees (+80 role, +40 tag) while the link twin
            // does not, clearing the runner-up margin.
            "role and tag agreement disambiguates two same-text controls",
            bundle(|b| {
                b.text = s("Add to cart");
                b.role = s("button");
                b.tag = "button".into();
            }),
            vec![
                button("Add to cart", "link", "a"),
                button("Add to cart", "button", "button"),
            ],
            Want::Winner(1),
        ),
        (
            "an absent element resolves to none",
            bundle(|b| {
                b.id = s("missing");
                b.name = s("Ghost button");
            }),
            vec![candidate(|c| {
                c.id = s("other");
                c.name = s("Something else");
                c.text = s("unrelated");
            })],
            Want::None,
        ),
        (
            "no candidates resolves to none",
            bundle(|b| b.id = s("anything")),
            vec![],
            Want::None,
        ),
        (
            // Exact name (+400) vs includes-name (+280); both interactive (+50)
            // and far (proximity 0): scores 450 vs 330, a gap of exactly
            // LOCATOR_RUNNER_UP_MARGIN.
            "confident when the top score clears the runner-up by exactly the margin",
            bundle(|b| b.name = s("Save")),
            vec![named("Save changes"), named("Save")],
            Want::Winner(1),
        ),
        (
            "ambiguous when two candidates tie under the margin",
            bundle(|b| b.name = s("Save")),
            vec![named("Save"), named("Save")],
            Want::Ambiguous,
        ),
        (
            // Text-includes (+200), non-interactive, far: 200 < the floor.
            "a lone match below the confidence floor is refused",
            bundle(|b| b.text = s("buy")),
            vec![candidate(|c| {
                c.text = s("buy now and save big");
                c.rect = far();
            })],
            Want::None,
        ),
        (
            // Text-exact (+320) + interactive (+50) = 370, at or above the
            // floor, with no runner-up.
            "a lone match above the confidence floor is accepted",
            bundle(|b| b.text = s("buy")),
            vec![button("buy", "", "")],
            Want::Winner(0),
        ),
    ];

    assert_eq!(LOCATOR_RUNNER_UP_MARGIN, 120.0);
    assert_eq!(LOCATOR_CONFIDENCE_FLOOR, 300.0);
    for (label, bundle, candidates, want) in rows {
        let got = resolve_locator(&bundle, &candidates);
        let ok = match (&got, &want) {
            (LocatorResolution::Confident { candidate, .. }, Want::Winner(i)) => candidate == i,
            (LocatorResolution::Ambiguous, Want::Ambiguous)
            | (LocatorResolution::None, Want::None) => true,
            _ => false,
        };
        assert!(ok, "{label}: got {got:?}, wanted {want:?}");
    }
}

#[test]
fn dispatch_point_maps_clamps_and_round_trips() {
    let target = rect(100.0, 200.0, 40.0, 20.0);
    let rows = [
        ((0.5, 0.5), (120.0, 210.0)),
        ((0.0, 0.0), (100.0, 200.0)),
        ((1.0, 1.0), (140.0, 220.0)),
        // Out-of-range fractions land on the rect's edge.
        ((1.5, -0.3), (140.0, 200.0)),
    ];
    for ((fx, fy), want) in rows {
        assert_eq!(dispatch_point(target, fx, fy), want, "offset ({fx}, {fy})");
    }

    // A confident resolution dispatches at the same point.
    let b = bundle(|b| {
        b.id = s("target");
        b.offset_x = 0.25;
        b.offset_y = 0.75;
    });
    let c = candidate(|c| {
        c.id = s("target");
        c.rect = target;
    });
    assert_eq!(
        resolve_locator(&b, &[c]),
        LocatorResolution::Confident {
            candidate: 0,
            point: (110.0, 215.0)
        }
    );
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
