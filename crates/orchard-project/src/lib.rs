#![forbid(unsafe_code)]

//! Project history and persistence contracts.

pub mod asset;
pub mod foundry;

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use orchard_core_legacy::{
    EditProgram, RevisionId, ShapeDocument, ValidationReport, validate_document,
};
use orchard_search_internal::Candidate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const PROJECT_SCHEMA_VERSION: u32 = 1;
const DOCUMENT_SCHEMA_VERSION: u32 = 1;
const ROOT_REVISION_ID: RevisionId = RevisionId(0);
const PROJECT_TEMP_PREFIX: &str = ".object-orchard-project-";
const TEMP_FILE_SUFFIX: &str = ".tmp";
const OBSOLETE_TEMP_MIN_AGE: Duration = Duration::from_secs(60 * 60);
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

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

/// Object Orchard project file.
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
    /// Project schema version is newer than this build understands.
    #[error("project schema version {found} is newer than supported version {supported}")]
    FutureSchemaVersion {
        /// Version found in the file.
        found: u32,
        /// Newest version supported by this build.
        supported: u32,
    },
    /// The project revision graph is malformed.
    #[error("invalid project: {0}")]
    InvalidProject(String),
    /// A document snapshot failed orchard-core-legacy validation.
    #[error("invalid shape document in revision {revision:?}")]
    InvalidDocument {
        /// Revision containing the invalid snapshot, if known.
        revision: Option<RevisionId>,
        /// Full validation report from orchard-core-legacy.
        report: ValidationReport,
    },
    /// Allocating a new revision ID would overflow.
    #[error("revision id overflow after {0}")]
    RevisionIdOverflow(u64),
    /// Serialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// Serialization failed for a specific path.
    #[error("json error while reading {}: {source}", path.display())]
    JsonAtPath {
        /// Path being read.
        path: PathBuf,
        /// Source error.
        #[source]
        source: serde_json::Error,
    },
    /// I/O failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// I/O failed for a specific path.
    #[error("io error while {action} {}: {source}", path.display())]
    PathIo {
        /// Action being performed.
        action: &'static str,
        /// Path being accessed.
        path: PathBuf,
        /// Source error.
        #[source]
        source: std::io::Error,
    },
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
        ensure_supported_project_schema(self.schema_version)?;
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
        atomic_project_write(path.as_ref(), &project_json_bytes(self)?)
    }

    /// Load JSON from disk.
    pub fn load_json(path: impl AsRef<Path>) -> Result<Self, ProjectError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| project_path_io("reading", path, source))?;
        ensure_supported_project_schema(project_schema_version(&bytes, path)?)?;
        let project: Self =
            serde_json::from_slice(&bytes).map_err(|source| ProjectError::JsonAtPath {
                path: path.to_path_buf(),
                source,
            })?;
        project.validate()?;
        Ok(project)
    }
}

#[derive(Debug, Deserialize)]
struct ProjectSchemaProbe {
    schema_version: u32,
}

fn ensure_supported_project_schema(schema_version: u32) -> Result<(), ProjectError> {
    if schema_version == PROJECT_SCHEMA_VERSION {
        Ok(())
    } else if schema_version > PROJECT_SCHEMA_VERSION {
        Err(ProjectError::FutureSchemaVersion {
            found: schema_version,
            supported: PROJECT_SCHEMA_VERSION,
        })
    } else {
        Err(ProjectError::UnsupportedSchemaVersion(schema_version))
    }
}

