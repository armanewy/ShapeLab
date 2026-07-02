//! SDK-free foundation draft contracts for LLM-assisted Foundry authoring.
//!
//! This module models structured draft kit foundations. It does not call an
//! LLM, generate meshes, inject geometry payloads, mutate recipes directly, or
//! publish content to the novice catalog.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    AttachmentExpectation, CANDIDATE_STRATEGY_PACK_SCHEMA_VERSION, CONTROL_PROFILE_SCHEMA_VERSION,
    CandidateStrategyPack, CatalogVisibilityPolicy, ControlOptionVisibility, ControlProfile,
    ControlProfileControl, ControlProfileControlKind, ControlProfileTopologyBehavior,
    DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS, ExportPartNamingPolicy, FAMILY_BLUEPRINT_SCHEMA_VERSION,
    FOUNDRY_KIT_PACKAGE_SCHEMA_VERSION, FOUNDRY_KIT_SCHEMA_VERSION, FamilyBlueprint,
    FamilyBlueprintRole, FoundryKit, FoundryKitPackage, FoundryKitQualityTier,
    FutureMaterialVocabulary, HighLevelScalePolicy, KIT_CATALOG_MANIFEST_SCHEMA_VERSION,
    KIT_COMPATIBILITY_MATRIX_SCHEMA_VERSION, KIT_REVIEW_MANIFEST_SCHEMA_VERSION,
    KitCandidateStrategy, KitCatalogManifest, KitCompatibilityMatrix, KitReviewManifest,
    PROVIDER_PACK_SCHEMA_VERSION, PreviewCameraPolicy, ProviderPack, ProviderPackOption,
    ProviderSlotExpectation, QUALITY_GATE_PROFILE_SCHEMA_VERSION, QualityGateProfile,
    STYLE_PACK_SCHEMA_VERSION, StylePack, StyleProviderCompatibility, StyleProviderIncompatibility,
    validate_foundry_kit_package,
};

include!("foundation/draft_contracts.rs");
include!("foundation/commands_and_templates.rs");
include!("foundation/archetype_box_fixture.rs");
include!("foundation/validation.rs");
include!("foundation/materialization.rs");
include!("foundation/adversarial_repairs.rs");
include!("foundation/tests.rs");
