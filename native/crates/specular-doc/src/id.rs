//! Identifiers. They are the `.canvas` id strings, so an id survives a
//! remove and re-insert and an undo restores the same identity.

use serde::{Deserialize, Serialize};

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Wraps an id string.
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// The id as written in the `.canvas` file.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(id: &str) -> Self {
                Self(id.to_owned())
            }
        }

        impl From<String> for $name {
            fn from(id: String) -> Self {
                Self(id)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id!(
    /// Identifies an [`Entity`](crate::Entity).
    EntityId
);
string_id!(
    /// Identifies an [`Edge`](crate::Edge).
    EdgeId
);
string_id!(
    /// Identifies an [`Annotation`](crate::Annotation).
    AnnotationId
);

/// One slot in the stack order. Entities and edges share the stack, and
/// share one id namespace, so an id names at most one of them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ItemId {
    /// An entity's slot.
    Entity(EntityId),
    /// An edge's slot.
    Edge(EdgeId),
}

impl ItemId {
    /// The id as written in the `.canvas` file's `entityOrder`.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Entity(id) => id.as_str(),
            Self::Edge(id) => id.as_str(),
        }
    }
}

impl From<EntityId> for ItemId {
    fn from(id: EntityId) -> Self {
        Self::Entity(id)
    }
}

impl From<EdgeId> for ItemId {
    fn from(id: EdgeId) -> Self {
        Self::Edge(id)
    }
}
