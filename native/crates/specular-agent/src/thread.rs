use crate::text::truncate;

/// A thread's id, chosen by the caller.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ThreadId(pub String);

impl ThreadId {
    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Where a thread is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Comments are queuing; nothing was sent yet.
    Draft,
    /// The agent answered at least once; follow-ups resume the session.
    Open,
    /// Archived: out of the list, still on disk.
    Closed,
}

impl Status {
    /// The word the file stores.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Open => "open",
            Self::Closed => "closed",
        }
    }
}

/// Who wrote a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The person.
    User,
    /// The agent.
    Agent,
}

/// Image formats the model accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    /// `image/png`
    Png,
    /// `image/jpeg`
    Jpeg,
    /// `image/gif`
    Gif,
    /// `image/webp`
    Webp,
}

impl MediaType {
    /// The MIME string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
        }
    }

    /// The format a MIME string names.
    pub fn parse(value: &str) -> Option<Self> {
        [Self::Png, Self::Jpeg, Self::Gif, Self::Webp]
            .into_iter()
            .find(|kind| kind.as_str() == value)
    }
}

/// A pasted image saved beside the thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// Relative to the space folder.
    pub path: String,
    /// The format.
    pub media_type: MediaType,
}

/// One line of the conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Unique within the thread.
    pub id: String,
    /// Who wrote it.
    pub role: Role,
    /// The words.
    pub text: String,
    /// ISO-8601.
    pub created_at: String,
    /// True until the thread is sent.
    pub queued: bool,
    /// The comment this message came from.
    pub annotation_id: Option<String>,
    /// Pasted images.
    pub images: Vec<Image>,
}

impl Message {
    /// A user message worth sending: words, images, or both.
    pub fn has_content(&self) -> bool {
        !self.text.trim().is_empty() || !self.images.is_empty()
    }
}

/// A conversation on one canvas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thread {
    /// Unique in the space.
    pub id: ThreadId,
    /// The canvas it belongs to.
    pub tab_id: String,
    /// Derived from the first user message.
    pub title: String,
    /// Where it is in its life.
    pub status: Status,
    /// ISO-8601.
    pub created_at: String,
    /// ISO-8601.
    pub updated_at: String,
    /// The `claude` session to resume.
    pub claude_session_id: Option<String>,
    /// The comments that fed it.
    pub annotation_ids: Vec<String>,
    /// The conversation, oldest first.
    pub messages: Vec<Message>,
}

/// The title a thread gets from its messages: the first user text, cut to 48
/// characters.
pub fn title_from_messages(messages: &[Message]) -> String {
    let first = messages
        .iter()
        .find(|m| m.role == Role::User && !m.text.trim().is_empty());
    match first {
        Some(m) => {
            let words: Vec<&str> = m.text.split_whitespace().collect();
            truncate(&words.join(" "), 48)
        }
        None if messages.iter().any(|m| !m.images.is_empty()) => "Image".to_owned(),
        None => "New thread".to_owned(),
    }
}
