//! The tab ref resolver: what a caller outside the app typed to name a
//! canvas, an id, an exact name, or `<project>/<name>` when the name is in
//! more than one project. One resolver, so `tab switch`,
//! `tab delete` and `--tab` agree on what a ref means and fail alike.

use super::{Canvas, Project};

/// Why a tab ref named no canvas. The text is what the caller is told.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TabRefError {
    /// The ref was empty.
    #[error("a tab ref (id or name) is required")]
    Empty,
    /// Several canvases have the name.
    #[error("tab name '{name}' matches {} tabs: {} \u{2014} use an id", ids.len(), ids.join(", "))]
    Ambiguous {
        /// The name asked for.
        name: String,
        /// The ids of the canvases that have it.
        ids: Vec<String>,
    },
    /// No canvas has the id or the name.
    #[error("unknown tab '{tab_ref}' \u{2014} available: {available}")]
    Unknown {
        /// The ref asked for.
        tab_ref: String,
        /// Every canvas, as `id (name)`.
        available: String,
    },
}

/// The canvas `tab_ref` names: an exact id first, then an exact name, then
/// `<project>/<name>` with the project's label or id, all once trimmed. It never guesses: an ambiguous name lists the ids that
/// have it and an unknown ref lists every canvas, so a caller can retry
/// without asking again.
pub(crate) fn resolve_tab_ref<'a>(
    canvases: &'a [Canvas],
    projects: &[Project],
    tab_ref: &str,
) -> Result<&'a Canvas, TabRefError> {
    let wanted = tab_ref.trim();
    if wanted.is_empty() {
        return Err(TabRefError::Empty);
    }
    if let Some(by_id) = canvases.iter().find(|canvas| canvas.id.as_str() == wanted) {
        return Ok(by_id);
    }
    let by_name: Vec<&Canvas> = (canvases.iter())
        .filter(|canvas| canvas.name.trim() == wanted)
        .collect();
    let in_project = || {
        let (project, name) = wanted.split_once('/')?;
        let project = (projects.iter())
            .find(|each| each.label == project.trim() || each.id.as_str() == project.trim())?;
        (canvases.iter())
            .find(|canvas| canvas.project == project.id && canvas.name.trim() == name.trim())
    };
    if by_name.is_empty()
        && let Some(found) = in_project()
    {
        return Ok(found);
    }
    match by_name.as_slice() {
        [only] => Ok(only),
        [] => Err(TabRefError::Unknown {
            tab_ref: wanted.to_owned(),
            available: (canvases.iter())
                .map(|canvas| format!("{} ({})", canvas.id, canvas.name))
                .collect::<Vec<_>>()
                .join(", "),
        }),
        several => Err(TabRefError::Ambiguous {
            name: wanted.to_owned(),
            ids: (several.iter())
                .map(|canvas| canvas.id.as_str().to_owned())
                .collect(),
        }),
    }
}