fn project_schema_version(bytes: &[u8], path: &Path) -> Result<u32, ProjectError> {
    let probe: ProjectSchemaProbe =
        serde_json::from_slice(bytes).map_err(|source| ProjectError::JsonAtPath {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(probe.schema_version)
}

fn project_json_bytes(project: &Project) -> Result<Vec<u8>, ProjectError> {
    let mut bytes = serde_json::to_vec_pretty(project)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn atomic_project_write(path: &Path, bytes: &[u8]) -> Result<(), ProjectError> {
    atomic_project_replace(path, |file| file.write_all(bytes))
}

fn atomic_project_replace(
    path: &Path,
    write_temp: impl FnOnce(&mut File) -> io::Result<()>,
) -> Result<(), ProjectError> {
    cleanup_obsolete_project_temp_files(path);

    let mut temp = TempSibling::create(path)?;

    write_temp(temp.file_mut())
        .map_err(|source| project_path_io("writing temporary project file for", path, source))?;
    temp.file_mut()
        .sync_all()
        .map_err(|source| project_path_io("flushing temporary project file for", path, source))?;
    temp.persist(path)
        .map_err(|source| project_path_io("replacing", path, source))?;

    cleanup_obsolete_project_temp_files(path);
    Ok(())
}

struct TempSibling {
    path: PathBuf,
    file: Option<File>,
    persisted: bool,
}

impl TempSibling {
    fn create(target: &Path) -> Result<Self, ProjectError> {
        let parent = sibling_directory(target);
        let prefix = project_temp_prefix(target);
        let process_id = process::id();

        for _ in 0..100 {
            let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!("{prefix}{process_id}-{counter}{TEMP_FILE_SUFFIX}"));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    return Ok(Self {
                        path,
                        file: Some(file),
                        persisted: false,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(project_path_io(
                        "creating temporary project file for",
                        target,
                        error,
                    ));
                }
            }
        }

        Err(project_path_io(
            "creating temporary project file for",
            target,
            io::Error::new(
                io::ErrorKind::AlreadyExists,
                "could not allocate a unique temporary filename",
            ),
        ))
    }

    fn file_mut(&mut self) -> &mut File {
        self.file
            .as_mut()
            .expect("temporary file handle must remain open until persist")
    }

    fn persist(mut self, target: &Path) -> io::Result<()> {
        drop(self.file.take());
        fs::rename(&self.path, target)?;
        self.persisted = true;
        Ok(())
    }
}

impl Drop for TempSibling {
    fn drop(&mut self) {
        if !self.persisted {
            drop(self.file.take());
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn cleanup_obsolete_project_temp_files(path: &Path) {
    let prefix = project_temp_prefix(path);
    let Ok(entries) = fs::read_dir(sibling_directory(path)) else {
        return;
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        if !file_name.starts_with(&prefix) || !file_name.ends_with(TEMP_FILE_SUFFIX) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_file() && obsolete_temp_metadata(&metadata) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn obsolete_temp_metadata(metadata: &fs::Metadata) -> bool {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age >= OBSOLETE_TEMP_MIN_AGE)
}

fn sibling_directory(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn project_temp_prefix(path: &Path) -> String {
    format!("{PROJECT_TEMP_PREFIX}{}-", path_file_fragment(path))
}

fn path_file_fragment(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(safe_file_fragment)
        .filter(|fragment| !fragment.is_empty())
        .unwrap_or_else(|| "untitled".to_owned())
}

fn safe_file_fragment(value: &str) -> String {
    let mut fragment = String::new();
    let mut pending_separator = false;

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            if pending_separator && !fragment.is_empty() {
                fragment.push('-');
            }
            fragment.push(character.to_ascii_lowercase());
            pending_separator = false;
        } else if !fragment.is_empty() {
            pending_separator = true;
        }

        if fragment.len() >= 48 {
            break;
        }
    }

    fragment.trim_matches('-').to_owned()
}

fn project_path_io(action: &'static str, path: &Path, source: io::Error) -> ProjectError {
    ProjectError::PathIo {
        action,
        path: path.to_path_buf(),
        source,
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
    use std::io::{self, Write};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use orchard_core_legacy::{
        CandidateId, EditProgram, NodeId, PrimitiveKind, RevisionId, ShapeDocument, ShapeNode,
        Transform3,
    };
    use orchard_search_internal::{Candidate, ShapeDescriptor};

    use super::{
        Project, ProjectError, atomic_project_replace, project_json_bytes, project_temp_prefix,
    };

    fn test_document(title: &str, radius: f32) -> ShapeDocument {
        ShapeDocument::new(
            title,
            ShapeNode {
                id: NodeId(1),
                name: "Root sphere".to_owned(),
                tags: Default::default(),
                enabled: true,
                transform: Transform3::default(),
                kind: orchard_core_legacy::NodeKind::Primitive(PrimitiveKind::Sphere { radius }),
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
        std::env::temp_dir().join(format!("object-orchard-{name}-{nonce}.json"))
    }

    struct TestTempDir {
        path: PathBuf,
    }

    impl TestTempDir {
        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestTempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn tempdir() -> TestTempDir {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!("object-orchard-project-tests-{nonce}"));
        fs::create_dir(&path).unwrap();
        TestTempDir { path }
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
    fn deterministic_json_formatting_is_stable() {
        let mut project = Project::new("Test", test_document("Initial", 1.0));
        project
            .accept_candidate(candidate("First", 1.1, 1))
            .unwrap();

        let first = project_json_bytes(&project).unwrap();
        let second = project_json_bytes(&project).unwrap();

        assert_eq!(first, second);
        assert!(first.ends_with(b"\n"));
    }

    #[test]
    fn save_json_atomically_replaces_existing_file() {
        let temp_dir = tempdir();
        let path = temp_dir.path().join("project.shapelab.json");
        let first = Project::new("First", test_document("Initial", 1.0));
        let second = Project::new("Second", test_document("Updated", 1.4));

        first.save_json(&path).unwrap();
        second.save_json(&path).unwrap();

        assert_eq!(Project::load_json(&path).unwrap(), second);
    }

    #[test]
    fn interrupted_temp_write_preserves_existing_project_file() {
        let temp_dir = tempdir();
        let path = temp_dir.path().join("project.shapelab.json");
        let original = Project::new("Original", test_document("Initial", 1.0));
        original.save_json(&path).unwrap();
        let original_bytes = fs::read(&path).unwrap();

        let error = atomic_project_replace(&path, |file| {
            file.write_all(b"{\"schema_version\":1,\"title\":\"partial\"")?;
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "simulated interrupted write",
            ))
        })
        .unwrap_err();

        assert!(matches!(error, ProjectError::PathIo { .. }));
        assert_eq!(fs::read(&path).unwrap(), original_bytes);
        assert_eq!(Project::load_json(&path).unwrap(), original);
    }

    #[test]
    fn failed_replacement_keeps_temporary_file_out_of_target() {
        let temp_dir = tempdir();
        let path = temp_dir.path().join("project-directory.shapelab.json");
        fs::create_dir(&path).unwrap();

        let error = Project::new("Test", test_document("Initial", 1.0))
            .save_json(&path)
            .unwrap_err();

        assert!(matches!(error, ProjectError::PathIo { .. }));
        assert!(path.is_dir());
    }

    #[test]
    fn malformed_and_unknown_schema_json_are_rejected() {
        let malformed = temp_json_path("malformed");
        fs::write(&malformed, b"{not valid json").unwrap();
        assert!(matches!(
            Project::load_json(&malformed),
            Err(ProjectError::JsonAtPath { .. })
        ));
        let _ = fs::remove_file(&malformed);

        let future_schema = temp_json_path("future-schema");
        let mut project = Project::new("Test", test_document("Initial", 1.0));
        project.schema_version = 2;
        fs::write(&future_schema, serde_json::to_vec(&project).unwrap()).unwrap();
        assert!(matches!(
            Project::load_json(&future_schema),
            Err(ProjectError::FutureSchemaVersion {
                found: 2,
                supported: 1
            })
        ));
        let _ = fs::remove_file(&future_schema);

        let old_schema = temp_json_path("old-schema");
        project.schema_version = 0;
        fs::write(&old_schema, serde_json::to_vec(&project).unwrap()).unwrap();
        assert!(matches!(
            Project::load_json(&old_schema),
            Err(ProjectError::UnsupportedSchemaVersion(0))
        ));
        let _ = fs::remove_file(&old_schema);
    }

    #[test]
    fn future_schema_rejects_before_shape_deserialization() {
        let temp_dir = tempdir();
        let path = temp_dir.path().join("future.shapelab.json");
        fs::write(
            &path,
            br#"{"schema_version":2,"future_payload":{"unknown":true}}"#,
        )
        .unwrap();

        assert!(matches!(
            Project::load_json(&path),
            Err(ProjectError::FutureSchemaVersion {
                found: 2,
                supported: 1
            })
        ));
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

    #[test]
    fn temp_file_prefix_is_scoped_to_target_name() {
        let temp_dir = tempdir();
        let path = temp_dir.path().join("Unsafe Project Name!.shapelab.json");

        assert_eq!(
            project_temp_prefix(&path),
            ".object-orchard-project-unsafe-project-name-shapelab-json-"
        );
    }
}
