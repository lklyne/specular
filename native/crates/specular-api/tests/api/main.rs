//! The API driven through a scripted app, with no socket: a request goes
//! in, the event it plans goes through `update`, and the response and the
//! document come out.

mod cdp;
mod common;
mod contract;
mod documents;
mod pages;
mod plan;
mod tabs;
mod tasks;
mod verbs;
