//! Foundry pack contracts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    CatalogContentRef, FOUNDRY_PACK_DOCUMENT_SCHEMA_VERSION, FoundryAssetDocument,
    FoundryCatalogLock, FoundryLock,
};

/// Policy for shared providers across a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SharedProviderPolicy {
    /// Members can choose providers independently.
    Independent,
    /// Members must use the pack's shared provider choices.
    SharedExact(BTreeMap<String, CatalogContentRef>),
}

/// Coherence policy for a foundry pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackCoherencePolicy {
    /// All members must share the exact family and style refs.
    ExactFamilyAndStyle,
    /// Members may share only family.
    SharedFamilyOnly,
    /// Pack-authored coherence key.
    Custom(String),
}

/// Export profile for a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryPackExportProfile {
    /// Export profile key.
    pub profile: String,
    /// Whether all members must export successfully.
    pub require_all_members: bool,
}

/// Pack-level semantic source document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryPackDocument {
    /// Foundry pack schema version.
    pub schema_version: u32,
    /// Stable pack ID.
    pub pack_id: String,
    /// Shared family reference.
    pub shared_family_ref: CatalogContentRef,
    /// Shared style reference.
    pub shared_style_ref: CatalogContentRef,
    /// Shared locks.
    pub shared_locks: Vec<FoundryLock>,
    /// Shared provider policy.
    pub shared_provider_policy: SharedProviderPolicy,
    /// Named member documents.
    pub members: BTreeMap<String, FoundryAssetDocument>,
    /// Coherence policy.
    pub coherence_policy: PackCoherencePolicy,
    /// Export profile.
    pub export_profile: FoundryPackExportProfile,
    /// Optional exact catalog lock.
    pub catalog_lock: Option<FoundryCatalogLock>,
}

impl FoundryPackDocument {
    /// Construct an empty exact-family/style pack.
    #[must_use]
    pub fn new(
        pack_id: impl Into<String>,
        shared_family_ref: CatalogContentRef,
        shared_style_ref: CatalogContentRef,
        export_profile: FoundryPackExportProfile,
    ) -> Self {
        Self {
            schema_version: FOUNDRY_PACK_DOCUMENT_SCHEMA_VERSION,
            pack_id: pack_id.into(),
            shared_family_ref,
            shared_style_ref,
            shared_locks: Vec::new(),
            shared_provider_policy: SharedProviderPolicy::Independent,
            members: BTreeMap::new(),
            coherence_policy: PackCoherencePolicy::ExactFamilyAndStyle,
            export_profile,
            catalog_lock: None,
        }
    }
}
