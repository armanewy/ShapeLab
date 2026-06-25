//! Branchable project persistence for semantic foundry projects.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use shape_asset::{AssetRecipe, RevisionId};
use shape_foundry::{
    CatalogContentRef, ControlValue, FOUNDRY_PROJECT_DOCUMENT_SCHEMA_VERSION, FOUNDRY_PROJECT_KIND,
    FoundryAssetDocument, FoundryBuildStamp, FoundryCatalogLock, FoundryCommand,
    FoundryConformanceSummary, FoundryEdit, FoundryProjectDocument, FoundryProjectRevision,
    FoundryProjectRevisionProgram, FoundryRecipeSnapshotError, FoundryValidationReport,
    GeneratedRecipeSnapshot, ProviderOverride, SHAPE_FOUNDRY_CRATE_VERSION, document_catalog_refs,
    validate_foundry_document, verify_catalog_content_fingerprint,
};
use thiserror::Error;

/// Required filename suffix for user-facing foundry project files.
pub const FOUNDRY_PROJECT_FILE_SUFFIX: &str = ".shapelab-foundry.json";

const ROOT_REVISION_ID: RevisionId = RevisionId(0);
const FOUNDRY_PROJECT_TEMP_PREFIX: &str = ".shape-lab-foundry-project-";
const TEMP_FILE_SUFFIX: &str = ".tmp";
const OBSOLETE_TEMP_MIN_AGE: Duration = Duration::from_secs(60 * 60);
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Branchable foundry project file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryProject {
    /// Replayable foundry project payload.
    #[serde(flatten)]
    pub document: FoundryProjectDocument,
}

impl Deref for FoundryProject {
    type Target = FoundryProjectDocument;

    fn deref(&self) -> &Self::Target {
        &self.document
    }
}

impl DerefMut for FoundryProject {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.document
    }
}

/// Marker a UI can compare later to detect unsaved project state.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FoundryProjectPersistenceMarker {
    /// Current revision when the marker was captured.
    pub current_revision: RevisionId,
    /// Next revision ID when the marker was captured.
    pub next_revision_id: u64,
    /// Number of revisions when the marker was captured.
    pub revision_count: usize,
}

/// Catalog/compiler/recipe inputs available during foundry project load.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FoundryProjectLoadContext {
    /// Exact catalog references currently available, keyed by semantic lock key.
    pub catalog_refs: BTreeMap<String, CatalogContentRef>,
    /// Exact generated recipes currently available, keyed by revision ID.
    pub available_recipes: BTreeMap<RevisionId, AssetRecipe>,
    /// Require every lock entry to be satisfied by current catalog refs or an embedded snapshot.
    pub require_catalog_inputs: bool,
    /// Current catalog format version, when known.
    pub current_catalog_version: Option<u32>,
    /// Current compiler version recorded in catalog locks, when known.
    pub current_compiler_version: Option<String>,
    /// Current shape-foundry crate version.
    pub current_foundry_version: Option<String>,
    /// Current shape-family-compile crate version, when known to the caller.
    pub current_family_compile_version: Option<String>,
}

impl FoundryProjectLoadContext {
    /// Create a load context using this build's known foundry version.
    #[must_use]
    pub fn current_runtime() -> Self {
        Self {
            current_foundry_version: Some(SHAPE_FOUNDRY_CRATE_VERSION.to_owned()),
            ..Self::default()
        }
    }
}

/// Load result including recovery and stale-build diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct FoundryProjectLoadOutcome {
    /// Loaded project.
    pub project: FoundryProject,
    /// Deterministic load verification report.
    pub report: FoundryProjectLoadReport,
}

/// Deterministic load verification report.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FoundryProjectLoadReport {
    /// True when at least one revision was opened using embedded catalog snapshots.
    pub read_only_recovery: bool,
    /// Revisions that require read-only recovery from embedded catalog snapshots.
    pub recovery_revisions: BTreeSet<RevisionId>,
    /// Revisions whose stored build should be considered stale.
    pub stale_builds: BTreeMap<RevisionId, Vec<FoundryBuildStaleReason>>,
    /// Revisions whose generated recipe snapshot was verified against available inputs.
    pub verified_recipe_revisions: Vec<RevisionId>,
}

/// Why a stored foundry build is stale relative to load-time inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum FoundryBuildStaleReason {
    /// The catalog format version changed.
    CatalogVersionChanged {
        /// Version recorded in the revision lock.
        stored: u32,
        /// Version available at load time.
        current: u32,
    },
    /// The compiler version recorded in the catalog lock changed.
    CatalogCompilerVersionChanged {
        /// Version recorded in the revision lock.
        stored: String,
        /// Version available at load time.
        current: String,
    },
    /// A locked catalog reference no longer matches the current catalog.
    CatalogReferenceChanged {
        /// Semantic lock key.
        key: String,
    },
    /// The shape-foundry crate version changed.
    FoundryVersionChanged {
        /// Version recorded in the build stamp.
        stored: String,
        /// Version available at load time.
        current: String,
    },
    /// The shape-family-compile crate version changed.
    FamilyCompileVersionChanged {
        /// Version recorded in the build stamp.
        stored: String,
        /// Version available at load time.
        current: String,
    },
}

/// Foundry project plus file-path and clean-state bookkeeping.
#[derive(Debug, Clone, PartialEq)]
pub struct FoundryProjectFile {
    /// Branchable project payload.
    pub project: FoundryProject,
    /// Current project path, when saved or loaded from disk.
    pub path: Option<PathBuf>,
    /// Load-time verification report.
    pub load_report: FoundryProjectLoadReport,
    clean_marker: Option<FoundryProjectPersistenceMarker>,
}

