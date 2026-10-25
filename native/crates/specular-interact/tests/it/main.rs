//! Every behavior test of `specular-interact`, as one binary: a module a
//! file, so the crate is linked once and not once a file. Snapshots are in
//! `snapshots/`, named `it__<module>__<test>.snap`.

mod anchoring;
mod arrange;
mod attachment;
mod auto_layout;
mod bindings;
mod clipboard;
mod comment_rules;
mod cursors;
mod draw;
mod drop;
mod edge_labels;
mod edges;
mod flick;
mod gestures;
mod groups_drag;
mod groups_enter;
mod groups_fit;
mod groups_verbs;
mod guides;
mod history_selection;
mod hit_test;
mod html_files;
mod images;
mod layout_handles;
mod motion;
mod moves;
mod palette;
mod physical_key;
mod polish;
mod resizes;
mod routing;
mod safe_area;
mod save;
mod select;
mod space;
mod stack_order;
mod sync;
mod text_fit;
mod text_rules;
mod theme;
mod tools;
mod verbs;
