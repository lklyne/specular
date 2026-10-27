//! [`CanvasSync`]: one canvas's shared document beside its [`Document`].

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use specular_doc::{
    AnnotationId, Command, CommandError, Document, EdgeId, Entity, EntityId, ItemId,
};
use yrs::types::{Event, PathSegment};
use yrs::undo::Options;
use yrs::updates::decoder::Decode;
use yrs::{
    Any, DeepObservable, Doc, GetString, In, Map, MapPrelim, MapRef, OffsetKind, Origin, Out,
    ReadTxn, StateVector, Text, TextPrelim, Transact, TransactionMut, UndoManager, Update,
};

use crate::fields::{self, Fields, ORDER};
use crate::order;

/// Local edits that are undo steps.
const LOCAL: &str = "specular:local";
/// Local edits that are not: seeding, drag frames, and the state a step
/// starts from.
const QUIET: &str = "specular:quiet";
/// Updates from peers.
const REMOTE: &str = "specular:remote";

const ENTITIES: &str = "entities";
const EDGES: &str = "edges";
const ANNOTATIONS: &str = "annotations";
const NOTES: &str = "notes";

/// Why a peer's update or state was refused.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    /// The bytes are not a yrs update.
    #[error("not a yrs update: {0}")]
    Decode(String),
    /// The update did not apply.
    #[error("update refused: {0}")]
    Apply(String),
}

/// A canvas's [`Document`] mirrored into a yrs document, so that peers can
/// edit it at the same time and merge without conflicts.
///
/// The [`Document`] stays the app's only copy of the canvas. This holds the
/// shared document beside it and keeps the two in step:
///
/// - Local changes go through [`apply`](Self::apply) or
///   [`apply_from`](Self::apply_from), which are one undo step each, as with
///   [`History`](specular_doc::History), or are reported after the fact with
///   [`record`](Self::record), which is not an undo step. Each becomes yrs
///   changes, collected for [`take_outgoing`](Self::take_outgoing).
/// - A peer's update goes to [`apply_remote`](Self::apply_remote), which
///   merges it and applies the difference to the [`Document`] as commands,
///   with no undo step.
/// - [`undo`](Self::undo) and [`redo`](Self::redo) revert only this peer's
///   steps, and skip any field a peer has changed since.
///
/// `S` is state carried with each undo step, as with `History<S>`: the app
/// passes the selection.
pub struct CanvasSync<S = ()> {
    doc: Doc,
    entities: MapRef,
    edges: MapRef,
    annotations: MapRef,
    notes: MapRef,
    undo: UndoManager,
    dirty: Arc<Mutex<Dirty>>,
    outgoing: Arc<Mutex<Vec<Vec<u8>>>>,
    /// The carried state of each step on the undo manager's undo stack, in
    /// the same order.
    undo_states: Vec<Step<S>>,
    /// The same for its redo stack.
    redo_states: Vec<Step<S>>,
    open: bool,
    revision: u64,
}

impl<S> std::fmt::Debug for CanvasSync<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CanvasSync")
            .field("client", &self.doc.client_id())
            .field("undo", &self.undo_states.len())
            .field("redo", &self.redo_states.len())
            .field("revision", &self.revision)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
struct Step<S> {
    before: S,
    after: S,
}

/// What a transaction changed, by item id, gathered by the observers.
#[derive(Debug, Default)]
struct Dirty {
    entities: BTreeSet<String>,
    edges: BTreeSet<String>,
    annotations: BTreeSet<String>,
    notes: BTreeSet<String>,
    /// An entity or edge was added or removed, or its order key changed.
    order: bool,
    /// The same for annotations.
    annotation_order: bool,
}

/// The items a command touches, which are the ones to copy into the shared
/// document after it runs.
#[derive(Debug, Default)]
struct Touched {
    entities: BTreeSet<EntityId>,
    edges: BTreeSet<EdgeId>,
    annotations: BTreeSet<AnnotationId>,
    notes: BTreeSet<String>,
    order: bool,
    annotation_order: bool,
}

