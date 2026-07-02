//! Branchable project persistence for explicit asset recipes.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use orchard_asset::{
    AssetEditProgram, AssetRecipe, AssetValidationReport, RevisionId,
    apply_edit_program_with_report, validate_asset_recipe,
};
use orchard_compile::export::{
    ExportError, GroupedObjReport, ModelExportPackagePaths, fnv64, write_grouped_obj_export,
    write_model_package,
};
use orchard_compile::{CompileError, CompileValidationReport, compile_asset};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current schema version for `.shapelab-asset.json` project files.
pub const ASSET_PROJECT_SCHEMA_VERSION: u32 = 1;
/// Distinct project kind marker used to reject legacy `ShapeDocument` project JSON.
pub const ASSET_PROJECT_KIND: &str = "shape-lab.asset-project";
/// Required filename suffix for user-facing asset project files.
pub const ASSET_PROJECT_FILE_SUFFIX: &str = ".shapelab-asset.json";

/// Export package paths returned by asset project package export.
pub type AssetModelExportPackagePaths = ModelExportPackagePaths;
/// Grouped OBJ export report returned by asset project OBJ export.
pub type AssetGroupedObjReport = GroupedObjReport;

const ROOT_REVISION_ID: RevisionId = RevisionId(0);
const ASSET_PROJECT_TEMP_PREFIX: &str = ".object-orchard-asset-project-";
const TEMP_FILE_SUFFIX: &str = ".tmp";
const OBSOLETE_TEMP_MIN_AGE: Duration = Duration::from_secs(60 * 60);
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// One stored asset recipe revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetRevision {
    /// Stable revision ID.
    pub id: RevisionId,
    /// Parent revision, if any.
    pub parent: Option<RevisionId>,
    /// Human-facing revision label.
    pub label: String,
    /// Accepted edit program that produced this revision.
    pub edit: Option<AssetEditProgram>,
    /// Complete recipe snapshot for direct restoration.
    pub recipe: AssetRecipe,
    /// Stable FNV-1a hash of the serialized compiled artifact.
    pub compiled_artifact_hash: u64,
    /// Deterministic validation summary captured when the revision was created.
    pub validation: AssetRevisionValidationSummary,
    /// Timestamp-free deterministic revision metadata.
    pub metadata: AssetRevisionMetadata,
}

/// Deterministic validation summary stored per revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRevisionValidationSummary {
    /// Whether the source recipe validation report had no issues.
    pub recipe_valid: bool,
    /// Number of source recipe validation issues.
    pub recipe_issue_count: usize,
    /// Stable sorted source recipe validation codes.
    pub recipe_issue_codes: Vec<String>,
    /// Whether the compiled artifact validation report had no issues.
    pub compiled_artifact_valid: bool,
    /// Number of compiled artifact validation issues.
    pub compiled_artifact_issue_count: usize,
    /// Stable sorted compiled artifact validation codes.
    pub compiled_artifact_issue_codes: Vec<String>,
}

/// Timestamp-free deterministic metadata stored per revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRevisionMetadata {
    /// Monotonic creation order within the project.
    pub ordinal: u64,
    /// Parent ordinal, if any.
    pub parent_ordinal: Option<u64>,
    /// Number of accepted edit operations.
    pub accepted_edit_operation_count: usize,
    /// Stable FNV-1a hash of the serialized recipe snapshot.
    pub recipe_hash: u64,
    /// Hash algorithm identifier for recipe and artifact hashes.
    pub hash_algorithm: String,
}

/// Branchable asset project file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetProject {
    /// Distinct project kind marker.
    pub project_kind: String,
    /// Project schema version.
    pub schema_version: u32,
    /// Project title.
    pub title: String,
    /// Current revision.
    pub current_revision: RevisionId,
    /// Next revision ID.
    pub next_revision_id: u64,
    /// Revision graph.
    pub revisions: BTreeMap<RevisionId, AssetRevision>,
}

