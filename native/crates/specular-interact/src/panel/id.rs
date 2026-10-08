//! [`ControlId`]: the name a renderer tracks a control by across frames.

use std::borrow::Cow;
use std::fmt;

/// A control's name, unique within one model and the same from one model to
/// the next while the control is the same, so a renderer can keep hover,
/// press and open state against it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ControlId(Cow<'static, str>);

impl ControlId {
    /// A control named `name`.
    pub const fn new(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }

    /// The control inside this one called `part`: an option of a dropdown, a
    /// swatch of a row.
    #[must_use]
    pub fn child(&self, part: impl fmt::Display) -> Self {
        Self(Cow::Owned(format!("{}.{part}", self.0)))
    }

    /// The name as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ControlId {
    fn from(name: String) -> Self {
        Self(Cow::Owned(name))
    }
}

impl fmt::Display for ControlId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
