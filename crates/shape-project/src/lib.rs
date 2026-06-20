#![forbid(unsafe_code)]

//! Project history and persistence contracts.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use shape_core::{EditProgram, RevisionId, ShapeDocument, ValidationReport, validate_document};
use shape_search::Candidate;
use thiserror::Error;

const PROJECT_SCHEMA_VERSION: u32 = 1;
const DOCUMENT_SCHEMA_VERSION: u32 = 1;
const ROOT_REVISION_ID: RevisionId = RevisionId(0);

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

/// Lightweight marker a UI can store after a successful save or load.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ProjectPersistenceMarker {
    /// Current revision when the marker was captured.
    pub current_revision: RevisionId,
    /// Next revision ID when the marker was captured.
    pub next_revision_id: u64,
    /// Number of revisions when the marker was captured.
    pub revision_count: usize,
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
    /// Project schema version is not supported.
    #[error("unsupported project schema version {0}")]
    UnsupportedSchemaVersion(u32),
    /// The project revision graph is malformed.
    #[error("invalid project: {0}")]
    InvalidProject(String),
    /// A document snapshot failed shape-core validation.
    #[error("invalid shape document in revision {revision:?}")]
    InvalidDocument {
        /// Revision containing the invalid snapshot, if known.
        revision: Option<RevisionId>,
        /// Full validation report from shape-core.
        report: ValidationReport,
    },
    /// Allocating a new revision ID would overflow.
    #[error("revision id overflow after {0}")]
    RevisionIdOverflow(u64),
    /// Serialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// I/O failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl Project {
    /// Create a project from an initial document.
    ///
    /// The established contract returns `Self`; callers that need fallible
    /// validation before construction can use [`Project::try_new`].
    #[must_use]
    pub fn new(title: impl Into<String>, document: ShapeDocument) -> Self {
        let title = title.into();
        let mut revisions = BTreeMap::new();
        revisions.insert(
            ROOT_REVISION_ID,
            Revision {
                id: ROOT_REVISION_ID,
                parent: None,
                label: "Initial".to_owned(),
                edit: None,
                document,
            },
        );
        Self {
            schema_version: PROJECT_SCHEMA_VERSION,
            title,
            current_revision: ROOT_REVISION_ID,
            next_revision_id: 1,
            revisions,
        }
    }

    /// Create a project and reject an invalid initial document.
    pub fn try_new(
        title: impl Into<String>,
        document: ShapeDocument,
    ) -> Result<Self, ProjectError> {
        ensure_valid_document(&document, Some(ROOT_REVISION_ID))?;
        let project = Self::new(title, document);
        project.validate()?;
        Ok(project)
    }

    /// Return the current revision.
    pub fn current(&self) -> Result<&Revision, ProjectError> {
        self.revisions
            .get(&self.current_revision)
            .ok_or(ProjectError::UnknownRevision(self.current_revision))
    }

    /// Return the current document snapshot.
    pub fn current_document(&self) -> Result<&ShapeDocument, ProjectError> {
        Ok(&self.current()?.document)
    }

    /// Return true when the current revision has a parent.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.revisions
            .get(&self.current_revision)
            .and_then(|revision| revision.parent)
            .is_some()
    }

    /// Return the current revision path, starting at current and ending at root.
    pub fn revision_path_to_root(&self) -> Result<Vec<RevisionId>, ProjectError> {
        self.revision_path_to_root_from(self.current_revision)
    }

    /// Return a revision path, starting at `revision` and ending at root.
    pub fn revision_path_to_root_from(
        &self,
        revision: RevisionId,
    ) -> Result<Vec<RevisionId>, ProjectError> {
        let mut path = Vec::new();
        let mut cursor = revision;
        loop {
            let current = self
                .revisions
                .get(&cursor)
                .ok_or(ProjectError::UnknownRevision(cursor))?;
            path.push(cursor);
            let Some(parent) = current.parent else {
                break;
            };
            cursor = parent;
        }
        Ok(path)
    }

    /// Return a marker that can be compared later to detect unsaved changes.
    #[must_use]
    pub fn persistence_marker(&self) -> ProjectPersistenceMarker {
        ProjectPersistenceMarker {
            current_revision: self.current_revision,
            next_revision_id: self.next_revision_id,
            revision_count: self.revisions.len(),
        }
    }

    /// Return true when this project differs from a previously captured marker.
    #[must_use]
    pub fn is_dirty_since(&self, marker: ProjectPersistenceMarker) -> bool {
        self.persistence_marker() != marker
    }

    /// Accept a candidate and append a child revision.
    pub fn accept_candidate(&mut self, candidate: Candidate) -> Result<RevisionId, ProjectError> {
        self.current()?;
        ensure_valid_document(&candidate.document, None)?;
        let id = RevisionId(self.next_revision_id);
        if self.revisions.contains_key(&id) {
            return Err(ProjectError::InvalidProject(format!(
                "next revision id {id:?} already exists"
            )));
        }
        let next_revision_id = self
            .next_revision_id
            .checked_add(1)
            .ok_or(ProjectError::RevisionIdOverflow(self.next_revision_id))?;
        let label = revision_label(&candidate);
        self.revisions.insert(
            id,
            Revision {
                id,
                parent: Some(self.current_revision),
                label,
                edit: Some(candidate.edit),
                document: candidate.document,
            },
        );
        self.current_revision = id;
        self.next_revision_id = next_revision_id;
        Ok(id)
    }

    /// Move to the parent revision.
    pub fn undo(&mut self) -> Result<RevisionId, ProjectError> {
        let parent = self.current()?.parent.ok_or(ProjectError::NoParent)?;
        self.current_revision = parent;
        Ok(parent)
    }

    /// Return direct children of a revision in stable `RevisionId` order.
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

    /// Validate project-level and document-level invariants.
    pub fn validate(&self) -> Result<(), ProjectError> {
        if self.schema_version != PROJECT_SCHEMA_VERSION {
            return Err(ProjectError::UnsupportedSchemaVersion(self.schema_version));
        }
        if self.revisions.is_empty() {
            return Err(ProjectError::InvalidProject(
                "revision graph must contain revision 0".to_owned(),
            ));
        }
        let Some(root) = self.revisions.get(&ROOT_REVISION_ID) else {
            return Err(ProjectError::InvalidProject(
                "revision graph is missing revision 0".to_owned(),
            ));
        };
        if root.parent.is_some() {
            return Err(ProjectError::InvalidProject(
                "revision 0 must not have a parent".to_owned(),
            ));
        }
        if !self.revisions.contains_key(&self.current_revision) {
            return Err(ProjectError::UnknownRevision(self.current_revision));
        }

        let mut max_revision_id = 0;
        for (id, revision) in &self.revisions {
            if revision.id != *id {
                return Err(ProjectError::InvalidProject(format!(
                    "revision map key {id:?} does not match stored id {:?}",
                    revision.id
                )));
            }
            max_revision_id = max_revision_id.max(id.0);
            if *id != ROOT_REVISION_ID && revision.parent.is_none() {
                return Err(ProjectError::InvalidProject(format!(
                    "non-root revision {id:?} must have a parent"
                )));
            }
            if let Some(parent) = revision.parent {
                if !self.revisions.contains_key(&parent) {
                    return Err(ProjectError::UnknownRevision(parent));
                }
                if parent.0 >= id.0 {
                    return Err(ProjectError::InvalidProject(format!(
                        "revision {id:?} parent {parent:?} does not preserve monotonic ids"
                    )));
                }
            }
            ensure_valid_document(&revision.document, Some(*id))?;
        }

        if self.next_revision_id <= max_revision_id {
            return Err(ProjectError::InvalidProject(format!(
                "next revision id {} must be greater than existing max {max_revision_id}",
                self.next_revision_id
            )));
        }

        Ok(())
    }

    /// Save JSON to disk.
    pub fn save_json(&self, path: impl AsRef<Path>) -> Result<(), ProjectError> {
        self.validate()?;
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Load JSON from disk.
    pub fn load_json(path: impl AsRef<Path>) -> Result<Self, ProjectError> {
        let project: Self = serde_json::from_slice(&std::fs::read(path)?)?;
        project.validate()?;
        Ok(project)
    }
}