impl<S: Clone + Default> CanvasSync<S> {
    /// A shared document holding `document`, for the peer that publishes
    /// it. Peers that join use [`join`](Self::join) instead, so that two
    /// copies of the same canvas are not merged item by item.
    pub fn new(document: &Document) -> Self {
        let mut sync = Self::empty();
        let touched = Touched::everything(document);
        {
            let mut txn = sync.doc.transact_mut_with(QUIET);
            sync.write(&mut txn, document, &touched);
        }
        sync.clear_dirty();
        // The seed is the state a joining peer starts from, not an update.
        sync.take_outgoing();
        sync
    }

    /// Joins a canvas from a peer's full state (the bytes of
    /// [`encode_state`](Self::encode_state)), returning the shared document
    /// and the [`Document`] it holds.
    pub fn join(state: &[u8]) -> Result<(Self, Document), SyncError> {
        let mut sync = Self::empty();
        let mut document = Document::new();
        sync.apply_remote(state, &mut document)?;
        // Joining is not something to send back.
        sync.take_outgoing();
        Ok((sync, document))
    }

    fn empty() -> Self {
        // Utf16 matches Yjs, so a JavaScript peer and this one agree on
        // text positions.
        let doc = Doc::with_options(yrs::Options {
            offset_kind: OffsetKind::Utf16,
            ..yrs::Options::default()
        });
        let entities = doc.get_or_insert_map(ENTITIES);
        let edges = doc.get_or_insert_map(EDGES);
        let annotations = doc.get_or_insert_map(ANNOTATIONS);
        let notes = doc.get_or_insert_map(NOTES);

        let mut undo = UndoManager::with_options(Options {
            // Every step is one transaction, so nothing is merged by time.
            capture_timeout_millis: 0,
            tracked_origins: std::iter::once(Origin::from(LOCAL)).collect(),
            capture_transaction: None,
            timestamp: Arc::new(|| 0),
            init_undo_stack: Vec::new(),
            init_redo_stack: Vec::new(),
        });
        for root in [&entities, &edges, &annotations, &notes] {
            undo.expand_scope(&doc, root);
        }

        let dirty = Arc::new(Mutex::new(Dirty::default()));
        observe(
            &entities,
            &dirty,
            |dirty| &mut dirty.entities,
            |d| d.order = true,
        );
        observe(&edges, &dirty, |dirty| &mut dirty.edges, |d| d.order = true);
        observe(
            &annotations,
            &dirty,
            |dirty| &mut dirty.annotations,
            |d| {
                d.annotation_order = true;
            },
        );
        observe(&notes, &dirty, |dirty| &mut dirty.notes, |_| {});

        let outgoing = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&outgoing);
        let subscribed = doc.observe_update_v1("specular:outgoing", move |txn, event| {
            if txn.origin() != Some(&Origin::from(REMOTE)) {
                lock(&sink).push(event.update.clone());
            }
        });
        if let Err(error) = subscribed {
            tracing::error!("no outgoing updates: {error}");
        }

