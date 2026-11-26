//! Every behavior test of `specular-interact`, as one binary: a module a
//! file, so the crate is linked once and not once a file. Snapshots are in
//! `snapshots/`, named `it__<module>__<test>.snap`.

mod bindings;
mod clipboard;
mod commands;
mod comment_rules;
mod flick;
mod guides;
mod images;
mod layout;
mod motion;
mod palette;
mod physical_key;
mod resizes;
mod shared;
mod space;
mod state;
mod sync;
mod text_fit;
mod text_rules;
