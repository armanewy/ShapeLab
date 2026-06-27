//! Foundry asset source document contracts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    CatalogContentRef, ControlValue, FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION, FoundryBuildStamp,
    FoundryCatalogLock, FoundryVariationState, LocalRecipeOverride, VariationChannel,
    VariationScope,
};

/// Stable foundry document ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FoundryDocumentId(pub String);

/// Lock target in a foundry document.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FoundryLockTarget {
    /// Control ID.
    Control(String),
    /// Family role.
    Role(String),
    /// Provider selection for a role.
    Provider(String),
    /// Local override ID.
    Override(String),
    /// Export profile.
    ExportProfile(String),
    /// Variation scope.
    VariationScope(VariationScope),
    /// Variation channel.
    VariationChannel(VariationChannel),
    /// Focused semantic part group.
    FocusPartGroup(String),
    /// Material slot.
    MaterialSlot(String),
    /// Pack-authored target.
    Custom(String),
}

/// Lock mode.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundryLockMode {
    /// The target cannot be edited.
    Locked,
    /// The target can be edited but candidate generation must not alter it.
    SearchProtected,
}

/// Foundry lock row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryLock {
    /// Locked target.
    pub target: FoundryLockTarget,
    /// Lock mode.
    pub mode: FoundryLockMode,
    /// Optional reason shown to humans.
    pub reason: Option<String>,
}

/// Explicit provider override for a family role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderOverride {
    /// Family role.
    pub role: String,
    /// Selected provider content reference.
    pub provider_ref: CatalogContentRef,
}

/// Semantic source of truth for one foundry asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryAssetDocument {
    /// Foundry asset document schema version.
    pub schema_version: u32,
    /// Stable document ID.
    pub document_id: FoundryDocumentId,
    /// Family content reference.
    pub family_content_ref: CatalogContentRef,
    /// Style content reference.
    pub style_content_ref: CatalogContentRef,
    /// Family implementation reference.
    pub family_implementation_ref: CatalogContentRef,
    /// Style implementation reference.
    pub style_implementation_ref: CatalogContentRef,
    /// Customizer profile reference.
    pub customizer_profile_ref: CatalogContentRef,
    /// Whole-model control state keyed by control ID.
    pub control_state: BTreeMap<String, ControlValue>,
    /// Provider overrides keyed by family role.
    pub provider_overrides: BTreeMap<String, ProviderOverride>,
    /// Foundry locks.
    pub foundry_locks: Vec<FoundryLock>,
    /// Product variation focus and channels.
    #[serde(default)]
    pub variation_state: FoundryVariationState,
    /// Local recipe overrides applied after base instantiation.
    pub local_recipe_overrides: Vec<LocalRecipeOverride>,
    /// Deterministic seed for candidate and preview workflows.
    pub seed: u64,
    /// Optional exact catalog lock.
    pub catalog_lock: Option<FoundryCatalogLock>,
    /// Optional build stamp from the last completed build.
    pub build_stamp: Option<FoundryBuildStamp>,
}

impl FoundryAssetDocument {
    /// Create an empty document around exact catalog references.
    #[must_use]
    pub fn new(
        document_id: FoundryDocumentId,
        family_content_ref: CatalogContentRef,
        style_content_ref: CatalogContentRef,
        family_implementation_ref: CatalogContentRef,
        style_implementation_ref: CatalogContentRef,
        customizer_profile_ref: CatalogContentRef,
    ) -> Self {
        Self {
            schema_version: FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION,
            document_id,
            family_content_ref,
            style_content_ref,
            family_implementation_ref,
            style_implementation_ref,
            customizer_profile_ref,
            control_state: BTreeMap::new(),
            provider_overrides: BTreeMap::new(),
            foundry_locks: Vec::new(),
            variation_state: FoundryVariationState::default(),
            local_recipe_overrides: Vec::new(),
            seed: 0,
            catalog_lock: None,
            build_stamp: None,
        }
    }
}