fn revision_label(candidate: &Candidate) -> String {
    let trimmed = candidate.edit.label.trim();
    if trimmed.is_empty() {
        format!("Candidate {}", candidate.id.0)
    } else {
        trimmed.to_owned()
    }
}

fn ensure_valid_document(
    document: &ShapeDocument,
    revision: Option<RevisionId>,
) -> Result<(), ProjectError> {
    if document.schema_version != DOCUMENT_SCHEMA_VERSION {
        return Err(ProjectError::InvalidProject(format!(
            "revision {revision:?} contains unsupported document schema version {}",
            document.schema_version
        )));
    }
    let report = validate_document(document);
    if report.is_valid() {
        Ok(())
    } else {
        Err(ProjectError::InvalidDocument { revision, report })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use shape_core::{
        CandidateId, EditProgram, NodeId, PrimitiveKind, RevisionId, ShapeDocument, ShapeNode,
        Transform3,
    };
    use shape_search::{Candidate, ShapeDescriptor};

    use super::{Project, ProjectError};

    fn test_document(title: &str, radius: f32) -> ShapeDocument {
        ShapeDocument::new(
            title,
            ShapeNode {
                id: NodeId(1),
                name: "Root sphere".to_owned(),
                tags: Default::default(),
                enabled: true,
                transform: Transform3::default(),
                kind: shape_core::NodeKind::Primitive(PrimitiveKind::Sphere { radius }),
            },
        )
    }

    fn edit(label: &str, seed: u64) -> EditProgram {
        EditProgram {
            label: label.to_owned(),
            seed,
            operations: Vec::new(),
        }
    }

    fn candidate(label: &str, radius: f32, id: u64) -> Candidate {
        Candidate {
            id: CandidateId(id),
            document: test_document(label, radius),
            edit: edit(label, id),
            descriptor: ShapeDescriptor {
                values: vec![radius],
            },
            distance_from_parent: radius,
        }
    }

    fn temp_json_path(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("shape-lab-{name}-{nonce}.json"))
    }

    #[test]
    fn new_project_starts_at_revision_zero() {
        let project = Project::new("Test", test_document("Initial", 1.0));

        assert_eq!(project.schema_version, 1);
        assert_eq!(project.current_revision, RevisionId(0));
        assert_eq!(project.next_revision_id, 1);
        assert_eq!(project.current().unwrap().id, RevisionId(0));
        assert_eq!(project.current_document().unwrap().title, "Initial");
        assert!(!project.can_undo());
        assert!(project.validate().is_ok());
    }

    #[test]
    fn accept_candidate_creates_current_child_revision() {
        let mut project = Project::new("Test", test_document("Initial", 1.0));

        let revision = project
            .accept_candidate(candidate("Broader base", 1.2, 7))
            .unwrap();

        assert_eq!(revision, RevisionId(1));
        assert_eq!(project.current_revision, RevisionId(1));
        assert_eq!(project.next_revision_id, 2);
        assert_eq!(project.children_of(RevisionId(0)), vec![RevisionId(1)]);
        assert_eq!(
            project.current().unwrap().edit.as_ref().unwrap().label,
            "Broader base"
        );
        assert!(project.can_undo());
    }

    #[test]
    fn undo_moves_to_parent_and_preserves_children() {
        let mut project = Project::new("Test", test_document("Initial", 1.0));
        project
            .accept_candidate(candidate("First", 1.1, 1))
            .unwrap();

        let parent = project.undo().unwrap();

        assert_eq!(parent, RevisionId(0));
        assert_eq!(project.current_revision, RevisionId(0));
        assert_eq!(project.children_of(RevisionId(0)), vec![RevisionId(1)]);
        assert!(matches!(project.undo(), Err(ProjectError::NoParent)));
    }

    #[test]
    fn accepting_after_undo_creates_multi_branch_history() {
        let mut project = Project::new("Test", test_document("Initial", 1.0));
        let first = project
            .accept_candidate(candidate("First", 1.1, 1))
            .unwrap();
        project.undo().unwrap();
        let second = project
            .accept_candidate(candidate("Second", 1.3, 2))
            .unwrap();

        assert_eq!(first, RevisionId(1));
        assert_eq!(second, RevisionId(2));
        assert_eq!(
            project.children_of(RevisionId(0)),
            vec![RevisionId(1), RevisionId(2)]
        );
        assert_eq!(project.next_revision_id, 3);
    }

    #[test]
    fn switching_branches_rejects_missing_revisions() {
        let mut project = Project::new("Test", test_document("Initial", 1.0));
        let first = project
            .accept_candidate(candidate("First", 1.1, 1))
            .unwrap();
        project.undo().unwrap();
        let second = project
            .accept_candidate(candidate("Second", 1.3, 2))
            .unwrap();

        project.switch_to(first).unwrap();
        assert_eq!(project.current_revision, first);
        assert_eq!(
            project.revision_path_to_root().unwrap(),
            vec![RevisionId(1), RevisionId(0)]
        );
        project.switch_to(second).unwrap();
        assert_eq!(project.current_revision, second);
        assert!(matches!(
            project.switch_to(RevisionId(99)),
            Err(ProjectError::UnknownRevision(RevisionId(99)))
        ));
    }

    #[test]
    fn persistence_marker_detects_history_changes() {
        let mut project = Project::new("Test", test_document("Initial", 1.0));
        let marker = project.persistence_marker();

        assert!(!project.is_dirty_since(marker));
        project
            .accept_candidate(candidate("First", 1.1, 1))
            .unwrap();
        assert!(project.is_dirty_since(marker));
    }

    #[test]
    fn json_round_trip_preserves_exact_revision_graph() {
        let path = temp_json_path("round-trip");
        let mut project = Project::new("Test", test_document("Initial", 1.0));
        project
            .accept_candidate(candidate("First", 1.1, 1))
            .unwrap();
        project.undo().unwrap();
        project
            .accept_candidate(candidate("Second", 1.3, 2))
            .unwrap();

        project.save_json(&path).unwrap();
        let loaded = Project::load_json(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(loaded, project);
    }

    #[test]
    fn malformed_and_unknown_schema_json_are_rejected() {
        let malformed = temp_json_path("malformed");
        fs::write(&malformed, b"{not valid json").unwrap();
        assert!(matches!(
            Project::load_json(&malformed),
            Err(ProjectError::Json(_))
        ));
        let _ = fs::remove_file(&malformed);

        let unknown_schema = temp_json_path("unknown-schema");
        let mut project = Project::new("Test", test_document("Initial", 1.0));
        project.schema_version = 99;
        fs::write(&unknown_schema, serde_json::to_vec(&project).unwrap()).unwrap();
        assert!(matches!(
            Project::load_json(&unknown_schema),
            Err(ProjectError::UnsupportedSchemaVersion(99))
        ));
        let _ = fs::remove_file(&unknown_schema);
    }

    #[test]
    fn loaded_project_rejects_non_monotonic_revision_graph() {
        let path = temp_json_path("bad-graph");
        let mut project = Project::new("Test", test_document("Initial", 1.0));
        project
            .accept_candidate(candidate("First", 1.1, 1))
            .unwrap();
        project.next_revision_id = 1;
        fs::write(&path, serde_json::to_vec(&project).unwrap()).unwrap();

        assert!(matches!(
            Project::load_json(&path),
            Err(ProjectError::InvalidProject(_))
        ));
        let _ = fs::remove_file(&path);
    }
}
