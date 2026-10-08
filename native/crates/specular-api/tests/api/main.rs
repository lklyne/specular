//! The API driven through a scripted app, with no socket: a request goes
//! in, the event it plans goes through `update`, and the response and the
//! document come out.

mod common;
mod contract;
mod plan;
mod verbs;
