//! Where a control is, by the name its model gives it: where GPUI last laid
//! out the Kit element that draws it.

use glam::Vec2;

use crate::view::shown_controls;

/// The middle of the control named `name` among `shown`, or an error
/// listing the names that are shown. The list is in paint order, and a
/// name drawn twice is clicked where it is in front.
pub(super) fn find(name: &str, shown: &[(String, Vec2)]) -> Result<Vec2, String> {
    (shown.iter().rev())
        .find(|(shown, _)| shown == name)
        .map(|(_, at)| *at)
        .ok_or_else(|| {
            let mut names: Vec<&str> = Vec::new();
            for (shown, _) in shown {
                if !names.contains(&shown.as_str()) {
                    names.push(shown);
                }
            }
            format!(
                "no control `{name}` is shown; these are: {}",
                names.join(" ")
            )
        })
}

/// The middle of the control named `name` as the window shows it now.
pub(super) fn locate(name: &str) -> Result<Vec2, String> {
    find(name, &shown_controls())
}
