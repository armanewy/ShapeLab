#![forbid(unsafe_code)]

//! Project history and persistence contracts.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use shape_core::{EditProgram, RevisionId, ShapeDocument};
use shape_search::Candidate;
use thiserror::Error;

/// One revision in a branchable project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Revision {
    /// Stable revision ID.
    pub id: RevisionId,
    /// Parent revision.
    pub parent: Option<RevisionId>,
    /// Human-facing label.
    pub label: String,
    /// Edit that produced this revision.
    pub edit: Option<EditProgram>,
    /// Complete document snapshot.
    pub document: ShapeDocument,
}

/// Shape Lab project file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    /// Project schema version.
    pub schema_version: u32,
    /// Project title.
    pub title: String,
    /// Current revision.
    pub current_revision: RevisionId,
    /// Next revision ID.
    pub next_revision_id: u64,
    /// Revision graph.
    pub revisions: BTreeMap<RevisionId, Revision>,
}

/// Project errors.
#[derive(Debug, Error)]
pub enum ProjectError {
    /// Revision not found.
    #[error("unknown revision {0:?}")]
    UnknownRevision(RevisionId),
    /// There is no parent to undo to.
    #[error("current revision has no parent")]
    NoParent,
    /// Serialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// I/O failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl Project {
    /// Create a project from an initial document.
    #[must_use]
    pub fn new(title: impl Into<String>, document: ShapeDocument) -> Self {
        let root = RevisionId(1);
        let title = title.into();
        let mut revisions = BTreeMap::new();
        revisions.insert(
            root,
            Revision {
                id: root,
                parent: None,
                label: "Initial".to_owned(),
                edit: None,
                document,
            },
        );
        Self {
            schema_version: 1,
            title,
            current_revision: root,
            next_revision_id: 2,
            revisions,
        }
    }

    /// Return the current revision.
    pub fn current(&self) -> Result<&Revision, ProjectError> {
        self.revisions
            .get(&self.current_revision)
            .ok_or(ProjectError::UnknownRevision(self.current_revision))
    }

    /// Accept a candidate and append a revision.
    pub fn accept_candidate(&mut self, candidate: Candidate) -> Result<RevisionId, ProjectError> {
        let id = RevisionId(self.next_revision_id);
        self.next_revision_id = self.next_revision_id.saturating_add(1);
        self.revisions.insert(
            id,
            Revision {
                id,
                parent: Some(self.current_revision),
                label: candidate.edit.label.clone(),
                edit: Some(candidate.edit),
                document: candidate.document,
            },
        );
        self.current_revision = id;
        Ok(id)
    }

    /// Move to the parent revision.
    pub fn undo(&mut self) -> Result<RevisionId, ProjectError> {
        let parent = self.current()?.parent.ok_or(ProjectError::NoParent)?;
        self.current_revision = parent;
        Ok(parent)
    }

    /// Return direct children of a revision.
    #[must_use]
    pub fn children_of(&self, revision: RevisionId) -> Vec<RevisionId> {
        self.revisions
            .values()
            .filter(|candidate| candidate.parent == Some(revision))
            .map(|candidate| candidate.id)
            .collect()
    }

    /// Switch to an existing revision.
    pub fn switch_to(&mut self, revision: RevisionId) -> Result<(), ProjectError> {
        if self.revisions.contains_key(&revision) {
            self.current_revision = revision;
            Ok(())
        } else {
            Err(ProjectError::UnknownRevision(revision))
        }
    }

    /// Save JSON to disk.
    pub fn save_json(&self, path: impl AsRef<Path>) -> Result<(), ProjectError> {
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Load JSON from disk.
    pub fn load_json(path: impl AsRef<Path>) -> Result<Self, ProjectError> {
        Ok(serde_json::from_slice(&std::fs::read(path)?)?)
    }
}