        Self {
            doc,
            entities,
            edges,
            annotations,
            notes,
            undo,
            dirty,
            outgoing,
            undo_states: Vec::new(),
            redo_states: Vec::new(),
            open: false,
            revision: 0,
        }
    }

    /// Runs `command` on `document` as one undo step, and shares it.
    pub fn apply(&mut self, document: &mut Document, command: Command) -> Result<(), CommandError> {
        self.apply_from(document, command, S::default())
    }

    /// [`apply`](Self::apply), with `before` as the state an undo of this
    /// step returns to, as [`History::apply_from`](specular_doc::History::apply_from).
    pub fn apply_from(
        &mut self,
        document: &mut Document,
        command: Command,
        before: S,
    ) -> Result<(), CommandError> {
        let touched = Touched::by(&command);
        // The step starts from what the document holds now, even if a drag
        // already shared frames past it, so that its undo comes back here.
        {
            let mut txn = self.doc.transact_mut_with(QUIET);
            self.write(&mut txn, document, &touched);
        }
        document.apply(command)?;
        let stack = self.undo.undo_stack().len();
        {
            let mut txn = self.doc.transact_mut_with(LOCAL);
            self.write(&mut txn, document, &touched);
        }
        self.clear_dirty();
        if self.undo.undo_stack().len() > stack {
            self.undo_states.push(Step {
                after: before.clone(),
                before,
            });
            self.redo_states.clear();
            self.open = true;
        }
        self.revision += 1;
        Ok(())
    }

    /// Shares a command the caller already ran on `document`, with no undo
    /// step: a frame of a drag, or anything else that changed the document
    /// outside [`apply`](Self::apply).
    pub fn record(&mut self, document: &Document, command: &Command) {
        let touched = Touched::by(command);
        {
            let mut txn = self.doc.transact_mut_with(QUIET);
            self.write(&mut txn, document, &touched);
        }
        self.clear_dirty();
    }

    /// Gives the latest step the state it left behind, as
    /// [`History::settle`](specular_doc::History::settle).
    pub fn settle(&mut self, after: S) {
        if self.open
            && let Some(step) = self.undo_states.last_mut()
        {
            step.after = after;
        }
        self.open = false;
    }

    /// Undoes this peer's latest step that still changes something, and
    /// returns the state from before it. A field a peer has changed since
    /// keeps the peer's value.
    pub fn undo(&mut self, document: &mut Document) -> Option<S> {
        self.step(document, true).map(|step| step.before)
    }

    /// Redoes the latest undone step and returns the state from after it.
    pub fn redo(&mut self, document: &mut Document) -> Option<S> {
        self.step(document, false).map(|step| step.after)
    }

    /// Whether [`undo`](Self::undo) has a step to try.
    pub fn can_undo(&self) -> bool {
        self.undo.can_undo()
    }

    /// Whether [`redo`](Self::redo) has a step to try.
    pub fn can_redo(&self) -> bool {
        self.undo.can_redo()
    }

    /// A count that goes up with each local step, undo, redo and applied
    /// remote change, as [`History::revision`](specular_doc::History::revision).
    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn step(&mut self, document: &mut Document, undoing: bool) -> Option<Step<S>> {
        self.open = false;
        let (from, to) = if undoing {
            (self.undo.undo_stack().len(), self.undo.redo_stack().len())
        } else {
            (self.undo.redo_stack().len(), self.undo.undo_stack().len())
        };
        if from == 0 {
            return None;
        }
        let changed = if undoing {
            self.undo.undo_blocking()
        } else {
            self.undo.redo_blocking()
        };
        let (from_after, to_after) = if undoing {
            (self.undo.undo_stack().len(), self.undo.redo_stack().len())
        } else {
            (self.undo.redo_stack().len(), self.undo.undo_stack().len())
        };
        let (from_states, to_states) = if undoing {
            (&mut self.undo_states, &mut self.redo_states)
        } else {
            (&mut self.redo_states, &mut self.undo_states)
        };
        // The manager pops steps until one changes something; the last one
        // popped is the one that ran.
        let popped = from_states.split_off(from_after.min(from_states.len()));
        let ran = popped.first().cloned();
        if to_after > to
            && let Some(ran) = &ran
        {
            to_states.push(ran.clone());
        }
        self.project(document);
        if changed {
            self.revision += 1;
            ran
        } else {
            None
        }
    }

    /// Merges a peer's update and applies what it changed to `document`,
    /// with no undo step. Returns the commands that ran, or `None` when the
    /// update changed nothing here.
    pub fn apply_remote(
        &mut self,
        update: &[u8],
        document: &mut Document,
    ) -> Result<Option<Command>, SyncError> {
        let update = Update::decode_v1(update).map_err(|e| SyncError::Decode(e.to_string()))?;
        {
            let mut txn = self.doc.transact_mut_with(REMOTE);
            txn.apply_update(update)
                .map_err(|e| SyncError::Apply(e.to_string()))?;
        }
        let applied = self.project(document);
        if applied.is_some() {
            self.revision += 1;
        }
        Ok(applied)
    }

    /// The updates made here since the last call, merged into one, to send
    /// to peers. `None` when there is nothing new.
    pub fn take_outgoing(&mut self) -> Option<Vec<u8>> {
        let updates = std::mem::take(&mut *lock(&self.outgoing));
        match updates.len() {
            0 => None,
            1 => updates.into_iter().next(),
            _ => match yrs::merge_updates_v1(&updates) {
                Ok(merged) => Some(merged),
                Err(error) => {
                    tracing::error!("outgoing updates not merged: {error}");
                    None
                }
            },
        }
    }

    /// The whole shared document as one update, for a peer joining or for
    /// the server to store.
    pub fn encode_state(&self) -> Vec<u8> {
        self.doc
            .transact()
            .encode_state_as_update_v1(&StateVector::default())
    }

    /// What this peer has seen, encoded, for a peer to answer with
    /// [`diff`](Self::diff).
    pub fn state_vector(&self) -> Vec<u8> {
        use yrs::updates::encoder::Encode;
        self.doc.transact().state_vector().encode_v1()
    }

    /// What this peer has that a peer with `state_vector` lacks.
    pub fn diff(&self, state_vector: &[u8]) -> Result<Vec<u8>, SyncError> {
        let state_vector =
            StateVector::decode_v1(state_vector).map_err(|e| SyncError::Decode(e.to_string()))?;
        Ok(self.doc.transact().encode_state_as_update_v1(&state_vector))
    }

    fn clear_dirty(&self) {
        *lock(&self.dirty) = Dirty::default();
    }

    // --- Document to shared document -----------------------------------

    /// Copies the `touched` items from `document` into the shared document,
    /// writing only the fields that differ.
    fn write(&self, txn: &mut TransactionMut<'_>, document: &Document, touched: &Touched) {
        for id in &touched.entities {
            let fields = document.entity(id).and_then(fields::entity);
            put(txn, &self.entities, id.as_str(), fields);
        }
        for id in &touched.edges {
            let fields = document.edge(id).and_then(fields::edge);
            put(txn, &self.edges, id.as_str(), fields);
        }
        for id in &touched.annotations {
            let fields = document.annotation(id).and_then(fields::annotation);
            put(txn, &self.annotations, id.as_str(), fields);
        }
        for file in &touched.notes {
            self.write_note(txn, file, document.note(file));
        }
        if touched.order {
            let items: Vec<(&MapRef, &str)> = (document.order().iter())
                .map(|item| match item {
                    ItemId::Entity(id) => (&self.entities, id.as_str()),
                    ItemId::Edge(id) => (&self.edges, id.as_str()),
                })
                .collect();
            write_order(txn, &items);
        }
        if touched.annotation_order {
            let items: Vec<(&MapRef, &str)> = (document.annotations().iter())
                .map(|annotation| (&self.annotations, annotation.id.as_str()))
                .collect();
            write_order(txn, &items);
        }
    }

    /// Edits the shared text of a note into `text` by replacing only the
    /// span that changed, so that peers typing in other parts of it both
    /// keep their typing.
    fn write_note(&self, txn: &mut TransactionMut<'_>, file: &str, text: Option<&str>) {
        let Some(text) = text else {
            if self.notes.get(txn, file).is_some() {
                self.notes.remove(txn, file);
            }
            return;
        };
        let Some(Out::YText(shared)) = self.notes.get(txn, file) else {
            self.notes.insert(txn, file, TextPrelim::new(text));
            return;
        };
        let current = shared.get_string(txn);
        let (start, removed, inserted) = changed_span(&current, text);
        if removed > 0 {
            shared.remove_range(
                txn,
                utf16_len(&current[..start]),
                utf16_len(&current[start..start + removed]),
            );
        }
        if !inserted.is_empty() {
            shared.insert(txn, utf16_len(&current[..start]), inserted);
        }
    }

    // --- Shared document to Document -----------------------------------

    /// Applies to `document` whatever the last transactions changed in the
    /// shared document, and returns the commands that ran.
    fn project(&self, document: &mut Document) -> Option<Command> {
        let dirty = std::mem::take(&mut *lock(&self.dirty));
        let Run {
            mut ran, refused, ..
        } = self.project_dirty(document, &dirty);
        let repaired = repair_cycles(document, &refused, &mut ran);
        if !repaired.is_empty() {
            {
                let mut txn = self.doc.transact_mut_with(QUIET);
                self.write(&mut txn, document, &repaired);
            }
            self.clear_dirty();
        }
        match ran.len() {
            0 => None,
            1 => ran.pop(),
            _ => Some(Command::Batch(ran)),
        }
    }

    fn project_dirty<'a>(&self, document: &'a mut Document, dirty: &Dirty) -> Run<'a> {
        let txn = self.doc.transact();
        let mut run = Run {
            document,
            ran: Vec::new(),
            refused: Vec::new(),
        };
        self.project_stack(&txn, dirty, &mut run);
        self.project_annotations(&txn, dirty, &mut run);
        for file in &dirty.notes {
            let wanted = match self.notes.get(&txn, file) {
                Some(Out::YText(text)) => Some(text.get_string(&txn)),
                _ => None,
            };
            if run.document.note(file) != wanted.as_deref() {
                run.run(Command::SetNote {
                    file: file.clone(),
                    text: wanted,
                });
            }
        }
        run
    }

    /// Entities, edges and their stack order.
    fn project_stack<T: ReadTxn>(&self, txn: &T, dirty: &Dirty, run: &mut Run<'_>) {
        for id in &dirty.entities {
            let id = EntityId::from(id.as_str());
            let wanted = read_fields(txn, &self.entities, id.as_str())
                .and_then(|fields| fields::to_entity(id.as_str(), fields));
            for command in entity_changes(run.document, &id, wanted) {
                run.run(command);
            }
        }
        for id in &dirty.edges {
            let id = EdgeId::from(id.as_str());
            let wanted = read_fields(txn, &self.edges, id.as_str())
                .and_then(|fields| fields::to_edge(id.as_str(), fields));
            let command = match (run.document.edge(&id), wanted) {
                (Some(_), None) => Some(Command::RemoveEdge(id)),
                (None, Some(edge)) => Some(Command::InsertEdge {
                    edge: Box::new(edge),
                    at: run.document.stack_len(),
                }),
                (Some(current), Some(edge)) if *current != edge => {
                    Some(Command::ReplaceEdge(Box::new(edge)))
                }
                _ => None,
            };
            if let Some(command) = command {
                run.run(command);
            }
        }
        if dirty.order {
            let order = sorted(run.document.order().iter().cloned(), |item| {
                let map = match item {
                    ItemId::Entity(_) => &self.entities,
                    ItemId::Edge(_) => &self.edges,
                };
                (order_key(txn, map, item.as_str()), item.as_str().to_owned())
            });
            if order != run.document.order() {
                run.run(Command::SetOrder(order));
            }
        }
    }

    /// Annotations and their order.
    fn project_annotations<T: ReadTxn>(&self, txn: &T, dirty: &Dirty, run: &mut Run<'_>) {
        for id in &dirty.annotations {
            let id = AnnotationId::from(id.as_str());
            let wanted = read_fields(txn, &self.annotations, id.as_str())
                .and_then(|fields| fields::to_annotation(id.as_str(), fields));
            let command = match (run.document.annotation(&id), wanted) {
                (Some(_), None) => Some(Command::RemoveAnnotation(id)),
                (None, Some(annotation)) => Some(Command::InsertAnnotation {
                    annotation: Box::new(annotation),
                    at: run.document.annotations().len(),
                }),
                (Some(current), Some(annotation)) if *current != annotation => {
                    Some(Command::ReplaceAnnotation(Box::new(annotation)))
                }
                _ => None,
            };
            if let Some(command) = command {
                run.run(command);
            }
        }
        if dirty.annotation_order {
            let ids = run.document.annotations().iter().map(|a| a.id.clone());
            let wanted = sorted(ids, |id| {
                let key = order_key(txn, &self.annotations, id.as_str());
                (key, id.as_str().to_owned())
            });
            // There is no reorder command for annotations, so a misplaced
            // one is taken out and put back where it belongs.
            for (at, id) in wanted.into_iter().enumerate() {
                let current = run.document.annotations().iter().position(|a| a.id == id);
                if current.is_some_and(|current| current != at)
                    && let Some(annotation) = run.document.annotation(&id).cloned()
                {
                    run.run(Command::RemoveAnnotation(id));
                    run.run(Command::InsertAnnotation {
                        annotation: Box::new(annotation),
                        at,
                    });
                }
            }
        }
    }
}

