//! Whole-model customizer control contracts.

use std::collections::{BTreeMap, BTreeSet};

use orchard_family::{
    FamilyDefaultValue, FamilyParameterKind, FamilyParameterSlot, ParameterExecutionPolicy,
};
use orchard_family_compile::FamilyValue;
use serde::{Deserialize, Serialize};

use crate::CUSTOMIZER_PROFILE_SCHEMA_VERSION;
use crate::document::FoundryAssetDocument;
use crate::edit::TouchedSemanticTarget;

include!("control/contracts.rs");
include!("control/evaluation.rs");
include!("control/canonicalization.rs");
include!("control/domains.rs");
include!("control/lookup_and_describe.rs");
