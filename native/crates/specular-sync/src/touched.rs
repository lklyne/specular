//! Which items to copy into the shared document: the ones a command
//! touched, or the ones the document changed behind the sync's back.

use std::collections::{BTreeSet, HashMap};

use specular_doc::{
    Annotation, AnnotationId, Command, Document, Edge, EdgeId, Entity, EntityId, ItemId,
};

/// Items to copy from the document into the shared document.
#[derive(Debug, Default)]
pub(crate) struct Touched {
    pub(crate) entities: BTreeSet<EntityId>,
    pub(crate) edges: BTreeSet<EdgeId>,
    pub(crate) annotations: BTreeSet<AnnotationId>,
    pub(crate) notes: BTreeSet<String>,
    pub(crate) order: bool,
    pub(crate) annotation_order: bool,
}

impl Touched {
    pub(crate) fn everything(document: &Document) -> Self {
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

    pub(crate) fn by(command: &Command) -> Self {
        let mut touched = Self::default();
        touched.add(command);
        touched
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

impl Touched {
    pub(crate) fn is_empty(&self) -> bool {
        self.entities.is_empty()
            && self.edges.is_empty()
            && self.annotations.is_empty()
            && self.notes.is_empty()
            && !self.order
            && !self.annotation_order
    }
}

/// The document as the shared document last saw it, so that a change made
/// straight to the document, outside any step or record, can be found by
/// comparing. Items compare as structs, so finding nothing changed costs a
/// walk of the document and no serialising.
#[derive(Debug, Default)]
pub(crate) struct Shadow {
    entities: HashMap<EntityId, Entity>,
    edges: HashMap<EdgeId, Edge>,
    annotations: Vec<Annotation>,
    notes: HashMap<String, String>,
    order: Vec<ItemId>,
}

impl Shadow {
    pub(crate) fn of(document: &Document) -> Self {
        let mut shadow = Self::default();
        shadow.update(document, &Touched::everything(document));
        shadow
    }

    /// What differs between `document` and the shadow.
    pub(crate) fn changes(&self, document: &Document) -> Touched {
        let mut touched = Touched::default();
        for entity in document.entities() {
            if self.entities.get(&entity.id) != Some(entity) {
                touched.entities.insert(entity.id.clone());
            }
        }
        for id in self.entities.keys() {
            if document.entity(id).is_none() {
                touched.entities.insert(id.clone());
            }
        }
        for edge in document.edges() {
            if self.edges.get(&edge.id) != Some(edge) {
                touched.edges.insert(edge.id.clone());
            }
        }
        for id in self.edges.keys() {
            if document.edge(id).is_none() {
                touched.edges.insert(id.clone());
            }
        }
        touched.order = self.order != document.order();
        if self.annotations != document.annotations() {
            for annotation in document.annotations() {
                let seen = self.annotations.iter().find(|a| a.id == annotation.id);
                if seen != Some(annotation) {
                    touched.annotations.insert(annotation.id.clone());
                }
            }
            for annotation in &self.annotations {
                if document.annotation(&annotation.id).is_none() {
                    touched.annotations.insert(annotation.id.clone());
                }
            }
            touched.annotation_order = true;
        }
        for (file, text) in document.notes() {
            if self.notes.get(file).map(String::as_str) != Some(text) {
                touched.notes.insert(file.to_owned());
            }
        }
        for file in self.notes.keys() {
            if document.note(file).is_none() {
                touched.notes.insert(file.clone());
            }
        }
        touched
    }

    /// Copies the `touched` items from `document`.
    pub(crate) fn update(&mut self, document: &Document, touched: &Touched) {
        for id in &touched.entities {
            match document.entity(id) {
                Some(entity) => self.entities.insert(id.clone(), entity.clone()),
                None => self.entities.remove(id),
            };
        }
        for id in &touched.edges {
            match document.edge(id) {
                Some(edge) => self.edges.insert(id.clone(), edge.clone()),
                None => self.edges.remove(id),
            };
        }
        if !touched.annotations.is_empty() || touched.annotation_order {
            self.annotations = document.annotations().to_vec();
        }
        for file in &touched.notes {
            match document.note(file) {
                Some(text) => self.notes.insert(file.clone(), text.to_owned()),
                None => self.notes.remove(file),
            };
        }
        if touched.order {
            self.order = document.order().to_vec();
        }
    }
}
