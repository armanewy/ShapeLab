//! Catalog reference and locking contracts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shape_family_compile::identity::CatalogContentFingerprint;

/// Stable reference to one catalog content document.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CatalogContentRef {
    /// Stable content ID in the catalog namespace.
    pub stable_id: String,
    /// Content schema version.
    pub schema_version: u32,
    /// Exact 256-bit content fingerprint.
    pub fingerprint: CatalogContentFingerprint,
}

/// Embedded snapshot fallback for reproducible builds when a catalog is absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddedCatalogSnapshot {
    /// Reference this snapshot satisfies.
    pub content_ref: CatalogContentRef,
    /// UTF-8 JSON payload captured at lock time.
    pub canonical_json: String,
}

/// Exact catalog lock used by foundry documents, packs, and projects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryCatalogLock {
    /// Required exact references keyed by semantic role, such as `family` or `style_impl`.
    pub exact_refs: BTreeMap<String, CatalogContentRef>,
    /// Optional embedded snapshots for read-only recovery.
    #[serde(default)]
    pub embedded_snapshots: Vec<EmbeddedCatalogSnapshot>,
    /// Compiler crate version used to create the lock.
    pub compiler_version: String,
    /// Catalog format version.
    pub catalog_version: u32,
}