/// The document a projection changes, and the commands it ran.
struct Run<'a> {
    document: &'a mut Document,
    ran: Vec<Command>,
    refused: Vec<Command>,
}

impl Run<'_> {
    fn run(&mut self, command: Command) {
        match self.document.apply(command.clone()) {
            Ok(_) => self.ran.push(command),
            Err(error) => {
                tracing::warn!("a peer's change was refused: {error}");
                self.refused.push(command);
            }
        }
    }
}

/// Two peers that each move a group into the other at the same time leave
/// a loop in the shared document, which this document refuses, so each
/// peer would keep a different half of it. Every peer breaks such a loop
/// the same way: the group with the greatest id in it is taken out of its
/// parent. The other move is then applied, and what was repaired is
/// returned to be written back, so the shared document holds no loop
/// either.
fn repair_cycles(document: &mut Document, refused: &[Command], ran: &mut Vec<Command>) -> Touched {
    let mut repaired = Touched::default();
    for command in refused {
        let Command::SetParent {
            id,
            parent: Some(parent),
        } = command
        else {
            continue;
        };
        // The loop: `id`, `parent`, and the groups above `parent` up to `id`.
        let mut members = vec![id.clone(), parent.clone()];
        members.extend(
            document
                .ancestors(parent)
                .map(|group| group.id.clone())
                .take_while(|group| group != id),
        );
        if !document.ancestors(parent).any(|group| group.id == *id) {
            continue;
        }
        let Some(breaker) = members.iter().max().cloned() else {
            continue;
        };
        let mut steps = vec![Command::SetParent {
            id: breaker.clone(),
            parent: None,
        }];
        if breaker != *id {
            steps.push(command.clone());
        }
        for step in steps {
            if document.apply(step.clone()).is_ok() {
                ran.push(step);
            }
        }
        repaired.entities.insert(breaker);
        repaired.entities.insert(id.clone());
    }
    repaired
}

