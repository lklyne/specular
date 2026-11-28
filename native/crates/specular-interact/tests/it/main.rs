//! Every behavior test of `specular-interact`, as one binary: a module a
//! file, so the crate is linked once and not once a file. Snapshots are in
//! `snapshots/`, named `it__<module>__<test>.snap`.

mod bindings;
mod bridge;
mod clipboard;
mod commands;
mod comment_rules;
mod context_menu;
mod flick;
mod guides;
mod images;
mod keys;
mod layout;
mod mods;
mod mods_off;
mod motion;
mod overlays;
mod palette;
mod physical_key;
mod presence;
mod resizes;
mod self_anchor;
mod shared;
mod space;
mod state;
mod sync;
mod text_fit;
mod text_rules;