/// Recovery snapshot metadata returned after a snapshot is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundryRecoverySnapshot {
    /// Recovery snapshot file path.
    pub path: PathBuf,
    /// Project marker represented by the snapshot file.
    pub marker: FoundryProjectPersistenceMarker,
}

/// Foundry project persistence errors.
#[derive(Debug, Error)]
pub enum FoundryProjectError {
    /// Revision not found.
    #[error("unknown foundry revision {0:?}")]
    UnknownRevision(RevisionId),
    /// There is no parent revision to undo to.
    #[error("current foundry revision has no parent")]
    NoParent,
    /// Saving requires a file path.
    #[error("foundry project save requires a file path")]
    MissingSavePath,
    /// Project file does not use the foundry project filename suffix.
    #[error("foundry project path must end with {FOUNDRY_PROJECT_FILE_SUFFIX}: {}", path.display())]
    InvalidProjectPath {
        /// Invalid path.
        path: PathBuf,
    },
    /// Project kind marker is missing or unsupported.
    #[error("unsupported foundry project kind {found:?}; expected {expected}")]
    UnsupportedProjectKind {
        /// Found project kind.
        found: Option<String>,
        /// Expected project kind.
        expected: &'static str,
    },
    /// Project schema version is not supported.
    #[error("unsupported foundry project schema version {0}")]
    UnsupportedSchemaVersion(u32),
    /// Project schema version is newer than this build understands.
    #[error("foundry project schema version {found} is newer than supported version {supported}")]
    FutureSchemaVersion {
        /// Version found in the file.
        found: u32,
        /// Newest version supported by this build.
        supported: u32,
    },
    /// The project revision graph is malformed.
    #[error("invalid foundry project: {0}")]
    InvalidProject(String),
    /// A foundry document snapshot failed validation.
    #[error("invalid foundry document in revision {revision:?}")]
    InvalidDocument {
        /// Revision containing the invalid snapshot, if known.
        revision: Option<RevisionId>,
        /// Full validation report from shape-foundry.
        report: Box<FoundryValidationReport>,
    },
    /// A command cannot be replayed as a semantic document edit.
    #[error("cannot replay command {command} in revision {revision:?}: {reason}")]
    ReplayRejected {
        /// Revision whose program was rejected.
        revision: RevisionId,
        /// Stable command name.
        command: &'static str,
        /// Human-readable deterministic reason.
        reason: String,
    },
    /// Replaying a revision program did not reproduce the stored child document.
    #[error("replayed program did not reproduce foundry document in revision {0:?}")]
    ReplayMismatch(RevisionId),
    /// A current catalog reference did not match the revision lock and no recovery snapshot exists.
    #[error("catalog fingerprint mismatch in revision {revision:?} for key {key}")]
    CatalogFingerprintMismatch {
        /// Revision containing the mismatch.
        revision: RevisionId,
        /// Semantic lock key.
        key: String,
    },
    /// A required catalog input was unavailable and no recovery snapshot exists.
    #[error("missing catalog input in revision {revision:?} for key {key}")]
    MissingCatalogInput {
        /// Revision containing the missing input.
        revision: RevisionId,
        /// Semantic lock key.
        key: String,
    },
    /// A generated recipe input was available but the revision has no stored snapshot.
    #[error("missing generated recipe snapshot in revision {0:?}")]
    MissingRecipeSnapshot(RevisionId),
    /// A stored generated recipe snapshot did not match the available recipe.
    #[error("generated recipe snapshot mismatch in revision {0:?}")]
    RecipeSnapshotMismatch(RevisionId),
    /// Recipe snapshot serialization or fingerprinting failed.
    #[error("recipe snapshot error: {0}")]
    RecipeSnapshot(#[from] FoundryRecipeSnapshotError),
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

impl FoundryProject {
    /// Create a new branchable foundry project from an initial semantic document.
    pub fn new(
        title: impl Into<String>,
        document: FoundryAssetDocument,
        catalog_lock: FoundryCatalogLock,
        build_stamp: Option<FoundryBuildStamp>,
        recipe_snapshot: Option<GeneratedRecipeSnapshot>,
        conformance: FoundryConformanceSummary,
    ) -> Result<Self, FoundryProjectError> {
        let revision = build_revision(
            ROOT_REVISION_ID,
            None,
            "Initial",
            None,
            document,
            catalog_lock,
            build_stamp,
            recipe_snapshot,
            conformance,
        )?;
        let mut revisions = BTreeMap::new();
        revisions.insert(ROOT_REVISION_ID, revision);
        let project = Self {
            document: FoundryProjectDocument {
                project_kind: FOUNDRY_PROJECT_KIND.to_owned(),
                schema_version: FOUNDRY_PROJECT_DOCUMENT_SCHEMA_VERSION,
                title: title.into(),
                current_revision: ROOT_REVISION_ID,
                next_revision_id: 1,
                revisions,
            },
        };
        project.validate()?;
        Ok(project)
    }

    /// Return the current revision.
    pub fn current(&self) -> Result<&FoundryProjectRevision, FoundryProjectError> {
        self.revisions
            .get(&self.current_revision)
            .ok_or(FoundryProjectError::UnknownRevision(self.current_revision))
    }

    /// Return the current semantic document snapshot.
    pub fn current_document(&self) -> Result<&FoundryAssetDocument, FoundryProjectError> {
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

    /// Accept a foundry edit and append a child revision.
    pub fn accept_edit(
        &mut self,
        edit: FoundryEdit,
        document: FoundryAssetDocument,
        catalog_lock: FoundryCatalogLock,
        build_stamp: Option<FoundryBuildStamp>,
        recipe_snapshot: Option<GeneratedRecipeSnapshot>,
        conformance: FoundryConformanceSummary,
    ) -> Result<RevisionId, FoundryProjectError> {
        self.accept_program(
            FoundryProjectRevisionProgram::from_edit(edit),
            document,
            catalog_lock,
            build_stamp,
            recipe_snapshot,
            conformance,
        )
    }

    /// Accept a raw command program and append a child revision.
    #[allow(clippy::too_many_arguments)]
    pub fn accept_commands(
        &mut self,
        label: impl Into<String>,
        commands: Vec<FoundryCommand>,
        document: FoundryAssetDocument,
        catalog_lock: FoundryCatalogLock,
        build_stamp: Option<FoundryBuildStamp>,
        recipe_snapshot: Option<GeneratedRecipeSnapshot>,
        conformance: FoundryConformanceSummary,
    ) -> Result<RevisionId, FoundryProjectError> {
        self.accept_program(
            FoundryProjectRevisionProgram::from_commands(label, commands),
            document,
            catalog_lock,
            build_stamp,
            recipe_snapshot,
            conformance,
        )
    }

    /// Accept a revision program and append a child revision.
    pub fn accept_program(
        &mut self,
        program: FoundryProjectRevisionProgram,
        document: FoundryAssetDocument,
        catalog_lock: FoundryCatalogLock,
        build_stamp: Option<FoundryBuildStamp>,
        recipe_snapshot: Option<GeneratedRecipeSnapshot>,
        conformance: FoundryConformanceSummary,
    ) -> Result<RevisionId, FoundryProjectError> {
        let parent = self.current_revision;
        let parent_revision = self.current()?;
        let id = RevisionId(self.next_revision_id);
        if self.revisions.contains_key(&id) {
            return Err(FoundryProjectError::InvalidProject(format!(
                "next revision id {id:?} already exists"
            )));
        }
        let next_revision_id =
            self.next_revision_id
                .checked_add(1)
                .ok_or(FoundryProjectError::InvalidProject(format!(
                    "foundry revision id overflow after {}",
                    self.next_revision_id
                )))?;
        let label = revision_label(&program);
        let revision = build_revision(
            id,
            Some(parent),
            label,
            Some(program),
            document,
            catalog_lock,
            build_stamp,
            recipe_snapshot,
            conformance,
        )?;
        validate_revision_replay(parent_revision, &revision)?;
        self.revisions.insert(id, revision);
        self.current_revision = id;
        self.next_revision_id = next_revision_id;
        Ok(id)
    }

    /// Move to the parent revision without deleting child revisions.
    pub fn undo(&mut self) -> Result<RevisionId, FoundryProjectError> {
        let parent = self
            .current()?
            .parent
            .ok_or(FoundryProjectError::NoParent)?;
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

    /// Switch to an existing revision snapshot.
    pub fn switch_to(&mut self, revision: RevisionId) -> Result<(), FoundryProjectError> {
        if self.revisions.contains_key(&revision) {
            self.current_revision = revision;
            Ok(())
        } else {
            Err(FoundryProjectError::UnknownRevision(revision))
        }
    }

    /// Return the current revision path, starting at current and ending at root.
    pub fn revision_path_to_root(&self) -> Result<Vec<RevisionId>, FoundryProjectError> {
        self.revision_path_to_root_from(self.current_revision)
    }

    /// Return a revision path, starting at `revision` and ending at root.
    pub fn revision_path_to_root_from(
        &self,
        revision: RevisionId,
    ) -> Result<Vec<RevisionId>, FoundryProjectError> {
        let mut path = Vec::new();
        let mut cursor = revision;
        loop {
            let current = self
                .revisions
                .get(&cursor)
                .ok_or(FoundryProjectError::UnknownRevision(cursor))?;
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
    pub fn persistence_marker(&self) -> FoundryProjectPersistenceMarker {
        FoundryProjectPersistenceMarker {
            current_revision: self.current_revision,
            next_revision_id: self.next_revision_id,
            revision_count: self.revisions.len(),
        }
    }

    /// Return true when this project differs from a previously captured marker.
    #[must_use]
    pub fn is_dirty_since(&self, marker: FoundryProjectPersistenceMarker) -> bool {
        self.persistence_marker() != marker
    }

    /// Validate project-level, document-level, and replay invariants.
    pub fn validate(&self) -> Result<(), FoundryProjectError> {
        ensure_supported_project_kind(Some(self.project_kind.clone()))?;
        ensure_supported_project_schema(self.schema_version)?;
        if self.revisions.is_empty() {
            return Err(FoundryProjectError::InvalidProject(
                "revision graph must contain revision 0".to_owned(),
            ));
        }
        let Some(root) = self.revisions.get(&ROOT_REVISION_ID) else {
            return Err(FoundryProjectError::InvalidProject(
                "revision graph is missing revision 0".to_owned(),
            ));
        };
        if root.parent.is_some() {
            return Err(FoundryProjectError::InvalidProject(
                "revision 0 must not have a parent".to_owned(),
            ));
        }
        if !self.revisions.contains_key(&self.current_revision) {
            return Err(FoundryProjectError::UnknownRevision(self.current_revision));
        }

        let mut max_revision_id = 0;
        for (id, revision) in &self.revisions {
            if revision.id != *id {
                return Err(FoundryProjectError::InvalidProject(format!(
                    "revision map key {id:?} does not match stored id {:?}",
                    revision.id
                )));
            }
            max_revision_id = max_revision_id.max(id.0);
            if *id != ROOT_REVISION_ID && revision.parent.is_none() {
                return Err(FoundryProjectError::InvalidProject(format!(
                    "non-root revision {id:?} must have a parent"
                )));
            }
            if let Some(parent) = revision.parent {
                if !self.revisions.contains_key(&parent) {
                    return Err(FoundryProjectError::UnknownRevision(parent));
                }
                if parent.0 >= id.0 {
                    return Err(FoundryProjectError::InvalidProject(format!(
                        "revision {id:?} parent {parent:?} does not preserve monotonic ids"
                    )));
                }
            }
            validate_revision_payload(revision)?;
        }

        for revision in self.revisions.values() {
            let Some(parent_id) = revision.parent else {
                continue;
            };
            let parent = self
                .revisions
                .get(&parent_id)
                .ok_or(FoundryProjectError::UnknownRevision(parent_id))?;
            validate_revision_replay(parent, revision)?;
        }

        if self.next_revision_id <= max_revision_id {
            return Err(FoundryProjectError::InvalidProject(format!(
                "next revision id {} must be greater than existing max {max_revision_id}",
                self.next_revision_id
            )));
        }

        Ok(())
    }

    /// Save foundry project JSON to disk.
    pub fn save_json(&self, path: impl AsRef<Path>) -> Result<(), FoundryProjectError> {
        self.validate()?;
        atomic_foundry_project_write(path.as_ref(), &foundry_project_json_bytes(self)?)
    }

    /// Load foundry project JSON from disk.
    pub fn load_json(path: impl AsRef<Path>) -> Result<Self, FoundryProjectError> {
        Ok(Self::load_json_with_context(path, &FoundryProjectLoadContext::default())?.project)
    }

    /// Load foundry project JSON and verify load-time catalog and recipe inputs.
    pub fn load_json_with_context(
        path: impl AsRef<Path>,
        context: &FoundryProjectLoadContext,
    ) -> Result<FoundryProjectLoadOutcome, FoundryProjectError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| foundry_path_io("reading", path, source))?;
        let probe = foundry_project_schema_probe(&bytes, path)?;
        ensure_supported_project_kind(probe.project_kind)?;
        ensure_supported_project_schema(probe.schema_version.ok_or_else(|| {
            FoundryProjectError::InvalidProject(
                "foundry project schema_version is missing".to_owned(),
            )
        })?)?;
        let project: Self =
            serde_json::from_slice(&bytes).map_err(|source| FoundryProjectError::JsonAtPath {
                path: path.to_path_buf(),
                source,
            })?;
        project.validate()?;
        let mut report = FoundryProjectLoadReport::default();
        verify_load_context(&project, context, &mut report)?;
        Ok(FoundryProjectLoadOutcome { project, report })
    }

    /// Write a recovery snapshot of the current project to a path.
    pub fn save_recovery_snapshot(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<FoundryRecoverySnapshot, FoundryProjectError> {
        let path = path.as_ref();
        self.save_json(path)?;
        Ok(FoundryRecoverySnapshot {
            path: path.to_path_buf(),
            marker: self.persistence_marker(),
        })
    }
}

impl FoundryProjectFile {
    /// Create a new dirty project file wrapper from an initial foundry document.
    pub fn new(
        title: impl Into<String>,
        document: FoundryAssetDocument,
        catalog_lock: FoundryCatalogLock,
        build_stamp: Option<FoundryBuildStamp>,
        recipe_snapshot: Option<GeneratedRecipeSnapshot>,
        conformance: FoundryConformanceSummary,
    ) -> Result<Self, FoundryProjectError> {
        let project = FoundryProject::new(
            title,
            document,
            catalog_lock,
            build_stamp,
            recipe_snapshot,
            conformance,
        )?;
        Ok(Self {
            project,
            path: None,
            load_report: FoundryProjectLoadReport::default(),
            clean_marker: None,
        })
    }

    /// Wrap a project as already clean at its current state.
    #[must_use]
    pub fn clean(project: FoundryProject, path: Option<PathBuf>) -> Self {
        let clean_marker = Some(project.persistence_marker());
        Self {
            project,
            path,
            load_report: FoundryProjectLoadReport::default(),
            clean_marker,
        }
    }

    /// Load a project file and mark it clean.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, FoundryProjectError> {
        Self::load_with_context(path, &FoundryProjectLoadContext::default())
    }

    /// Load a project file with explicit catalog/recipe verification inputs.
    pub fn load_with_context(
        path: impl AsRef<Path>,
        context: &FoundryProjectLoadContext,
    ) -> Result<Self, FoundryProjectError> {
        let path = path.as_ref();
        ensure_foundry_project_path(path)?;
        let outcome = FoundryProject::load_json_with_context(path, context)?;
        let clean_marker = Some(outcome.project.persistence_marker());
        Ok(Self {
            project: outcome.project,
            path: Some(path.to_path_buf()),
            load_report: outcome.report,
            clean_marker,
        })
    }

    /// Return true when the project differs from the last successful load or save.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.clean_marker
            .is_none_or(|marker| self.project.is_dirty_since(marker))
    }

    /// Accept a revision program and mark the file dirty.
    pub fn accept_program(
        &mut self,
        program: FoundryProjectRevisionProgram,
        document: FoundryAssetDocument,
        catalog_lock: FoundryCatalogLock,
        build_stamp: Option<FoundryBuildStamp>,
        recipe_snapshot: Option<GeneratedRecipeSnapshot>,
        conformance: FoundryConformanceSummary,
    ) -> Result<RevisionId, FoundryProjectError> {
        self.project.accept_program(
            program,
            document,
            catalog_lock,
            build_stamp,
            recipe_snapshot,
            conformance,
        )
    }

    /// Undo to the current revision's parent and mark the file dirty.
    pub fn undo(&mut self) -> Result<RevisionId, FoundryProjectError> {
        self.project.undo()
    }

    /// Switch to an existing revision and mark the file dirty when the revision changes.
    pub fn switch_to(&mut self, revision: RevisionId) -> Result<(), FoundryProjectError> {
        self.project.switch_to(revision)
    }

    /// Save to the current path and mark the file clean after success.
    pub fn save(&mut self) -> Result<(), FoundryProjectError> {
        let path = self
            .path
            .clone()
            .ok_or(FoundryProjectError::MissingSavePath)?;
        self.project.save_json(&path)?;
        self.clean_marker = Some(self.project.persistence_marker());
        Ok(())
    }

    /// Save to a new path and mark the file clean after success.
    pub fn save_as(&mut self, path: impl AsRef<Path>) -> Result<(), FoundryProjectError> {
        let path = path.as_ref();
        ensure_foundry_project_path(path)?;
        self.project.save_json(path)?;
        self.path = Some(path.to_path_buf());
        self.clean_marker = Some(self.project.persistence_marker());
        Ok(())
    }

    /// Write a recovery snapshot without changing dirty state.
    pub fn save_recovery_snapshot(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<FoundryRecoverySnapshot, FoundryProjectError> {
        self.project.save_recovery_snapshot(path)
    }
}

#[derive(Debug, Deserialize)]
struct FoundryProjectSchemaProbe {
    project_kind: Option<String>,
    schema_version: Option<u32>,
}

#[allow(clippy::too_many_arguments)]
fn build_revision(
    id: RevisionId,
    parent: Option<RevisionId>,
    label: impl Into<String>,
    program: Option<FoundryProjectRevisionProgram>,
    mut document: FoundryAssetDocument,
    catalog_lock: FoundryCatalogLock,
    build_stamp: Option<FoundryBuildStamp>,
    recipe_snapshot: Option<GeneratedRecipeSnapshot>,
    conformance: FoundryConformanceSummary,
) -> Result<FoundryProjectRevision, FoundryProjectError> {
    document.catalog_lock = Some(catalog_lock.clone());
    document.build_stamp = build_stamp.clone();
    let revision = FoundryProjectRevision {
        id,
        parent,
        label: label.into(),
        document,
        program,
        catalog_lock,
        build_stamp,
        recipe_snapshot,
        conformance,
    };
    validate_revision_payload(&revision)?;
    Ok(revision)
}

fn validate_revision_payload(revision: &FoundryProjectRevision) -> Result<(), FoundryProjectError> {
    ensure_valid_document(&revision.document, Some(revision.id))?;
    ensure_revision_catalog_matches_document(revision)?;
    validate_embedded_snapshots(revision)?;
    if revision.document.catalog_lock.as_ref() != Some(&revision.catalog_lock) {
        return Err(FoundryProjectError::InvalidProject(format!(
            "revision {:?} document catalog lock differs from revision catalog lock",
            revision.id
        )));
    }
    if revision.document.build_stamp.as_ref() != revision.build_stamp.as_ref() {
        return Err(FoundryProjectError::InvalidProject(format!(
            "revision {:?} document build stamp differs from revision build stamp",
            revision.id
        )));
    }
    if let (Some(snapshot), Some(stamp)) = (&revision.recipe_snapshot, &revision.build_stamp)
        && snapshot.recipe_fingerprint != stamp.recipe_fingerprint
    {
        return Err(FoundryProjectError::InvalidProject(format!(
            "revision {:?} recipe snapshot fingerprint differs from build stamp",
            revision.id
        )));
    }
    if revision.parent.is_some() && revision.program.is_none() {
        return Err(FoundryProjectError::InvalidProject(format!(
            "non-root revision {:?} must store a replay program",
            revision.id
        )));
    }
    Ok(())
}

fn validate_revision_replay(
    parent: &FoundryProjectRevision,
    revision: &FoundryProjectRevision,
) -> Result<(), FoundryProjectError> {
    let Some(program) = &revision.program else {
        return Ok(());
    };
    let replayed = replay_semantic_program(revision.id, &parent.document, program)?;
    if replayed != semantic_document(&revision.document) {
        return Err(FoundryProjectError::ReplayMismatch(revision.id));
    }
    Ok(())
}

fn replay_semantic_program(
    revision: RevisionId,
    parent: &FoundryAssetDocument,
    program: &FoundryProjectRevisionProgram,
) -> Result<FoundryAssetDocument, FoundryProjectError> {
    let mut document = semantic_document(parent);
    for command in program.commands() {
        apply_semantic_command(revision, &mut document, command)?;
    }
    Ok(document)
}

fn semantic_document(document: &FoundryAssetDocument) -> FoundryAssetDocument {
    let mut semantic = document.clone();
    semantic.catalog_lock = None;
    semantic.build_stamp = None;
    semantic
}

fn apply_semantic_command(
    revision: RevisionId,
    document: &mut FoundryAssetDocument,
    command: &FoundryCommand,
) -> Result<(), FoundryProjectError> {
    match command {
        FoundryCommand::SetControl { control_id, value } => {
            document
                .control_state
                .insert(control_id.clone(), value.clone());
        }
        FoundryCommand::ResetControl { control_id } => {
            document.control_state.remove(control_id);
        }
        FoundryCommand::SelectProvider { role, provider_ref } => {
            document.provider_overrides.insert(
                role.clone(),
                ProviderOverride {
                    role: role.clone(),
                    provider_ref: provider_ref.clone(),
                },
            );
        }
        FoundryCommand::SetStyle {
            style_content_ref,
            style_implementation_ref,
        } => {
            document.style_content_ref = style_content_ref.clone();
            document.style_implementation_ref = style_implementation_ref.clone();
        }
        FoundryCommand::SetLock { lock } => {
            if let Some(existing) = document
                .foundry_locks
                .iter_mut()
                .find(|existing| existing.target == lock.target)
            {
                *existing = lock.clone();
            } else {
                document.foundry_locks.push(lock.clone());
            }
        }
        FoundryCommand::ClearLock { target } => {
            document
                .foundry_locks
                .retain(|existing| existing.target != *target);
        }
        FoundryCommand::SetVariationIntent { intent } => {
            document.variation_state.intent = intent.clone().normalized();
        }
        FoundryCommand::SetVariationScope { scope } => {
            document.variation_state.intent.scope = scope.clone();
            document.variation_state.intent = document.variation_state.intent.clone().normalized();
        }
        FoundryCommand::SetVariationChannels { channels } => {
            document.variation_state.intent.channels = channels.clone();
            document.variation_state.intent = document.variation_state.intent.clone().normalized();
        }
        FoundryCommand::ClearVariationFocus => {
            document.variation_state.intent.scope = shape_foundry::VariationScope::WholeAsset;
            document.variation_state.intent = document.variation_state.intent.clone().normalized();
        }
        FoundryCommand::SetFocusPartGroup { group_id } => {
            document.variation_state.intent.scope =
                shape_foundry::VariationScope::SemanticPartGroup {
                    group_id: group_id.clone(),
                    display_name: focus_part_display_name(group_id),
                };
            document.variation_state.intent = document.variation_state.intent.clone().normalized();
        }
        FoundryCommand::SetRolePresence { role, enabled } => {
            document
                .control_state
                .insert(role.clone(), ControlValue::Toggle(*enabled));
        }
        FoundryCommand::GenerateCandidates(_)
        | FoundryCommand::AcceptCandidate { .. }
        | FoundryCommand::RejectCandidate { .. }
        | FoundryCommand::Undo
        | FoundryCommand::SwitchRevision { .. }
        | FoundryCommand::Export { .. }
        | FoundryCommand::AddCurrentToPack { .. } => {
            return Err(unsupported_command(
                revision,
                command,
                "command is a runtime action, not a semantic document edit",
            ));
        }
    }
    Ok(())
}

fn unsupported_command(
    revision: RevisionId,
    command: &FoundryCommand,
    reason: impl Into<String>,
) -> FoundryProjectError {
    FoundryProjectError::ReplayRejected {
        revision,
        command: command_name(command),
        reason: reason.into(),
    }
}

fn focus_part_display_name(group_id: &str) -> String {
    group_id
        .split(['_', '-', '.'])
        .filter(|part| !part.trim().is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut label = first.to_uppercase().collect::<String>();
                    label.push_str(&chars.as_str().to_ascii_lowercase());
                    label
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn command_name(command: &FoundryCommand) -> &'static str {
    match command {
        FoundryCommand::SetControl { .. } => "set_control",
        FoundryCommand::ResetControl { .. } => "reset_control",
        FoundryCommand::SelectProvider { .. } => "select_provider",
        FoundryCommand::SetRolePresence { .. } => "set_role_presence",
        FoundryCommand::SetStyle { .. } => "set_style",
        FoundryCommand::SetLock { .. } => "set_lock",
        FoundryCommand::ClearLock { .. } => "clear_lock",
        FoundryCommand::SetVariationIntent { .. } => "set_variation_intent",
        FoundryCommand::SetVariationScope { .. } => "set_variation_scope",
        FoundryCommand::SetVariationChannels { .. } => "set_variation_channels",
        FoundryCommand::ClearVariationFocus => "clear_variation_focus",
        FoundryCommand::SetFocusPartGroup { .. } => "set_focus_part_group",
        FoundryCommand::GenerateCandidates(_) => "generate_candidates",
        FoundryCommand::AcceptCandidate { .. } => "accept_candidate",
        FoundryCommand::RejectCandidate { .. } => "reject_candidate",
        FoundryCommand::Undo => "undo",
        FoundryCommand::SwitchRevision { .. } => "switch_revision",
        FoundryCommand::Export { .. } => "export",
        FoundryCommand::AddCurrentToPack { .. } => "add_current_to_pack",
    }
}

fn ensure_valid_document(
    document: &FoundryAssetDocument,
    revision: Option<RevisionId>,
) -> Result<(), FoundryProjectError> {
    let report = validate_foundry_document(document);
    if report.is_valid() {
        Ok(())
    } else {
        Err(FoundryProjectError::InvalidDocument {
            revision,
            report: Box::new(report),
        })
    }
}

fn ensure_revision_catalog_matches_document(
    revision: &FoundryProjectRevision,
) -> Result<(), FoundryProjectError> {
    let expected_refs = document_catalog_refs(&revision.document);
    for (key, expected) in &expected_refs {
        let Some(actual) = revision.catalog_lock.exact_refs.get(key) else {
            return Err(FoundryProjectError::InvalidProject(format!(
                "revision {:?} catalog lock is missing {key}",
                revision.id
            )));
        };
        if actual != expected {
            return Err(FoundryProjectError::InvalidProject(format!(
                "revision {:?} catalog lock {key} does not match document reference",
                revision.id
            )));
        }
    }
    for key in revision.catalog_lock.exact_refs.keys() {
        if !expected_refs.contains_key(key) {
            return Err(FoundryProjectError::InvalidProject(format!(
                "revision {:?} catalog lock contains unused exact reference {key}",
                revision.id
            )));
        }
    }
    Ok(())
}

fn validate_embedded_snapshots(
    revision: &FoundryProjectRevision,
) -> Result<(), FoundryProjectError> {
    for snapshot in &revision.catalog_lock.embedded_snapshots {
        if snapshot.canonical_json.trim().is_empty() {
            return Err(FoundryProjectError::InvalidProject(format!(
                "revision {:?} has an empty embedded catalog snapshot",
                revision.id
            )));
        }
        if !revision
            .catalog_lock
            .exact_refs
            .values()
            .any(|content_ref| content_ref == &snapshot.content_ref)
        {
            return Err(FoundryProjectError::InvalidProject(format!(
                "revision {:?} embedded catalog snapshot does not satisfy its lock",
                revision.id
            )));
        }
        verify_catalog_content_fingerprint(
            "embedded_snapshot",
            &snapshot.content_ref,
            &snapshot.canonical_json,
        )
        .map_err(|error| {
            FoundryProjectError::InvalidProject(format!(
                "revision {:?} embedded catalog snapshot fingerprint mismatch: {error:?}",
                revision.id
            ))
        })?;
    }
    Ok(())
}

fn verify_load_context(
    project: &FoundryProject,
    context: &FoundryProjectLoadContext,
    report: &mut FoundryProjectLoadReport,
) -> Result<(), FoundryProjectError> {
    for revision in project.revisions.values() {
        let mut stale = Vec::new();
        verify_catalog_inputs(revision, context, report, &mut stale)?;
        verify_build_stamp(revision, context, &mut stale);
        verify_available_recipe(revision, context, report)?;
        if !stale.is_empty() {
            report.stale_builds.insert(revision.id, stale);
        }
    }
    Ok(())
}

fn verify_catalog_inputs(
    revision: &FoundryProjectRevision,
    context: &FoundryProjectLoadContext,
    report: &mut FoundryProjectLoadReport,
    stale: &mut Vec<FoundryBuildStaleReason>,
) -> Result<(), FoundryProjectError> {
    if let Some(current) = context.current_catalog_version
        && revision.catalog_lock.catalog_version != current
    {
        stale.push(FoundryBuildStaleReason::CatalogVersionChanged {
            stored: revision.catalog_lock.catalog_version,
            current,
        });
    }
    if let Some(current) = &context.current_compiler_version
        && &revision.catalog_lock.compiler_version != current
    {
        stale.push(FoundryBuildStaleReason::CatalogCompilerVersionChanged {
            stored: revision.catalog_lock.compiler_version.clone(),
            current: current.clone(),
        });
    }

    for (key, expected) in &revision.catalog_lock.exact_refs {
        match context.catalog_refs.get(key) {
            Some(actual) if actual == expected => {}
            Some(_) => {
                stale.push(FoundryBuildStaleReason::CatalogReferenceChanged { key: key.clone() });
                if has_embedded_snapshot(&revision.catalog_lock, expected) {
                    mark_recovery(report, revision.id);
                } else {
                    return Err(FoundryProjectError::CatalogFingerprintMismatch {
                        revision: revision.id,
                        key: key.clone(),
                    });
                }
            }
            None if context.require_catalog_inputs => {
                if has_embedded_snapshot(&revision.catalog_lock, expected) {
                    mark_recovery(report, revision.id);
                } else {
                    return Err(FoundryProjectError::MissingCatalogInput {
                        revision: revision.id,
                        key: key.clone(),
                    });
                }
            }
            None => {}
        }
    }
    Ok(())
}

fn verify_build_stamp(
    revision: &FoundryProjectRevision,
    context: &FoundryProjectLoadContext,
    stale: &mut Vec<FoundryBuildStaleReason>,
) {
    let Some(stamp) = &revision.build_stamp else {
        return;
    };
    if let Some(current) = &context.current_foundry_version
        && &stamp.foundry_version != current
    {
        stale.push(FoundryBuildStaleReason::FoundryVersionChanged {
            stored: stamp.foundry_version.clone(),
            current: current.clone(),
        });
    }
    if let Some(current) = &context.current_family_compile_version
        && &stamp.family_compile_version != current
    {
        stale.push(FoundryBuildStaleReason::FamilyCompileVersionChanged {
            stored: stamp.family_compile_version.clone(),
            current: current.clone(),
        });
    }
}

fn verify_available_recipe(
    revision: &FoundryProjectRevision,
    context: &FoundryProjectLoadContext,
    report: &mut FoundryProjectLoadReport,
) -> Result<(), FoundryProjectError> {
    let Some(recipe) = context.available_recipes.get(&revision.id) else {
        return Ok(());
    };
    let Some(snapshot) = &revision.recipe_snapshot else {
        return Err(FoundryProjectError::MissingRecipeSnapshot(revision.id));
    };
    if !snapshot.matches_recipe(recipe)? {
        return Err(FoundryProjectError::RecipeSnapshotMismatch(revision.id));
    }
    report.verified_recipe_revisions.push(revision.id);
    Ok(())
}

fn has_embedded_snapshot(lock: &FoundryCatalogLock, expected: &CatalogContentRef) -> bool {
    lock.embedded_snapshots
        .iter()
        .any(|snapshot| snapshot.content_ref == *expected)
}

fn mark_recovery(report: &mut FoundryProjectLoadReport, revision: RevisionId) {
    report.read_only_recovery = true;
    report.recovery_revisions.insert(revision);
}

fn revision_label(program: &FoundryProjectRevisionProgram) -> String {
    let trimmed = program.label().trim();
    if trimmed.is_empty() {
        "Foundry edit".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn ensure_supported_project_kind(kind: Option<String>) -> Result<(), FoundryProjectError> {
    if kind.as_deref() == Some(FOUNDRY_PROJECT_KIND) {
        Ok(())
    } else {
        Err(FoundryProjectError::UnsupportedProjectKind {
            found: kind,
            expected: FOUNDRY_PROJECT_KIND,
        })
    }
}

fn ensure_supported_project_schema(schema_version: u32) -> Result<(), FoundryProjectError> {
    if schema_version == FOUNDRY_PROJECT_DOCUMENT_SCHEMA_VERSION {
        Ok(())
    } else if schema_version > FOUNDRY_PROJECT_DOCUMENT_SCHEMA_VERSION {
        Err(FoundryProjectError::FutureSchemaVersion {
            found: schema_version,
            supported: FOUNDRY_PROJECT_DOCUMENT_SCHEMA_VERSION,
        })
    } else {
        Err(FoundryProjectError::UnsupportedSchemaVersion(
            schema_version,
        ))
    }
}

fn foundry_project_schema_probe(
    bytes: &[u8],
    path: &Path,
) -> Result<FoundryProjectSchemaProbe, FoundryProjectError> {
    serde_json::from_slice(bytes).map_err(|source| FoundryProjectError::JsonAtPath {
        path: path.to_path_buf(),
        source,
    })
}

fn foundry_project_json_bytes(project: &FoundryProject) -> Result<Vec<u8>, FoundryProjectError> {
    let mut bytes = serde_json::to_vec_pretty(project)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Return true when `path` uses the `.shapelab-foundry.json` filename suffix.
#[must_use]
pub fn has_foundry_project_suffix(path: impl AsRef<Path>) -> bool {
    path.as_ref()
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(FOUNDRY_PROJECT_FILE_SUFFIX))
}

/// Ensure a path uses the `.shapelab-foundry.json` filename suffix.
pub fn ensure_foundry_project_path(path: impl AsRef<Path>) -> Result<(), FoundryProjectError> {
    let path = path.as_ref();
    if has_foundry_project_suffix(path) {
        Ok(())
    } else {
        Err(FoundryProjectError::InvalidProjectPath {
            path: path.to_path_buf(),
        })
    }
}

/// Return a deterministic sibling recovery snapshot path for a project file path.
#[must_use]
pub fn recovery_snapshot_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    let parent = sibling_directory(path);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("untitled.shapelab-foundry.json");
    parent.join(format!(".{name}.autosave"))
}

fn atomic_foundry_project_write(path: &Path, bytes: &[u8]) -> Result<(), FoundryProjectError> {
    atomic_foundry_project_replace(path, |file| file.write_all(bytes))
}

fn atomic_foundry_project_replace(
    path: &Path,
    write_temp: impl FnOnce(&mut File) -> io::Result<()>,
) -> Result<(), FoundryProjectError> {
    cleanup_obsolete_foundry_temp_files(path);

    let mut temp = TempSibling::create(path)?;

    write_temp(temp.file_mut()).map_err(|source| {
        foundry_path_io("writing temporary foundry project file for", path, source)
    })?;
    temp.file_mut().sync_all().map_err(|source| {
        foundry_path_io("flushing temporary foundry project file for", path, source)
    })?;
    temp.persist(path)
        .map_err(|source| foundry_path_io("replacing", path, source))?;

    cleanup_obsolete_foundry_temp_files(path);
    Ok(())
}

struct TempSibling {
    path: PathBuf,
    file: Option<File>,
    persisted: bool,
}

impl TempSibling {
    fn create(target: &Path) -> Result<Self, FoundryProjectError> {
        let parent = sibling_directory(target);
        let prefix = foundry_temp_prefix(target);
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
                    return Err(foundry_path_io(
                        "creating temporary foundry project file for",
                        target,
                        error,
                    ));
                }
            }
        }

        Err(foundry_path_io(
            "creating temporary foundry project file for",
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

fn cleanup_obsolete_foundry_temp_files(path: &Path) {
    let prefix = foundry_temp_prefix(path);
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

fn foundry_temp_prefix(path: &Path) -> String {
    format!("{FOUNDRY_PROJECT_TEMP_PREFIX}{}-", path_file_fragment(path))
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

fn foundry_path_io(action: &'static str, path: &Path, source: io::Error) -> FoundryProjectError {
    FoundryProjectError::PathIo {
        action,
        path: path.to_path_buf(),
        source,
    }
}
