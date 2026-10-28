//! Two apps on one canvas through a real live room: a server run with
//! `wrangler dev` and an edit link it made. Skipped unless
//! `SPECULAR_LIVE_LINK` holds that link:
//!
//! ```sh
//! SPECULAR_LIVE_LINK='http://localhost:8787/c/<doc>#t=<edit token>' \
//!   cargo test -p specular-shell --lib through_a_room -- --ignored
//! ```

use std::time::{Duration, Instant};

use specular_doc::Rect;
use specular_interact::{Effect, Event, ShareEvent};
use specular_testkit::{TestApp, document, sticky};

use super::{Incoming, LiveLink, ShareLink};

/// An app and its connection, wired as the runtime wires them.
struct Peer {
    app: TestApp,
    live: LiveLink,
}

impl Peer {
    fn connect(app: TestApp, link: &str) -> Self {
        let live = LiveLink::start(ShareLink::parse(link).expect("a link")).expect("a thread");
        Self { app, live }
    }

    /// Hands the app's updates to the room and the room's to the app, for
    /// `time`.
    fn pump(&mut self, time: Duration) {
        let until = Instant::now() + time;
        while Instant::now() < until {
            for effect in self.app.take_effects() {
                if let Effect::ShareSend { update, .. } = effect {
                    self.live.send(update);
                }
            }
            while let Some(incoming) = self.live.take() {
                let canvas = self.app.app().space().active().id.clone();
                let event = match incoming {
                    Incoming::State(state) => ShareEvent::Connected { canvas, state },
                    Incoming::Update(update) => ShareEvent::Receive { canvas, update },
                    Incoming::Refused(reason) => panic!("refused: {reason}"),
                };
                self.app.send(Event::Share(event));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

#[test]
#[ignore = "needs a live room: SPECULAR_LIVE_LINK"]
fn two_apps_edit_one_canvas_through_a_room() {
    let Ok(link) = std::env::var("SPECULAR_LIVE_LINK") else {
        return;
    };
    let second = Duration::from_secs(1);
    let mut ann = Peer::connect(
        TestApp::from_document(document([
            sticky("s1", Rect::new(0.0, 0.0, 200.0, 100.0), "one"),
            sticky("s2", Rect::new(400.0, 0.0, 200.0, 100.0), "two"),
        ])),
        &link,
    );
    ann.pump(2 * second);
    assert!(ann.app.app().is_shared(), "the first app shares its canvas");

    let mut ben = Peer::connect(TestApp::empty(), &link);
    ben.pump(2 * second);
    assert!(ben.app.app().is_shared());
    assert_eq!(ben.app.document().order(), ann.app.document().order());

    ann.app.drag((100.0, 50.0), (100.0, 450.0));
    ben.app.drag((500.0, 50.0), (500.0, 450.0));
    for _ in 0..3 {
        ann.pump(second / 2);
        ben.pump(second / 2);
    }
    assert_eq!(ann.app.document(), ben.app.document());
    assert_ne!(ben.app.rect("s1"), Rect::new(0.0, 0.0, 200.0, 100.0));
}