/// `items` sorted by `key`, an order key and an id. Keyless items, which a
/// refused change can leave, go last.
fn sorted<I>(
    items: impl Iterator<Item = I>,
    key: impl Fn(&I) -> (Option<String>, String),
) -> Vec<I> {
    let mut items: Vec<I> = items.collect();
    items.sort_by_cached_key(|item| {
        let (order, id) = key(item);
        (order.is_none(), order, id)
    });
    items
}

impl Touched {
    fn everything(document: &Document) -> Self {
        Self {
            entities: document.entities().map(|e| e.id.clone()).collect(),
            edges: document.edges().map(|e| e.id.clone()).collect(),
            annotations: document
                .annotations()
                .iter()
                .map(|a| a.id.clone())
                .collect(),
            notes: document.notes().map(|(file, _)| file.to_owned()).collect(),
            order: true,
            annotation_order: true,
        }
    }

    fn by(command: &Command) -> Self {
        let mut touched = Self::default();
        touched.add(command);
        touched
    }

    fn is_empty(&self) -> bool {
        self.entities.is_empty()
            && self.edges.is_empty()
            && self.annotations.is_empty()
            && self.notes.is_empty()
            && !self.order
            && !self.annotation_order
    }

    fn add(&mut self, command: &Command) {
        match command {
            Command::InsertEntity { entity, .. } => {
                self.entities.insert(entity.id.clone());
                self.order = true;
            }
            Command::RemoveEntity(id) => {
                self.entities.insert(id.clone());
                self.order = true;
            }
            Command::SetRect { id, .. }
            | Command::SetLabel { id, .. }
            | Command::SetParent { id, .. }
            | Command::SetAnchor { id, .. }
            | Command::SetKind { id, .. } => {
                self.entities.insert(id.clone());
            }
            Command::InsertEdge { edge, .. } => {
                self.edges.insert(edge.id.clone());
                self.order = true;
            }
            Command::RemoveEdge(id) => {
                self.edges.insert(id.clone());
                self.order = true;
            }
            Command::ReplaceEdge(edge) => {
                self.edges.insert(edge.id.clone());
            }
            Command::SetOrder(_) => self.order = true,
            Command::InsertAnnotation { annotation, .. } => {
                self.annotations.insert(annotation.id.clone());
                self.annotation_order = true;
            }
            Command::RemoveAnnotation(id) => {
                self.annotations.insert(id.clone());
                self.annotation_order = true;
            }
            Command::ReplaceAnnotation(annotation) => {
                self.annotations.insert(annotation.id.clone());
            }
            Command::SetNote { file, .. } => {
                self.notes.insert(file.clone());
            }
            Command::Batch(commands) => {
                for command in commands {
                    self.add(command);
                }
            }
        }
    }
}

