//! Foundry pack contracts and compilation.
#![allow(clippy::result_large_err)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_family::LengthValue;
use shape_family_compile::{
    RecipeFragment,
    identity::{ContentFingerprint, FingerprintError, fingerprint_serializable},
};

use crate::{
    CATALOG_LOCK_KEY_CUSTOMIZER_PROFILE, CATALOG_LOCK_KEY_FAMILY, CATALOG_LOCK_KEY_FAMILY_IMPL,
    CATALOG_LOCK_KEY_STYLE, CATALOG_LOCK_KEY_STYLE_IMPL, CatalogContentRef, ControlValue,
    FOUNDRY_PACK_DOCUMENT_SCHEMA_VERSION, FoundryAssetDocument, FoundryCatalogLock,
    FoundryCatalogResolver, FoundryCompilationError, FoundryCompilationOptions,
    FoundryCompilationOutput, FoundryConformanceSummary, FoundryLock, FoundryLockTarget,
    ProviderOverride, SharedProviderPolicy::SharedExact, compile_foundry_document_for_pack_report,
    document_catalog_refs, validate_foundry_pack,
};

include!("pack/contracts_compile.rs");
include!("pack/report_building.rs");
include!("pack/coherence_checks.rs");