/// Marker a UI can compare later to detect unsaved project state.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct AssetProjectPersistenceMarker {
    /// Current revision when the marker was captured.
    pub current_revision: RevisionId,
    /// Next revision ID when the marker was captured.
    pub next_revision_id: u64,
    /// Number of revisions when the marker was captured.
    pub revision_count: usize,
}

/// Asset project plus file-path and clean-state bookkeeping.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetProjectFile {
    /// Branchable project payload.
    pub project: AssetProject,
    /// Current project path, when saved or loaded from disk.
    pub path: Option<PathBuf>,
    clean_marker: Option<AssetProjectPersistenceMarker>,
}

/// Autosave snapshot metadata returned after a recovery snapshot is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetAutosaveSnapshot {
    /// Autosave file path.
    pub path: PathBuf,
    /// Project marker represented by the autosave file.
    pub marker: AssetProjectPersistenceMarker,
}

/// Asset project errors.
#[derive(Debug, Error)]
pub enum AssetProjectError {
    /// Revision not found.
    #[error("unknown asset revision {0:?}")]
    UnknownRevision(RevisionId),
    /// There is no parent revision to undo to.
    #[error("current asset revision has no parent")]
    NoParent,
    /// Saving requires a file path.
    #[error("asset project save requires a file path")]
    MissingSavePath,
    /// Project file does not use the asset project filename suffix.
    #[error("asset project path must end with {ASSET_PROJECT_FILE_SUFFIX}: {}", path.display())]
    InvalidProjectPath {
        /// Invalid path.
        path: PathBuf,
    },
    /// Project kind marker is missing or unsupported.
    #[error("unsupported project kind {found:?}; expected {expected}")]
    UnsupportedProjectKind {
        /// Found project kind.
        found: Option<String>,
        /// Expected project kind.
        expected: &'static str,
    },
    /// Project schema version is not supported.
    #[error("unsupported asset project schema version {0}")]
    UnsupportedSchemaVersion(u32),
    /// Project schema version is newer than this build understands.
    #[error("asset project schema version {found} is newer than supported version {supported}")]
    FutureSchemaVersion {
        /// Version found in the file.
        found: u32,
        /// Newest version supported by this build.
        supported: u32,
    },
    /// The project revision graph is malformed.
    #[error("invalid asset project: {0}")]
    InvalidProject(String),
    /// A recipe snapshot failed asset validation.
    #[error("invalid asset recipe in revision {revision:?}")]
    InvalidRecipe {
        /// Revision containing the invalid snapshot, if known.
        revision: Option<RevisionId>,
        /// Full validation report from orchard-asset.
        report: Box<AssetValidationReport>,
    },
    /// An accepted edit program was rejected.
    #[error("asset edit program rejected")]
    EditRejected {
        /// Deterministic edit report.
        report: Box<orchard_asset::AssetEditReport>,
    },
    /// Allocating a new revision ID would overflow.
    #[error("asset revision id overflow after {0}")]
    RevisionIdOverflow(u64),
    /// Asset compilation failed.
    #[error("asset compile error: {0}")]
    Compile(#[from] CompileError),
    /// Asset export failed.
    #[error("asset export error: {0}")]
    Export(#[from] ExportError),
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

impl AssetProject {
    /// Create a new branchable asset project from a template recipe.
    pub fn new_from_template(
        title: impl Into<String>,
        recipe: AssetRecipe,
    ) -> Result<Self, AssetProjectError> {
        ensure_valid_recipe(&recipe, Some(ROOT_REVISION_ID))?;
        let revision = build_revision(ROOT_REVISION_ID, None, "Initial", None, recipe, 0, None)?;
        let mut revisions = BTreeMap::new();
        revisions.insert(ROOT_REVISION_ID, revision);
        let project = Self {
            project_kind: ASSET_PROJECT_KIND.to_owned(),
            schema_version: ASSET_PROJECT_SCHEMA_VERSION,
            title: title.into(),
            current_revision: ROOT_REVISION_ID,
            next_revision_id: 1,
            revisions,
        };
        project.validate()?;
        Ok(project)
    }

    /// Create a project and use the recipe title as the project title.
    pub fn from_template(recipe: AssetRecipe) -> Result<Self, AssetProjectError> {
        let title = recipe.title.clone();
        Self::new_from_template(title, recipe)
    }

    /// Return the current revision.
    pub fn current(&self) -> Result<&AssetRevision, AssetProjectError> {
        self.revisions
            .get(&self.current_revision)
            .ok_or(AssetProjectError::UnknownRevision(self.current_revision))
    }

    /// Return the current complete recipe snapshot.
    pub fn current_recipe(&self) -> Result<&AssetRecipe, AssetProjectError> {
        Ok(&self.current()?.recipe)
    }

    /// Return true when the current revision has a parent.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.revisions
            .get(&self.current_revision)
            .and_then(|revision| revision.parent)
            .is_some()
    }

    /// Accept an asset edit program and append a child revision.
    pub fn accept_candidate(
        &mut self,
        program: AssetEditProgram,
    ) -> Result<RevisionId, AssetProjectError> {
        let parent = self.current_revision;
        let parent_revision = self.current()?;
        let outcome = apply_edit_program_with_report(&parent_revision.recipe, &program).map_err(
            |rejection| AssetProjectError::EditRejected {
                report: Box::new(rejection.report),
            },
        )?;
        let id = RevisionId(self.next_revision_id);
        if self.revisions.contains_key(&id) {
            return Err(AssetProjectError::InvalidProject(format!(
                "next revision id {id:?} already exists"
            )));
        }
        let next_revision_id = self
            .next_revision_id
            .checked_add(1)
            .ok_or(AssetProjectError::RevisionIdOverflow(self.next_revision_id))?;
        let label = revision_label(&program);
        let revision = build_revision(
            id,
            Some(parent),
            label,
            Some(program),
            outcome.recipe,
            id.0,
            Some(parent_revision.metadata.ordinal),
        )?;
        self.revisions.insert(id, revision);
        self.current_revision = id;
        self.next_revision_id = next_revision_id;
        Ok(id)
    }

    /// Move to the parent revision without deleting child revisions.
    pub fn undo(&mut self) -> Result<RevisionId, AssetProjectError> {
        let parent = self.current()?.parent.ok_or(AssetProjectError::NoParent)?;
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
    pub fn switch_to(&mut self, revision: RevisionId) -> Result<(), AssetProjectError> {
        if self.revisions.contains_key(&revision) {
            self.current_revision = revision;
            Ok(())
        } else {
            Err(AssetProjectError::UnknownRevision(revision))
        }
    }

    /// Return the current revision path, starting at current and ending at root.
    pub fn revision_path_to_root(&self) -> Result<Vec<RevisionId>, AssetProjectError> {
        self.revision_path_to_root_from(self.current_revision)
    }

    /// Return a revision path, starting at `revision` and ending at root.
    pub fn revision_path_to_root_from(
        &self,
        revision: RevisionId,
    ) -> Result<Vec<RevisionId>, AssetProjectError> {
        let mut path = Vec::new();
        let mut cursor = revision;
        loop {
            let current = self
                .revisions
                .get(&cursor)
                .ok_or(AssetProjectError::UnknownRevision(cursor))?;
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
    pub fn persistence_marker(&self) -> AssetProjectPersistenceMarker {
        AssetProjectPersistenceMarker {
            current_revision: self.current_revision,
            next_revision_id: self.next_revision_id,
            revision_count: self.revisions.len(),
        }
    }

    /// Return true when this project differs from a previously captured marker.
    #[must_use]
    pub fn is_dirty_since(&self, marker: AssetProjectPersistenceMarker) -> bool {
        self.persistence_marker() != marker
    }

    /// Validate project-level and recipe-level invariants.
    pub fn validate(&self) -> Result<(), AssetProjectError> {
        ensure_supported_project_kind(Some(self.project_kind.clone()))?;
        ensure_supported_project_schema(self.schema_version)?;
        if self.revisions.is_empty() {
            return Err(AssetProjectError::InvalidProject(
                "revision graph must contain revision 0".to_owned(),
            ));
        }
        let Some(root) = self.revisions.get(&ROOT_REVISION_ID) else {
            return Err(AssetProjectError::InvalidProject(
                "revision graph is missing revision 0".to_owned(),
            ));
        };
        if root.parent.is_some() {
            return Err(AssetProjectError::InvalidProject(
                "revision 0 must not have a parent".to_owned(),
            ));
        }
        if !self.revisions.contains_key(&self.current_revision) {
            return Err(AssetProjectError::UnknownRevision(self.current_revision));
        }

        let mut max_revision_id = 0;
        let mut ordinals = BTreeSet::new();
        for (id, revision) in &self.revisions {
            if revision.id != *id {
                return Err(AssetProjectError::InvalidProject(format!(
                    "revision map key {id:?} does not match stored id {:?}",
                    revision.id
                )));
            }
            max_revision_id = max_revision_id.max(id.0);
            if *id != ROOT_REVISION_ID && revision.parent.is_none() {
                return Err(AssetProjectError::InvalidProject(format!(
                    "non-root revision {id:?} must have a parent"
                )));
            }
            if let Some(parent) = revision.parent {
                if !self.revisions.contains_key(&parent) {
                    return Err(AssetProjectError::UnknownRevision(parent));
                }
                if parent.0 >= id.0 {
                    return Err(AssetProjectError::InvalidProject(format!(
                        "revision {id:?} parent {parent:?} does not preserve monotonic ids"
                    )));
                }
            }
            if !ordinals.insert(revision.metadata.ordinal) {
                return Err(AssetProjectError::InvalidProject(format!(
                    "duplicate revision ordinal {}",
                    revision.metadata.ordinal
                )));
            }
            ensure_valid_recipe(&revision.recipe, Some(*id))?;
            validate_revision_payload(revision)?;
        }

        if self.next_revision_id <= max_revision_id {
            return Err(AssetProjectError::InvalidProject(format!(
                "next revision id {} must be greater than existing max {max_revision_id}",
                self.next_revision_id
            )));
        }

        Ok(())
    }

    /// Save asset project JSON to disk.
    pub fn save_json(&self, path: impl AsRef<Path>) -> Result<(), AssetProjectError> {
        self.validate()?;
        atomic_asset_project_write(path.as_ref(), &asset_project_json_bytes(self)?)
    }

    /// Load asset project JSON from disk.
    pub fn load_json(path: impl AsRef<Path>) -> Result<Self, AssetProjectError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| asset_path_io("reading", path, source))?;
        let probe = asset_project_schema_probe(&bytes, path)?;
        ensure_supported_project_kind(probe.project_kind)?;
        ensure_supported_project_schema(probe.schema_version.ok_or_else(|| {
            AssetProjectError::InvalidProject("asset project schema_version is missing".to_owned())
        })?)?;
        let project: Self =
            serde_json::from_slice(&bytes).map_err(|source| AssetProjectError::JsonAtPath {
                path: path.to_path_buf(),
                source,
            })?;
        project.validate()?;
        Ok(project)
    }

    /// Write a recovery snapshot of the current project to an autosave path.
    pub fn save_autosave_snapshot(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<AssetAutosaveSnapshot, AssetProjectError> {
        let path = path.as_ref();
        self.save_json(path)?;
        Ok(AssetAutosaveSnapshot {
            path: path.to_path_buf(),
            marker: self.persistence_marker(),
        })
    }

    /// Compile and export the current model package.
    pub fn export_current_model_package(
        &self,
        out_dir: impl AsRef<Path>,
    ) -> Result<ModelExportPackagePaths, AssetProjectError> {
        let recipe = self.current_recipe()?;
        let artifact = compile_asset(recipe)?;
        Ok(write_model_package(recipe, &artifact, out_dir)?)
    }

    /// Compile and export the current model as grouped OBJ.
    pub fn export_current_obj(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<GroupedObjReport, AssetProjectError> {
        let recipe = self.current_recipe()?;
        let artifact = compile_asset(recipe)?;
        let export = write_grouped_obj_export(&artifact, Some(recipe))?;
        atomic_asset_project_write(path.as_ref(), export.obj.as_bytes())?;
        Ok(export.report)
    }
}

impl AssetProjectFile {
    /// Create a new dirty project file wrapper from a template recipe.
    pub fn new_from_template(
        title: impl Into<String>,
        recipe: AssetRecipe,
    ) -> Result<Self, AssetProjectError> {
        let project = AssetProject::new_from_template(title, recipe)?;
        Ok(Self {
            project,
            path: None,
            clean_marker: None,
        })
    }

    /// Wrap a project as already clean at its current state.
    #[must_use]
    pub fn clean(project: AssetProject, path: Option<PathBuf>) -> Self {
        let clean_marker = Some(project.persistence_marker());
        Self {
            project,
            path,
            clean_marker,
        }
    }

    /// Load a project file and mark it clean.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, AssetProjectError> {
        let path = path.as_ref();
        ensure_asset_project_path(path)?;
        let project = AssetProject::load_json(path)?;
        Ok(Self::clean(project, Some(path.to_path_buf())))
    }

    /// Return true when the project differs from the last successful load or save.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.clean_marker
            .is_none_or(|marker| self.project.is_dirty_since(marker))
    }

    /// Accept an asset edit candidate and mark the file dirty.
    pub fn accept_candidate(
        &mut self,
        program: AssetEditProgram,
    ) -> Result<RevisionId, AssetProjectError> {
        self.project.accept_candidate(program)
    }

    /// Undo to the current revision's parent and mark the file dirty.
    pub fn undo(&mut self) -> Result<RevisionId, AssetProjectError> {
        self.project.undo()
    }

    /// Switch to an existing revision and mark the file dirty when the revision changes.
    pub fn switch_to(&mut self, revision: RevisionId) -> Result<(), AssetProjectError> {
        self.project.switch_to(revision)
    }

    /// Save to the current path and mark the file clean after success.
    pub fn save(&mut self) -> Result<(), AssetProjectError> {
        let path = self
            .path
            .clone()
            .ok_or(AssetProjectError::MissingSavePath)?;
        self.project.save_json(&path)?;
        self.clean_marker = Some(self.project.persistence_marker());
        Ok(())
    }

    /// Save to a new path and mark the file clean after success.
    pub fn save_as(&mut self, path: impl AsRef<Path>) -> Result<(), AssetProjectError> {
        let path = path.as_ref();
        ensure_asset_project_path(path)?;
        self.project.save_json(path)?;
        self.path = Some(path.to_path_buf());
        self.clean_marker = Some(self.project.persistence_marker());
        Ok(())
    }

    /// Write a recovery snapshot without changing dirty state.
    pub fn save_autosave_snapshot(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<AssetAutosaveSnapshot, AssetProjectError> {
        self.project.save_autosave_snapshot(path)
    }

    /// Export the current model package without changing dirty state.
    pub fn export_current_model_package(
        &self,
        out_dir: impl AsRef<Path>,
    ) -> Result<ModelExportPackagePaths, AssetProjectError> {
        self.project.export_current_model_package(out_dir)
    }

    /// Export the current model as grouped OBJ without changing dirty state.
    pub fn export_current_obj(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<GroupedObjReport, AssetProjectError> {
        self.project.export_current_obj(path)
    }
}

#[derive(Debug, Deserialize)]
struct AssetProjectSchemaProbe {
    project_kind: Option<String>,
    schema_version: Option<u32>,
}

fn build_revision(
    id: RevisionId,
    parent: Option<RevisionId>,
    label: impl Into<String>,
    edit: Option<AssetEditProgram>,
    recipe: AssetRecipe,
    ordinal: u64,
    parent_ordinal: Option<u64>,
) -> Result<AssetRevision, AssetProjectError> {
    ensure_valid_recipe(&recipe, Some(id))?;
    let artifact = compile_asset(&recipe)?;
    let compiled_artifact_hash = stable_hash(&artifact)?;
    let recipe_hash = stable_hash(&recipe)?;
    let validation =
        validation_summary(&validate_asset_recipe(&recipe), &artifact.validation_report);
    let accepted_edit_operation_count = edit
        .as_ref()
        .map(|program| program.operations.len())
        .unwrap_or(0);
    Ok(AssetRevision {
        id,
        parent,
        label: label.into(),
        edit,
        recipe,
        compiled_artifact_hash,
        validation,
        metadata: AssetRevisionMetadata {
            ordinal,
            parent_ordinal,
            accepted_edit_operation_count,
            recipe_hash,
            hash_algorithm: "fnv1a64-json".to_owned(),
        },
    })
}

fn validation_summary(
    recipe_report: &AssetValidationReport,
    compile_report: &CompileValidationReport,
) -> AssetRevisionValidationSummary {
    AssetRevisionValidationSummary {
        recipe_valid: recipe_report.is_valid(),
        recipe_issue_count: recipe_report.issues.len(),
        recipe_issue_codes: sorted_codes(recipe_report.issues.iter().map(|issue| &issue.code)),
        compiled_artifact_valid: compile_report.is_valid(),
        compiled_artifact_issue_count: compile_report.issues.len(),
        compiled_artifact_issue_codes: sorted_codes(
            compile_report.issues.iter().map(|issue| &issue.code),
        ),
    }
}

fn validate_revision_payload(revision: &AssetRevision) -> Result<(), AssetProjectError> {
    let artifact = compile_asset(&revision.recipe)?;
    let expected_artifact_hash = stable_hash(&artifact)?;
    if revision.compiled_artifact_hash != expected_artifact_hash {
        return Err(AssetProjectError::InvalidProject(format!(
            "revision {:?} compiled artifact hash does not match its recipe snapshot",
            revision.id
        )));
    }
    let expected_recipe_hash = stable_hash(&revision.recipe)?;
    if revision.metadata.recipe_hash != expected_recipe_hash {
        return Err(AssetProjectError::InvalidProject(format!(
            "revision {:?} recipe hash does not match its recipe snapshot",
            revision.id
        )));
    }
    let expected_validation = validation_summary(
        &validate_asset_recipe(&revision.recipe),
        &artifact.validation_report,
    );
    if revision.validation != expected_validation {
        return Err(AssetProjectError::InvalidProject(format!(
            "revision {:?} validation summary does not match its recipe snapshot",
            revision.id
        )));
    }
    let expected_operation_count = revision
        .edit
        .as_ref()
        .map(|program| program.operations.len())
        .unwrap_or(0);
    if revision.metadata.accepted_edit_operation_count != expected_operation_count {
        return Err(AssetProjectError::InvalidProject(format!(
            "revision {:?} edit operation count metadata is inconsistent",
            revision.id
        )));
    }
    Ok(())
}

fn sorted_codes<'a>(codes: impl IntoIterator<Item = &'a String>) -> Vec<String> {
    codes
        .into_iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn ensure_valid_recipe(
    recipe: &AssetRecipe,
    revision: Option<RevisionId>,
) -> Result<(), AssetProjectError> {
    let report = validate_asset_recipe(recipe);
    if report.is_valid() {
        Ok(())
    } else {
        Err(AssetProjectError::InvalidRecipe {
            revision,
            report: Box::new(report),
        })
    }
}

fn revision_label(program: &AssetEditProgram) -> String {
    let trimmed = program.label.trim();
    if trimmed.is_empty() {
        "Asset edit".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn stable_hash(value: &impl Serialize) -> Result<u64, AssetProjectError> {
    Ok(fnv64(&serde_json::to_vec(value)?))
}

fn ensure_supported_project_kind(kind: Option<String>) -> Result<(), AssetProjectError> {
    if kind.as_deref() == Some(ASSET_PROJECT_KIND) {
        Ok(())
    } else {
        Err(AssetProjectError::UnsupportedProjectKind {
            found: kind,
            expected: ASSET_PROJECT_KIND,
        })
    }
}

fn ensure_supported_project_schema(schema_version: u32) -> Result<(), AssetProjectError> {
    if schema_version == ASSET_PROJECT_SCHEMA_VERSION {
        Ok(())
    } else if schema_version > ASSET_PROJECT_SCHEMA_VERSION {
        Err(AssetProjectError::FutureSchemaVersion {
            found: schema_version,
            supported: ASSET_PROJECT_SCHEMA_VERSION,
        })
    } else {
        Err(AssetProjectError::UnsupportedSchemaVersion(schema_version))
    }
}

fn asset_project_schema_probe(
    bytes: &[u8],
    path: &Path,
) -> Result<AssetProjectSchemaProbe, AssetProjectError> {
    serde_json::from_slice(bytes).map_err(|source| AssetProjectError::JsonAtPath {
        path: path.to_path_buf(),
        source,
    })
}

fn asset_project_json_bytes(project: &AssetProject) -> Result<Vec<u8>, AssetProjectError> {
    let mut bytes = serde_json::to_vec_pretty(project)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Return true when `path` uses the `.shapelab-asset.json` filename suffix.
#[must_use]
pub fn has_asset_project_suffix(path: impl AsRef<Path>) -> bool {
    path.as_ref()
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(ASSET_PROJECT_FILE_SUFFIX))
}

/// Ensure a path uses the `.shapelab-asset.json` filename suffix.
pub fn ensure_asset_project_path(path: impl AsRef<Path>) -> Result<(), AssetProjectError> {
    let path = path.as_ref();
    if has_asset_project_suffix(path) {
        Ok(())
    } else {
        Err(AssetProjectError::InvalidProjectPath {
            path: path.to_path_buf(),
        })
    }
}

/// Return a deterministic sibling autosave path for a project file path.
#[must_use]
pub fn autosave_snapshot_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    let parent = sibling_directory(path);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("untitled.shapelab-asset.json");
    parent.join(format!(".{name}.autosave"))
}

fn atomic_asset_project_write(path: &Path, bytes: &[u8]) -> Result<(), AssetProjectError> {
    atomic_asset_project_replace(path, |file| file.write_all(bytes))
}

fn atomic_asset_project_replace(
    path: &Path,
    write_temp: impl FnOnce(&mut File) -> io::Result<()>,
) -> Result<(), AssetProjectError> {
    cleanup_obsolete_asset_temp_files(path);

    let mut temp = TempSibling::create(path)?;

    write_temp(temp.file_mut()).map_err(|source| {
        asset_path_io("writing temporary asset project file for", path, source)
    })?;
    temp.file_mut().sync_all().map_err(|source| {
        asset_path_io("flushing temporary asset project file for", path, source)
    })?;
    temp.persist(path)
        .map_err(|source| asset_path_io("replacing", path, source))?;

    cleanup_obsolete_asset_temp_files(path);
    Ok(())
}

struct TempSibling {
    path: PathBuf,
    file: Option<File>,
    persisted: bool,
}

impl TempSibling {
    fn create(target: &Path) -> Result<Self, AssetProjectError> {
        let parent = sibling_directory(target);
        let prefix = asset_temp_prefix(target);
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
                    return Err(asset_path_io(
                        "creating temporary asset project file for",
                        target,
                        error,
                    ));
                }
            }
        }

        Err(asset_path_io(
            "creating temporary asset project file for",
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

fn cleanup_obsolete_asset_temp_files(path: &Path) {
    let prefix = asset_temp_prefix(path);
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

fn asset_temp_prefix(path: &Path) -> String {
    format!("{ASSET_PROJECT_TEMP_PREFIX}{}-", path_file_fragment(path))
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

fn asset_path_io(action: &'static str, path: &Path, source: io::Error) -> AssetProjectError {
    AssetProjectError::PathIo {
        action,
        path: path.to_path_buf(),
        source,
    }
}

/// Test-only hooks for asset project persistence failure scenarios.
#[doc(hidden)]
pub mod test_support {
    use std::fs::File;
    use std::io;
    use std::path::Path;

    use super::{AssetProjectError, atomic_asset_project_replace};

    /// Run the asset project atomic replacement helper with a custom temp writer.
    pub fn atomic_replace_for_test(
        path: &Path,
        write_temp: impl FnOnce(&mut File) -> io::Result<()>,
    ) -> Result<(), AssetProjectError> {
        atomic_asset_project_replace(path, write_temp)
    }
}