/// The commands that turn `document`'s entity `id` into `wanted`, field by
/// field, so that each lands as the command the app itself would use.
fn entity_changes(document: &Document, id: &EntityId, wanted: Option<Entity>) -> Vec<Command> {
    let current = document.entity(id);
    match (current, wanted) {
        (None, None) => Vec::new(),
        (Some(_), None) => vec![Command::RemoveEntity(id.clone())],
        (None, Some(entity)) => vec![Command::InsertEntity {
            entity: Box::new(entity),
            at: document.stack_len(),
        }],
        (Some(current), Some(wanted)) => {
            let same_kind =
                std::mem::discriminant(&current.kind) == std::mem::discriminant(&wanted.kind);
            if !same_kind || current.extra != wanted.extra {
                // No command changes these in place.
                let at = document.stack_index(&ItemId::Entity(id.clone()));
                return vec![
                    Command::RemoveEntity(id.clone()),
                    Command::InsertEntity {
                        entity: Box::new(wanted),
                        at: at.unwrap_or(document.stack_len().saturating_sub(1)),
                    },
                ];
            }
            let mut commands = Vec::new();
            if current.rect != wanted.rect {
                commands.push(Command::SetRect {
                    id: id.clone(),
                    rect: wanted.rect,
                });
            }
            if current.label != wanted.label {
                commands.push(Command::SetLabel {
                    id: id.clone(),
                    label: wanted.label,
                });
            }
            if current.parent != wanted.parent {
                commands.push(Command::SetParent {
                    id: id.clone(),
                    parent: wanted.parent,
                });
            }
            if current.anchor != wanted.anchor {
                commands.push(Command::SetAnchor {
                    id: id.clone(),
                    anchor: wanted.anchor.map(Box::new),
                });
            }
            if current.kind != wanted.kind {
                commands.push(Command::SetKind {
                    id: id.clone(),
                    kind: Box::new(wanted.kind),
                });
            }
            commands
        }
    }
}

