//! Whole-model customizer control contracts.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_family::{
    FamilyDefaultValue, FamilyParameterKind, FamilyParameterSlot, ParameterExecutionPolicy,
};
use shape_family_compile::FamilyValue;

use crate::CUSTOMIZER_PROFILE_SCHEMA_VERSION;
use crate::document::FoundryAssetDocument;
use crate::edit::TouchedSemanticTarget;

include!("control/contracts.rs");
include!("control/evaluation.rs");
include!("control/canonicalization.rs");
include!("control/domains.rs");
include!("control/lookup_and_describe.rs");
