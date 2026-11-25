//! [`Pages`]: the page source behind the shell, whichever it is, and what
//! it reported that the app and the compositor have not yet taken.
//!
//! The source sits in an `Rc<RefCell<..>>` because two things reach it on
//! the main thread: the shell, inside Rune's handlers, and (with CEF in a
//! window) a run-loop observer between them, which takes a page's frame as
//! soon as CEF delivers it and wakes Rune (see [`cef`](crate::cef)).

use std::cell::{RefCell, RefMut};
use std::rc::Rc;
use std::time::Instant;

use specular_core::{PageEvent, PageFrame, PageId, PageSource, SyntheticPageSource};
use specular_interact::PageNotice;

/// Which page source hosts the pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceKind {
    /// `SyntheticPageSource`: animated CPU frames, no browser.
    #[default]
    Synthetic,
    /// Windowless CEF browsers (`--features cef`, in a bundle).
    Cef,
}

/// Who turns CEF's message loop. Only CEF reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PumpKind {
    /// A timer on the main run loop, as a window does. A headless run
    /// spins that run loop briefly in each poll, standing in for a window's.
    #[default]
    Timer,
    /// `PageSource::pump` in each poll, as the real shell's headless run does.
    Caller,
}

/// The shared source and the events taken from it between polls.
pub type SharedSource = Rc<RefCell<Box<dyn PageSource>>>;
/// Events a run-loop observer took from the source, for the next poll.
pub type Inbox = Rc<RefCell<Vec<PageEvent>>>;

/// How page frames arrived, for the line printed after a run.
#[derive(Debug, Default, Clone, Copy)]
pub struct FrameTally {
    /// `PageFrame::GpuShared`: an IOSurface, imported zero-copy.
    pub shared: u64,
    /// `PageFrame::Cpu`: bytes, uploaded.
    pub cpu: u64,
    /// The sum of paint-to-poll delays, in microseconds, over `shared + cpu`.
    pub delay_us: u128,
    /// The longest paint-to-poll delay, in microseconds.
    pub max_delay_us: u128,
}

/// The page source and its queue.
pub struct Pages {
    source: SharedSource,
    inbox: Inbox,
    /// Frames taken so far, by kind.
    pub tally: FrameTally,
    /// Whether the shell turns the source's loop (`PageSource::pump`) or a
    /// run-loop timer does.
    pub pump: PumpKind,
    /// The source's name, as it gives it.
    pub name: &'static str,
    /// A headless run with a run-loop timer: each poll spins the main run
    /// loop, standing in for a window's.
    pub spin: bool,
    /// The source has shut down from a running loop.
    stopped: bool,
}

impl Pages {
    /// Synthetic pages.
    pub fn synthetic() -> Self {
        Self::new(Box::new(SyntheticPageSource::new()), PumpKind::Caller)
    }

    /// Pages from `source`, pumped as `pump` says.
    pub fn new(source: Box<dyn PageSource>, pump: PumpKind) -> Self {
        let name = source.name();
        Self {
            source: Rc::new(RefCell::new(source)),
            inbox: Rc::default(),
            tally: FrameTally::default(),
            pump,
            name,
            spin: false,
            stopped: false,
        }
    }

    /// The source, for one call. Never held across a nested run-loop turn.
    pub fn get(&self) -> RefMut<'_, Box<dyn PageSource>> {
        self.source.borrow_mut()
    }

    /// The source and the inbox, for the run-loop observer.
    pub fn shared(&self) -> (SharedSource, Inbox) {
        (Rc::clone(&self.source), Rc::clone(&self.inbox))
    }

    /// Turns the source once and appends everything it reported to `out`:
    /// what an observer took first, then what is new.
    pub fn take_events(&mut self, out: &mut Vec<PageEvent>) {
        let start = out.len();
        out.append(&mut self.inbox.borrow_mut());
        {
            let mut source = self.source.borrow_mut();
            source.flush_geometry();
            source.pump();
            source.drain_events(out);
        }
        let now = Instant::now();
        for event in &out[start..] {
            if let PageEvent::Frame(frame) = event {
                match frame.frame {
                    PageFrame::GpuShared(_) => self.tally.shared += 1,
                    PageFrame::Cpu(_) => self.tally.cpu += 1,
                }
                let delay = now.saturating_duration_since(frame.produced_at).as_micros();
                self.tally.delay_us += delay;
                self.tally.max_delay_us = self.tally.max_delay_us.max(delay);
            }
        }
    }

    /// One step of shutting the source down from a running loop; true once
    /// it is done.
    pub fn poll_shutdown(&mut self) -> bool {
        self.stopped = self.source.borrow_mut().poll_shutdown();
        self.stopped
    }
}

/// What the app is told about a page event: the kinds that change what is
/// drawn or how input reaches the page. `specular_shell::page_notice` has
/// them all.
pub fn notice_of(event: &PageEvent) -> Option<(PageId, PageNotice)> {
    Some(match event {
        PageEvent::Loaded { page, http_status } => (
            *page,
            PageNotice::Loaded {
                http_status: *http_status,
            },
        ),
        PageEvent::Crashed { page, reason } => (
            *page,
            PageNotice::Crashed {
                reason: reason.clone(),
            },
        ),
        PageEvent::ImeCompositionBounds { page, bounds } => {
            (*page, PageNotice::ImeCompositionBounds(*bounds))
        }
        PageEvent::Cursor { page, cursor } => (*page, PageNotice::Cursor(*cursor)),
        PageEvent::Title { page, title } => (*page, PageNotice::Title(title.clone())),
        PageEvent::Url { page, url } => (*page, PageNotice::Url(url.clone())),
        PageEvent::Loading {
            page,
            loading,
            can_go_back,
            can_go_forward,
        } => (
            *page,
            PageNotice::Loading {
                loading: *loading,
                can_go_back: *can_go_back,
                can_go_forward: *can_go_forward,
            },
        ),
        PageEvent::Scrolled { page, offset } => (
            *page,
            PageNotice::Scrolled {
                x: f64::from(offset.x),
                y: f64::from(offset.y),
            },
        ),
        PageEvent::ElementPlaces { page, places } => {
            (*page, PageNotice::ElementPlaces(places.clone()))
        }
        _ => return None,
    })
}

impl Drop for Pages {
    /// A run that ends with CEF still up stops it here, blocking, as
    /// `CefPageSource`'s own drop would; says so, since a window should
    /// have stopped it from its loop first.
    fn drop(&mut self) {
        if self.name != "cef" || self.stopped {
            return;
        }
        let Ok(mut source) = self.source.try_borrow_mut() else {
            return;
        };
        let started = Instant::now();
        source.shutdown();
        eprintln!(
            "rune-shell: page source stopped as the shell went, in {} ms",
            started.elapsed().as_millis()
        );
    }
}