/// Writes `fields` as the item `id` of `map`, changing only the fields that
/// differ, or removes the item when `fields` is `None`.
fn put(txn: &mut TransactionMut<'_>, map: &MapRef, id: &str, fields: Option<Fields>) {
    let Some(fields) = fields else {
        if map.get(txn, id).is_some() {
            map.remove(txn, id);
        }
        return;
    };
    let Some(Out::YMap(item)) = map.get(txn, id) else {
        let prelim: MapPrelim = fields.into_iter().map(|(k, v)| (k, In::Any(v))).collect();
        map.insert(txn, id, prelim);
        return;
    };
    let stale: Vec<String> = (item.keys(txn))
        .filter(|key| *key != ORDER && !fields.iter().any(|(field, _)| field == key))
        .map(str::to_owned)
        .collect();
    for key in stale {
        item.remove(txn, &key);
    }
    for (key, value) in fields {
        let same = matches!(item.get(txn, &key), Some(Out::Any(current)) if current == value);
        if !same {
            item.insert(txn, key, value);
        }
    }
}

/// Gives the items, listed in their wanted order, order keys that sort that
/// way, changing as few as it can.
fn write_order(txn: &mut TransactionMut<'_>, items: &[(&MapRef, &str)]) {
    let maps: Vec<Option<MapRef>> = (items.iter())
        .map(|(map, id)| match map.get(txn, id) {
            Some(Out::YMap(item)) => Some(item),
            _ => None,
        })
        .collect();
    let keys: Vec<Option<String>> = (maps.iter())
        .map(|item| match item.as_ref()?.get(txn, ORDER) {
            Some(Out::Any(Any::String(key))) => Some(key.to_string()),
            _ => None,
        })
        .collect();
    let current: Vec<Option<&str>> = keys.iter().map(Option::as_deref).collect();
    for (index, key) in order::assign(&current) {
        if let Some(item) = &maps[index] {
            item.insert(txn, ORDER, key);
        }
    }
}

