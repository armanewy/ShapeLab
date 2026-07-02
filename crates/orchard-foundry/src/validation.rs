//! Validation for foundry source and control contracts.

use std::collections::{BTreeMap, BTreeSet};

use orchard_family::ParameterExecutionPolicy;
use serde::{Deserialize, Serialize};

use crate::{
    CATALOG_LOCK_KEY_FAMILY, CATALOG_LOCK_KEY_STYLE, CUSTOMIZER_PROFILE_SCHEMA_VERSION,
    CatalogContentRef, ChoiceOption, ClosedInterval, ControlKind, ControlSlotBinding,
    ControlTopologyBehavior, ControlValue, CustomizerControl, CustomizerProfile,
    DomainCertification, FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION,
    FOUNDRY_PACK_DOCUMENT_SCHEMA_VERSION, FoundryAssetDocument, FoundryCommand, FoundryLockTarget,
    FoundryPackDocument, PackCoherencePolicy, ProviderOption, ResponseCurve, SharedProviderPolicy,
    VariationChannel, VariationIntent, VariationScope, document_catalog_refs,
};

include!("validation/document_profile_command.rs");
include!("validation/control_domains.rs");
include!("validation/references_values_variation.rs");
