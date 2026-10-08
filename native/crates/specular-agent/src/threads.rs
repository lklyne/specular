use std::collections::BTreeMap;

use crate::json::{Index, index_json};
use crate::run::Run;
use crate::thread::{Image, Message, Role, Status, Thread, ThreadId, title_from_messages};

/// What the caller must write after a mutation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Changed {
    /// Thread files to write.
    pub threads: Vec<ThreadId>,
    /// True when `index.json` needs writing.
    pub index: bool,
}

impl Changed {
    pub(crate) fn thread(id: &ThreadId) -> Self {
        Self {
            threads: vec![id.clone()],
            index: false,
        }
    }
}

/// Every thread of the space, the active thread of each canvas, and the runs
/// in flight.
#[derive(Debug, Clone, Default)]
pub struct Threads {
    pub(crate) items: Vec<Thread>,
    pub(crate) active: BTreeMap<String, ThreadId>,
    pub(crate) runs: BTreeMap<ThreadId, Run>,
}

impl Threads {
    /// Replaces everything with what the disk held. An active thread that is
    /// missing, closed, or on another canvas is dropped. Electron's single
    /// active id is taken for its thread's canvas when the map has none there.
    pub fn load(&mut self, threads: Vec<Thread>, index: &Index) {
        let mut candidates = index.by_canvas.clone();
        if let Some(id) = &index.active
            && let Some(thread) = threads.iter().find(|t| &t.id == id)
        {
            candidates
                .entry(thread.tab_id.clone())
                .or_insert_with(|| id.clone());
        }
        self.active = candidates
            .into_iter()
            .filter(|(tab, id)| {
                threads
                    .iter()
                    .any(|t| &t.id == id && &t.tab_id == tab && t.status != Status::Closed)
            })
            .collect();
        self.items = threads;
        self.runs.clear();
    }

    /// The text for `index.json`, naming `tab`'s active thread as the single
    /// active id Electron reads.
    pub fn index_json(&self, tab: Option<&str>) -> String {
        index_json(tab.and_then(|t| self.active.get(t)), &self.active)
    }

    /// Every thread, closed ones included.
    pub fn all(&self) -> &[Thread] {
        &self.items
    }

    /// The canvas's threads that are not closed, newest first.
    pub fn for_canvas(&self, tab: &str) -> Vec<&Thread> {
        let mut list: Vec<&Thread> = self
            .items
            .iter()
            .filter(|t| t.tab_id == tab && t.status != Status::Closed)
            .collect();
        list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        list
    }

    /// A thread by id.
    pub fn get(&self, id: &ThreadId) -> Option<&Thread> {
        self.items.iter().find(|t| &t.id == id)
    }

    /// The canvas's open conversation.
    pub fn active(&self, tab: &str) -> Option<&Thread> {
        self.active.get(tab).and_then(|id| self.get(id))
    }

    /// The run of a thread, going or failed.
    pub fn run(&self, id: &ThreadId) -> Option<&Run> {
        self.runs.get(id)
    }

    /// True while the agent is working on the thread.
    pub fn is_running(&self, id: &ThreadId) -> bool {
        self.runs
            .get(id)
            .is_some_and(|r| r.state == crate::run::RunState::Running)
    }

    /// The user messages waiting to be sent.
    pub fn queued(&self, id: &ThreadId) -> Vec<&Message> {
        self.get(id).map(queued_of).unwrap_or_default()
    }

    /// The thread a comment belongs to (closed threads do not count).
    pub fn thread_of_annotation(&self, annotation_id: &str) -> Option<&Thread> {
        self.items.iter().find(|t| {
            t.status != Status::Closed && t.annotation_ids.iter().any(|a| a == annotation_id)
        })
    }

    /// A fresh draft, made the canvas's active thread.
    pub fn new_thread(&mut self, tab: &str, id: ThreadId, now: &str) -> Changed {
        self.insert_draft(tab, id.clone(), now);
        Changed {
            threads: vec![id],
            index: true,
        }
    }

    /// Opens a non-closed thread of the canvas.
    pub fn select(&mut self, tab: &str, id: &ThreadId) -> Changed {
        let ok = self
            .get(id)
            .is_some_and(|t| t.tab_id == tab && t.status != Status::Closed);
        if !ok || self.active.get(tab) == Some(id) {
            return Changed::default();
        }
        self.active.insert(tab.to_owned(), id.clone());
        Changed {
            threads: Vec::new(),
            index: true,
        }
    }