fn read_fields<T: ReadTxn>(txn: &T, map: &MapRef, id: &str) -> Option<Fields> {
    let Some(Out::YMap(item)) = map.get(txn, id) else {
        return None;
    };
    Some(
        item.iter(txn)
            .filter_map(|(key, value)| match value {
                Out::Any(any) => Some((key.to_owned(), any)),
                _ => None,
            })
            .collect(),
    )
}

fn order_key<T: ReadTxn>(txn: &T, map: &MapRef, id: &str) -> Option<String> {
    let Some(Out::YMap(item)) = map.get(txn, id) else {
        return None;
    };
    match item.get(txn, ORDER) {
        Some(Out::Any(Any::String(key))) => Some(key.to_string()),
        _ => None,
    }
}

/// Watches one root map and marks what each transaction changed in it.
fn observe(
    root: &MapRef,
    dirty: &Arc<Mutex<Dirty>>,
    ids: fn(&mut Dirty) -> &mut BTreeSet<String>,
    reordered: fn(&mut Dirty),
) {
    let dirty = Arc::clone(dirty);
    root.observe_deep("specular:dirty", move |txn, events| {
        let mut dirty = lock(&dirty);
        for event in events.iter() {
            let path = event.path();
            match path.front() {
                // A change to the root map itself: items added or removed.
                None => {
                    if let Event::Map(event) = event {
                        for key in event.keys(txn).keys() {
                            ids(&mut dirty).insert(key.to_string());
                        }
                        reordered(&mut dirty);
                    }
                }
                // A change inside one item.
                Some(PathSegment::Key(id)) => {
                    ids(&mut dirty).insert(id.to_string());
                    if let Event::Map(event) = event
                        && event.keys(txn).contains_key(ORDER)
                    {
                        reordered(&mut dirty);
                    }
                }
                Some(PathSegment::Index(_)) => {}
            }
        }
    });
}

/// Where `old` and `new` differ: the byte offset of the first difference,
/// how many bytes of `old` go, and what of `new` takes their place. Offsets
/// fall on character boundaries.
fn changed_span<'a>(old: &str, new: &'a str) -> (usize, usize, &'a str) {
    let prefix: usize = (old.chars().zip(new.chars()))
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum();
    let suffix: usize = (old[prefix..].chars().rev().zip(new[prefix..].chars().rev()))
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum();
    let removed = old.len() - prefix - suffix;
    (prefix, removed, &new[prefix..new.len() - suffix])
}

fn utf16_len(text: &str) -> u32 {
    u32::try_from(text.encode_utf16().count()).unwrap_or(u32::MAX)
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
