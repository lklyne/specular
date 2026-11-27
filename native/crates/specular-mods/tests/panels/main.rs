//! The phase 4 gates (ADR 0055): each small panel's shipped mod over the
//! keyed state of a real app. Each shows every control the native panel's
//! model lists, leaves out what its props hide, presses a control through
//! the host, renders only when a key it read moves (never while idle,
//! never for a camera move), and fast enough for a frame. Run with
//! `--release -- --nocapture` for the numbers the gate reads; a debug build
//! checks only the counts.

#![expect(clippy::expect_used, reason = "a test fails by panicking")]

#[path = "../support/mod.rs"]
mod support;

mod menu;
mod share;
mod tabs;
mod toast;
mod welcome;