    /// Back to the list; nothing is archived.
    pub fn deselect(&mut self, tab: &str) -> Changed {
        Changed {
            threads: Vec::new(),
            index: self.active.remove(tab).is_some(),
        }
    }

    /// Archives a thread. Refused while the agent is working on it.
    pub fn close(&mut self, id: &ThreadId, now: &str) -> Changed {
        if self.is_running(id) {
            return Changed::default();
        }
        let Some(thread) = self.items.iter_mut().find(|t| &t.id == id) else {
            return Changed::default();
        };
        thread.status = Status::Closed;
        now.clone_into(&mut thread.updated_at);
        let before = self.active.len();
        self.active.retain(|_, active| active != id);
        self.runs.remove(id);
        Changed {
            threads: vec![id.clone()],
            index: self.active.len() != before,
        }
    }

    /// A comment joins the canvas's draft, or starts one. A pin that already
    /// has a thread adds to that thread instead and opens it.
    pub fn queue_comment(
        &mut self,
        tab: &str,
        new_thread_id: ThreadId,
        message_id: &str,
        annotation_id: &str,
        text: &str,
        now: &str,
    ) -> (ThreadId, Changed) {
        let before = self.active.clone();
        let existing = self
            .thread_of_annotation(annotation_id)
            .map(|t| (t.id.clone(), t.tab_id.clone()));
        let (id, key) = if let Some(found) = existing {
            found
        } else {
            let draft = self
                .active(tab)
                .filter(|t| t.status == Status::Draft)
                .map(|t| t.id.clone());
            if let Some(draft) = draft {
                (draft, tab.to_owned())
            } else {
                self.insert_draft(tab, new_thread_id.clone(), now);
                (new_thread_id, tab.to_owned())
            }
        };
        self.active.insert(key, id.clone());
        let text = text.trim();
        let message = Message {
            annotation_id: Some(annotation_id.to_owned()),
            ..user_message(
                message_id,
                if text.is_empty() { "(comment)" } else { text },
                Vec::new(),
                now,
            )
        };
        self.append(&id, message, now);
        let changed = Changed {
            threads: vec![id.clone()],
            index: before != self.active,
        };
        (id, changed)
    }

    /// The composer's text (and images) as a queued message on the active
    /// thread, starting a draft when there is none. `None` when there is
    /// nothing to send and no thread to send it on.
    pub fn queue_message(
        &mut self,
        tab: &str,
        new_thread_id: ThreadId,
        message_id: &str,
        text: &str,
        images: Vec<Image>,
        now: &str,
    ) -> Option<(ThreadId, Changed)> {
        let has_content = !text.trim().is_empty() || !images.is_empty();
        let active = self.active(tab).map(|t| t.id.clone());
        if !has_content {
            return active.map(|id| (id, Changed::default()));
        }
        let (id, started) = if let Some(id) = active {
            (id, false)
        } else {
            self.insert_draft(tab, new_thread_id.clone(), now);
            (new_thread_id, true)
        };
        self.append(&id, user_message(message_id, text.trim(), images, now), now);
        Some((
            id.clone(),
            Changed {
                threads: vec![id],
                index: started,
            },
        ))
    }

    fn insert_draft(&mut self, tab: &str, id: ThreadId, now: &str) {
        self.items.push(Thread {
            id: id.clone(),
            tab_id: tab.to_owned(),
            title: "New thread".to_owned(),
            status: Status::Draft,
            created_at: now.to_owned(),
            updated_at: now.to_owned(),
            claude_session_id: None,
            annotation_ids: Vec::new(),
            messages: Vec::new(),
        });
        self.active.insert(tab.to_owned(), id);
    }

    fn append(&mut self, id: &ThreadId, message: Message, now: &str) {
        let Some(thread) = self.items.iter_mut().find(|t| &t.id == id) else {
            return;
        };
        if let Some(a) = &message.annotation_id
            && !thread.annotation_ids.contains(a)
        {
            thread.annotation_ids.push(a.clone());
        }
        thread.messages.push(message);
        thread.title = title_from_messages(&thread.messages);
        now.clone_into(&mut thread.updated_at);
    }
}

fn user_message(id: &str, text: &str, images: Vec<Image>, now: &str) -> Message {
    Message {
        id: id.to_owned(),
        role: Role::User,
        text: text.to_owned(),
        created_at: now.to_owned(),
        queued: true,
        annotation_id: None,
        images,
    }
}

pub(crate) fn queued_of(thread: &Thread) -> Vec<&Message> {
    thread
        .messages
        .iter()
        .filter(|m| m.queued && m.role == Role::User && m.has_content())
        .collect()
}
