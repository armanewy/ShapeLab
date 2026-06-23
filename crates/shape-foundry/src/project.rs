//! Replayable foundry project contracts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shape_asset::RevisionId;

use crate::{
    FOUNDRY_PROJECT_DOCUMENT_SCHEMA_VERSION, FoundryAssetDocument, FoundryCatalogLock,
    FoundryConformanceSummary, FoundryEdit, GeneratedRecipeSnapshot,
};

/// Stored foundry project revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryProjectRevision {
    /// Stable revision ID.
    pub id: RevisionId,
    /// Parent revision.
    pub parent: Option<RevisionId>,
    /// Human-facing label.
    pub label: String,
    /// Semantic source snapshot.
    pub document: FoundryAssetDocument,
    /// Edit or command program that produced this revision.
    pub edit: Option<FoundryEdit>,
    /// Catalog lock.
    pub catalog_lock: FoundryCatalogLock,
    /// Exact generated recipe snapshot.
    pub recipe_snapshot: Option<GeneratedRecipeSnapshot>,
    /// Conformance summary.
    pub conformance: FoundryConformanceSummary,
}

/// Replayable foundry project file contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryProjectDocument {
    /// Foundry project schema version.
    pub schema_version: u32,
    /// Project title.
    pub title: String,
    /// Current revision.
    pub current_revision: RevisionId,
    /// Next revision ID.
    pub next_revision_id: u64,
    /// Revision graph.
    pub revisions: BTreeMap<RevisionId, FoundryProjectRevision>,
}

impl FoundryProjectDocument {
    /// Create an empty project contract.
    #[must_use]
    pub fn empty(title: impl Into<String>) -> Self {
        Self {
            schema_version: FOUNDRY_PROJECT_DOCUMENT_SCHEMA_VERSION,
            title: title.into(),
            current_revision: RevisionId(0),
            next_revision_id: 1,
            revisions: BTreeMap::new(),
        }
    }
}
